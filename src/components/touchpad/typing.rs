// Ayuz - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use tokio::sync::watch;

use crate::services::config::AppConfig;
use crate::services::touchpad_ctl;
use crate::services::typing_watch::{self, TypingConfig};

/// Shortest and longest reactivation delay offered in the UI, in seconds.
const DELAY_MIN_S: f64 = 0.1;
const DELAY_MAX_S: f64 = 5.0;

/// State for the "disable while typing" (palm rejection) component.
pub struct TypingModel {
    /// Suppress touchpad edge gestures while typing.
    disable_gestures: bool,
    /// Disable the whole touchpad while typing.
    disable_touchpad: bool,
    /// Idle time after the last keystroke before everything is restored.
    delay_ms: u32,
    /// Whether the desktop environment supports touchpad toggling (KDE or GNOME).
    desktop_supported: bool,
    /// Sender that keeps the watcher task alive; dropping it stops the task.
    watcher_tx: Option<watch::Sender<TypingConfig>>,
}

#[derive(Debug)]
pub enum TypingMsg {
    ToggleDisableGestures(bool),
    ToggleDisableTouchpad(bool),
    /// New reactivation delay, in seconds, as reported by the spin button.
    DelayChanged(f64),
    LoadProfile {
        disable_gestures: bool,
        disable_touchpad: bool,
        delay_ms: u32,
    },
}

#[relm4::component(pub)]
impl SimpleComponent for TypingModel {
    type Init = ();
    type Input = TypingMsg;
    type Output = String;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("typing_group_title"),
            set_description: Some(&t!("typing_group_desc")),

            #[template]
            add = &crate::components::widgets::DaemonWarningLabel {
                set_visible: !model.desktop_supported,
                set_label: &t!("typing_desktop_required"),
            },

            add = &adw::SwitchRow {
                set_title: &t!("typing_disable_gestures_title"),
                set_subtitle: &t!("typing_disable_gestures_subtitle"),

                #[watch]
                set_active: model.disable_gestures,

                connect_active_notify[sender] => move |s| {
                    sender.input(TypingMsg::ToggleDisableGestures(s.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("typing_disable_touchpad_title"),
                set_subtitle: &t!("typing_disable_touchpad_subtitle"),

                set_sensitive: model.desktop_supported,
                #[watch]
                set_active: model.disable_touchpad,

                connect_active_notify[sender] => move |s| {
                    sender.input(TypingMsg::ToggleDisableTouchpad(s.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("typing_delay_title"),
                set_subtitle: &t!("typing_delay_subtitle"),

                #[watch]
                set_sensitive: model.disable_gestures || model.disable_touchpad,

                add_suffix = &gtk::SpinButton::with_range(DELAY_MIN_S, DELAY_MAX_S, 0.1) {
                    set_valign: gtk::Align::Center,
                    set_digits: 1,

                    #[watch]
                    set_value: model.delay_ms as f64 / 1000.0,

                    connect_value_changed[sender] => move |spin| {
                        sender.input(TypingMsg::DelayChanged(spin.value()));
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let profile = AppConfig::load().active_profile().clone();
        let mut model = TypingModel {
            disable_gestures: profile.typing_disable_gestures,
            disable_touchpad: profile.typing_disable_touchpad,
            delay_ms: profile.typing_reactivation_delay_ms,
            desktop_supported: touchpad_ctl::desktop_supported(),
            watcher_tx: None,
        };
        let widgets = view_output!();
        model.sync_watcher();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: TypingMsg, _sender: ComponentSender<Self>) {
        match msg {
            TypingMsg::ToggleDisableGestures(active) => {
                if active == self.disable_gestures {
                    return;
                }
                self.disable_gestures = active;
                AppConfig::update(|c| c.active_profile_mut().typing_disable_gestures = active);
                self.sync_watcher();
            }
            TypingMsg::ToggleDisableTouchpad(active) => {
                if active == self.disable_touchpad {
                    return;
                }
                self.disable_touchpad = active;
                AppConfig::update(|c| c.active_profile_mut().typing_disable_touchpad = active);
                self.sync_watcher();
            }
            TypingMsg::DelayChanged(seconds) => {
                let delay_ms = (seconds * 1000.0).round() as u32;
                if delay_ms == self.delay_ms {
                    return;
                }
                self.delay_ms = delay_ms;
                AppConfig::update(|c| {
                    c.active_profile_mut().typing_reactivation_delay_ms = delay_ms
                });
                self.sync_watcher();
            }
            TypingMsg::LoadProfile {
                disable_gestures,
                disable_touchpad,
                delay_ms,
            } => {
                self.disable_gestures = disable_gestures;
                self.disable_touchpad = disable_touchpad;
                self.delay_ms = delay_ms;
                self.sync_watcher();
            }
        }
    }
}

impl TypingModel {
    /// Current settings as seen by the watcher task.
    fn watcher_config(&self) -> TypingConfig {
        TypingConfig {
            disable_gestures: self.disable_gestures,
            disable_touchpad: self.disable_touchpad && self.desktop_supported,
            reactivation_delay_ms: self.delay_ms,
        }
    }

    /// Starts, updates or stops the keyboard watcher so it matches the model.
    ///
    /// The task lives exactly as long as [`Self::watcher_tx`]: dropping the
    /// sender makes it revert whatever it changed and exit.
    fn sync_watcher(&mut self) {
        let config = self.watcher_config();
        if !config.any_enabled() {
            self.watcher_tx = None;
            return;
        }
        match &self.watcher_tx {
            Some(tx) => {
                let _ = tx.send(config);
            }
            None => {
                let (tx, rx) = watch::channel(config);
                tokio::spawn(typing_watch::run(rx));
                self.watcher_tx = Some(tx);
            }
        }
    }
}
