// Asus Hub - Unofficial Control Center for Asus Laptops
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

use crate::services::commands::is_flatpak;
use crate::services::config::AppConfig;

/// Controls when the keyboard backlight idle-timeout is active.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum TimeoutMode {
    /// Backlight never times out automatically.
    #[default]
    Never,
    /// Timeout applies both on battery and when plugged in to AC power.
    BatteryAndAc,
    /// Timeout applies only when running on battery power.
    BatteryOnly,
}

impl From<u32> for TimeoutMode {
    fn from(v: u32) -> Self {
        match v {
            1 => Self::BatteryAndAc,
            2 => Self::BatteryOnly,
            _ => Self::Never,
        }
    }
}

const TIMEOUT_SECONDS: [u32; 3] = [60, 120, 300];

/// Builds a `busctl` command string to set keyboard backlight brightness via UPower.
///
/// When `battery_only` is `true`, the command is wrapped in a shell conditional that reads
/// the first `online` sysfs file to skip execution when the device is plugged in to AC power.
fn busctl_brightness_cmd(value: i32, battery_only: bool) -> String {
    let base = format!(
        "busctl call --system org.freedesktop.UPower \
         /org/freedesktop/UPower/KbdBacklight \
         org.freedesktop.UPower.KbdBacklight SetBrightness i {value}"
    );
    if battery_only {
        format!(
            "if [ \"$(cat /sys/class/power_supply/*/online | head -n1)\" = \"0\" ]; \
             then {base}; fi"
        )
    } else {
        base
    }
}

pub struct BacklightIdleModel {
    timeout_mode: TimeoutMode,
    check_never: gtk::CheckButton,
    check_battery_and_ac: gtk::CheckButton,
    check_battery_only: gtk::CheckButton,
    dropdown_battery_and_ac: gtk::DropDown,
    dropdown_battery_only: gtk::DropDown,
    swayidle_task: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug)]
pub enum BacklightIdleMsg {
    ChangeMode(TimeoutMode),
    BatteryAndAcTimeChanged(u32),
    BatteryOnlyTimeChanged(u32),
}

#[derive(Debug)]
pub enum BacklightIdleCommandOutput {
    Error(String),
}

#[relm4::component(pub)]
impl Component for BacklightIdleModel {
    type Init = ();
    type Input = BacklightIdleMsg;
    type Output = String;
    type CommandOutput = BacklightIdleCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("sleep_group_title"),
            set_description: Some(&t!("sleep_group_desc")),

            add = &adw::ActionRow {
                set_title: &t!("sleep_mode_never_title"),
                set_subtitle: &t!("sleep_mode_never_subtitle"),
                add_prefix = &model.check_never.clone(),
                set_activatable_widget: Some(&model.check_never),
            },

            add = &adw::ActionRow {
                set_title: &t!("sleep_mode_always_title"),
                add_prefix = &model.check_battery_and_ac.clone(),
                set_activatable_widget: Some(&model.check_battery_and_ac),
                add_suffix = &model.dropdown_battery_and_ac.clone() -> gtk::DropDown {
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_sensitive: model.timeout_mode == TimeoutMode::BatteryAndAc,
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("sleep_mode_battery_title"),
                add_prefix = &model.check_battery_only.clone(),
                set_activatable_widget: Some(&model.check_battery_only),
                add_suffix = &model.dropdown_battery_only.clone() -> gtk::DropDown {
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_sensitive: model.timeout_mode == TimeoutMode::BatteryOnly,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();
        let mode = TimeoutMode::from(config.kbd_timeout_mode);

        let check_never = gtk::CheckButton::new();
        let check_battery_and_ac = gtk::CheckButton::new();
        let check_battery_only = gtk::CheckButton::new();
        check_battery_and_ac.set_group(Some(&check_never));
        check_battery_only.set_group(Some(&check_never));

        match mode {
            TimeoutMode::Never => check_never.set_active(true),
            TimeoutMode::BatteryAndAc => check_battery_and_ac.set_active(true),
            TimeoutMode::BatteryOnly => check_battery_only.set_active(true),
        }

        let t1 = t!("sleep_timeout_1min");
        let t2 = t!("sleep_timeout_2min");
        let t5 = t!("sleep_timeout_5min");
        let time_options = gtk::StringList::new(&[&t1, &t2, &t5]);
        let dropdown_battery_and_ac =
            gtk::DropDown::new(Some(time_options.clone()), gtk::Expression::NONE);
        let dropdown_battery_only = gtk::DropDown::new(Some(time_options), gtk::Expression::NONE);
        dropdown_battery_and_ac.set_selected(config.kbd_timeout_battery_ac_index);
        dropdown_battery_only.set_selected(config.kbd_timeout_battery_only_index);

        for (btn, mode_val) in [
            (&check_never, TimeoutMode::Never),
            (&check_battery_and_ac, TimeoutMode::BatteryAndAc),
            (&check_battery_only, TimeoutMode::BatteryOnly),
        ] {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(BacklightIdleMsg::ChangeMode(mode_val));
                }
            });
        }

