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

use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::touchpad_ctl;

/// State for the touchpad enable/disable component.
pub struct TouchpadModel {
    /// Whether the touchpad is currently enabled.
    touchpad_active: bool,
    /// Remaining seconds on the disable-confirmation countdown (starts at 10).
    countdown: u8,
    /// When `true`, the confirmation row is shown and the auto-revert timer is running.
    confirmation_required: bool,
    /// Handle for the 10-second countdown task; abort it to cancel the revert.
    timer_handle: Option<tokio::task::JoinHandle<()>>,
    /// Whether the current desktop environment supports touchpad toggling (KDE or GNOME).
    desktop_supported: bool,
}

/// Input messages for the touchpad component.
#[derive(Debug)]
pub enum TouchpadMsg {
    /// User toggled the enable/disable switch.
    ToggleTouchpad(bool),
    /// User pressed the confirm button within the 10-second window after disabling.
    ConfirmClicked,
    /// Apply touchpad state from a profile without saving.
    LoadProfile(bool),
}

/// Async command results for the touchpad component.
#[derive(Debug)]
pub enum TouchpadCommandOutput {
    /// An error message to forward as a toast notification.
    Error(String),
    /// Fired every second to decrement the on-screen countdown.
    CountdownTick,
    /// Fired when the 10-second confirmation window expires without user action,
    /// triggering an automatic re-enable of the touchpad.
    TimerElapsed,
}

#[relm4::component(pub)]
impl Component for TouchpadModel {
    type Init = ();
    type Input = TouchpadMsg;
    type Output = String;
    type CommandOutput = TouchpadCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("touchpad_group_title"),

            #[template]
            add = &crate::components::widgets::DaemonWarningLabel {
                #[watch]
                set_visible: !model.desktop_supported,
                set_label: &t!("touchpad_desktop_required"),
            },

            add = &gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",

                append = &adw::SwitchRow {
                    set_title: &t!("touchpad_enable_title"),
                    set_subtitle: &t!("touchpad_enable_subtitle"),

                    #[watch]
                    set_active: model.touchpad_active,
                    #[watch]
                    set_sensitive: model.desktop_supported,

                    connect_active_notify[sender] => move |s| {
                        sender.input(TouchpadMsg::ToggleTouchpad(s.is_active()));
                    },
                },

                append = &adw::ActionRow {
                    #[watch]
                    set_visible: model.confirmation_required,

                    #[watch]
                    set_title: &t!("touchpad_countdown_title", seconds = model.countdown.to_string()),

                    add_suffix = &gtk::Button {
                        set_label: &t!("touchpad_confirm_label"),
                        add_css_class: "suggested-action",
                        set_valign: gtk::Align::Center,

                        connect_clicked[sender] => move |_| {
                            sender.input(TouchpadMsg::ConfirmClicked);
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let touchpad_active = AppConfig::load().active_profile().touchpad_active;
        let model = TouchpadModel {
            touchpad_active,
            countdown: 10,
            confirmation_required: false,
            timer_handle: None,
            desktop_supported: touchpad_ctl::desktop_supported(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: TouchpadMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            TouchpadMsg::ToggleTouchpad(active) => {
                if active == self.touchpad_active {
                    return;
                }
                self.touchpad_active = active;

                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }

                if !active {
                    self.confirmation_required = true;
                    self.countdown = 10;

                    let cmd_sender = sender.command_sender().clone();
                    self.timer_handle = Some(tokio::spawn(async move {
                        for _ in 0..10 {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            cmd_sender.emit(TouchpadCommandOutput::CountdownTick);
                        }
                        cmd_sender.emit(TouchpadCommandOutput::TimerElapsed);
                    }));
                } else {
                    self.confirmation_required = false;
                }

                AppConfig::update(|c| c.active_profile_mut().touchpad_active = active);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = touchpad_ctl::set_enabled(active).await {
                                out.emit(TouchpadCommandOutput::Error(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadMsg::LoadProfile(active) => {
                if !self.desktop_supported {
                    return;
                }
                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }
                self.touchpad_active = active;
                self.confirmation_required = false;
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = touchpad_ctl::set_enabled(active).await {
                                out.emit(TouchpadCommandOutput::Error(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadMsg::ConfirmClicked => {
                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }
                self.confirmation_required = false;
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: TouchpadCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            TouchpadCommandOutput::CountdownTick => {
                self.countdown = self.countdown.saturating_sub(1);
            }
            TouchpadCommandOutput::TimerElapsed => {
                if !self.confirmation_required {
                    return;
                }
                self.touchpad_active = true;
                self.confirmation_required = false;
                self.timer_handle = None;

                AppConfig::update(|c| c.active_profile_mut().touchpad_active = true);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = touchpad_ctl::set_enabled(true).await {
                                out.emit(TouchpadCommandOutput::Error(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}
