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

pub struct VolumeModel {
    volume: f64,
}

#[derive(Debug)]
pub enum VolumeMsg {
    SetVolume(f64),
    UpdateUi(f64),
}

// SimpleComponent is intentional here: volume control needs no async CommandOutput or
// error forwarding to the parent - it handles all async work via tokio::spawn internally.
#[relm4::component(pub)]
impl SimpleComponent for VolumeModel {
    type Init = ();
    type Input = VolumeMsg;
    type Output = String;

    view! {
        adw::PreferencesGroup {
            set_title: &gtk::glib::markup_escape_text(&t!("volume_booster_title")),
            set_description: Some(&t!("volume_booster_desc")),

            add = &adw::ActionRow {
                set_title: &t!("volume_level_label"),

                add_suffix = &gtk::Scale {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_range: (0.0, 150.0),
                    #[watch]
                    set_value: model.volume,
                    set_width_request: 200,
                    connect_value_changed[sender] => move |scale| {
                        sender.input(VolumeMsg::SetVolume(scale.value()));
                    },
                },

                add_suffix = &gtk::Label {
                    #[watch]
                    set_label: &format!("{}%", model.volume as i32),
                    set_width_chars: 5,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            let output = if is_flatpak() {
                tokio::process::Command::new("flatpak-spawn")
                    .args(["--host", "wpctl", "get-volume", "@DEFAULT_AUDIO_SINK@"])
                    .output()
                    .await
            } else {
                tokio::process::Command::new("wpctl")
                    .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
                    .output()
                    .await
            };
            if let Ok(out) = output {
                let text = String::from_utf8_lossy(&out.stdout);
                // Format: "Volume: 0.45"
                if let Some(vol_str) = text.split_whitespace().nth(1) {
                    if let Ok(val) = vol_str.parse::<f64>() {
                        sender_clone.input(VolumeMsg::UpdateUi(val * 100.0));
                        return;
                    }
                }
            }
            sender_clone.input(VolumeMsg::UpdateUi(100.0));
        });

        let model = VolumeModel { volume: 100.0 };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: VolumeMsg, _sender: ComponentSender<Self>) {
        match msg {
            VolumeMsg::UpdateUi(vol) => {
                self.volume = vol;
            }
            VolumeMsg::SetVolume(vol) => {
                self.volume = vol;
                let vol_str = format!("{}%", vol as i32);
                if is_flatpak() {
                    let _ = tokio::process::Command::new("flatpak-spawn")
                        .args(["--host", "wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
                        .spawn();
                } else {
                    let _ = tokio::process::Command::new("wpctl")
                        .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &vol_str])
                        .spawn();
                }
            }
        }
    }
}