        {
            let sender = sender.clone();
            dropdown_battery_and_ac.connect_selected_notify(move |dd| {
                sender.input(BacklightIdleMsg::BatteryAndAcTimeChanged(dd.selected()));
            });
        }
        {
            let sender = sender.clone();
            dropdown_battery_only.connect_selected_notify(move |dd| {
                sender.input(BacklightIdleMsg::BatteryOnlyTimeChanged(dd.selected()));
            });
        }

        let mut model = BacklightIdleModel {
            timeout_mode: mode,
            check_never,
            check_battery_and_ac,
            check_battery_only,
            dropdown_battery_and_ac,
            dropdown_battery_only,
            swayidle_task: None,
        };

        let widgets = view_output!();
        model.apply_timeout(mode, &sender);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: BacklightIdleMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            BacklightIdleMsg::ChangeMode(mode) => {
                self.timeout_mode = mode;
                AppConfig::update(|c| c.kbd_timeout_mode = mode as u32);
                self.apply_timeout(mode, &sender);
            }
            BacklightIdleMsg::BatteryAndAcTimeChanged(index) => {
                AppConfig::update(|c| c.kbd_timeout_battery_ac_index = index);
                if self.timeout_mode == TimeoutMode::BatteryAndAc {
                    self.apply_timeout(TimeoutMode::BatteryAndAc, &sender);
                }
            }
            BacklightIdleMsg::BatteryOnlyTimeChanged(index) => {
                AppConfig::update(|c| c.kbd_timeout_battery_only_index = index);
                if self.timeout_mode == TimeoutMode::BatteryOnly {
                    self.apply_timeout(TimeoutMode::BatteryOnly, &sender);
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: BacklightIdleCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            BacklightIdleCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

impl BacklightIdleModel {
    /// (Re)starts the `swayidle` daemon with the current timeout settings.
    ///
    /// Aborts any previously running `swayidle` task before spawning a new one.
    /// Does nothing (and kills the old task) when `mode` is [`TimeoutMode::Never`].
    fn apply_timeout(&mut self, mode: TimeoutMode, sender: &ComponentSender<BacklightIdleModel>) {
        if let Some(task) = self.swayidle_task.take() {
            task.abort();
        }

        if mode == TimeoutMode::Never {
            return;
        }

        let seconds = match mode {
            TimeoutMode::Never => unreachable!("TimeoutMode::Never is handled by the early return above"),
            TimeoutMode::BatteryAndAc => {
                let idx = self.dropdown_battery_and_ac.selected() as usize;
                *TIMEOUT_SECONDS.get(idx).unwrap_or(&60)
            }
            TimeoutMode::BatteryOnly => {
                let idx = self.dropdown_battery_only.selected() as usize;
                *TIMEOUT_SECONDS.get(idx).unwrap_or(&60)
            }
        };

        let battery_only = mode == TimeoutMode::BatteryOnly;
        let timeout_cmd = busctl_brightness_cmd(0, battery_only);
        let resume_cmd = busctl_brightness_cmd(3, battery_only);
        let seconds_str = seconds.to_string();

        let (swayidle_program, swayidle_args): (String, Vec<String>) = if is_flatpak() {
            (
                "flatpak-spawn".to_string(),
                vec![
                    "--host".to_string(),
                    "swayidle".to_string(),
                    "-w".to_string(),
                    "timeout".to_string(),
                    seconds_str,
                    timeout_cmd,
                    "resume".to_string(),
                    resume_cmd,
                ],
            )
        } else {
            (
                "swayidle".to_string(),
                vec![
                    "-w".to_string(),
                    "timeout".to_string(),
                    seconds_str,
                    timeout_cmd,
                    "resume".to_string(),
                    resume_cmd,
                ],
            )
        };

        let cmd_sender = sender.command_sender().clone();
        let handle = tokio::spawn(async move {
            let mut child = match tokio::process::Command::new(&swayidle_program)
                .kill_on_drop(true)
                .args(&swayidle_args)
                .spawn()
            {
                Ok(c) => c,
                Err(e) => {
                    cmd_sender.emit(BacklightIdleCommandOutput::Error(
                        t!("error_swayidle_start", error = e.to_string()).to_string(),
                    ));
                    return;
                }
            };
            if let Err(e) = child.wait().await {
                cmd_sender.emit(BacklightIdleCommandOutput::Error(
                    t!("error_swayidle_wait", error = e.to_string()).to_string(),
                ));
            }
        });

        self.swayidle_task = Some(handle);
    }
}
