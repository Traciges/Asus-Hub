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

pub struct VolumeModel {
    volume: f64,
}

#[derive(Debug)]
pub enum VolumeMsg {
    SetVolume(f64),
    UpdateUi(f64),
    LoadProfile(f64),
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
            let vol = read_current_volume().await.unwrap_or(100.0);
            sender_clone.input(VolumeMsg::UpdateUi(vol));
        });

        tokio::spawn(start_volume_listener(sender.clone()));

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
                crate::services::config::AppConfig::update(|c| {
                    c.active_profile_mut().volume = vol;
                });
                let _ = tokio::process::Command::new("wpctl")
                    .args([
                        "set-volume",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{}%", vol as i32),
                    ])
                    .spawn();
            }
            VolumeMsg::LoadProfile(vol) => {
                self.volume = vol;
                let _ = tokio::process::Command::new("wpctl")
                    .args([
                        "set-volume",
                        "@DEFAULT_AUDIO_SINK@",
                        &format!("{}%", vol as i32),
                    ])
                    .spawn();
            }
        }
    }
}

async fn read_current_volume() -> Option<f64> {
    let out = tokio::process::Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .output()
        .await
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // Format: "Volume: 0.45"
    let vol_str = text.split_whitespace().nth(1)?;
    let val = vol_str.parse::<f64>().ok()?;
    Some(val * 100.0)
}

async fn start_volume_listener(sender: relm4::ComponentSender<VolumeModel>) {
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let mut child = match Command::new("pactl")
        .arg("subscribe")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.contains("'change'") && line.contains("sink") {
            if let Some(vol) = read_current_volume().await {
                sender.input(VolumeMsg::UpdateUi(vol));
            }
        }
    }
}
