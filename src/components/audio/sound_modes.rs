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
use gtk4::gio;
use gtk4::glib;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use std::path::PathBuf;

use crate::services::commands::{is_flatpak, run_command_blocking};
use crate::services::config::AppConfig;

const PRESET_MUSIC: &str = include_str!("../../../assets/presets/Music.json");
const PRESET_MOVIE: &str = include_str!("../../../assets/presets/Movie.json");
const PRESET_VIDEO: &str = include_str!("../../../assets/presets/Video.json");
const PRESET_VOICE: &str = include_str!("../../../assets/presets/Voice.json");
const PRESET_CUSTOM_EQ: &str = include_str!("../../../assets/presets/Perfect_EQ.json");

const PRESETS: &[(&str, &str)] = &[
    ("Movie", PRESET_MOVIE),
    ("Music", PRESET_MUSIC),
    ("Perfect_EQ", PRESET_CUSTOM_EQ),
    ("Video", PRESET_VIDEO),
    ("Voice", PRESET_VOICE),
];

// Index 0..6: Movie, Music, None(bypass), Perfect_EQ, Video, Voice, Custom
// Index 2 = None (no preset, bypass only)
const NONE_IDX: u32 = 2;
const CUSTOM_IDX: u32 = 6;
const PRESET_NAMES: &[&str] = &["Movie", "Music", "Perfect_EQ", "Video", "Voice"];
const EASYEFFECTS_STARTUP_DELAY_MS: u64 = 1500;

pub struct SoundModesModel {
    ee_installed: bool,
    current_profile: u32,
    previous_profile: u32,
    dropdown: gtk::DropDown,
}

#[derive(Debug)]
pub enum AudioMsg {
    ChangeProfile(u32),
    CustomPresetPathSelected(PathBuf),
    CustomCancelled(u32),
}

#[derive(Debug)]
pub enum AudioCommandOutput {
    EeChecked(bool),
    PresetsInstalled,
    ProfileSet(u32),
    CustomPresetLoaded(String),
    Error(String),
}

#[relm4::component(pub)]
impl Component for SoundModesModel {
    type Init = ();
    type Input = AudioMsg;
    type Output = String;
    type CommandOutput = AudioCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("audio_profiles_title"),
            set_description: Some(&t!("audio_profiles_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.ee_installed,
                set_label: &t!("ee_missing_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::ActionRow {
                set_title: &t!("audio_profile_label"),
                add_suffix = &model.dropdown.clone(),
                set_activatable_widget: Some(&model.dropdown),
                #[watch]
                set_sensitive: model.ee_installed,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();

        let options = gtk::StringList::new(&[
            &t!("audio_profile_film"),
            &t!("audio_profile_music"),
            &t!("audio_profile_none"),
            &t!("audio_profile_optimized"),
            &t!("audio_profile_video"),
            &t!("audio_profile_voice"),
            &t!("audio_profile_custom"),
        ]);
        let dropdown = gtk::DropDown::new(Some(options), gtk::Expression::NONE);
        dropdown.set_valign(gtk::Align::Center);
        dropdown.set_selected(config.audio_profile);

        {
            let sender = sender.clone();
            dropdown.connect_selected_notify(move |dd| {
                sender.input(AudioMsg::ChangeProfile(dd.selected()));
            });
        }

        let model = SoundModesModel {
            ee_installed: false,
            current_profile: config.audio_profile,
            previous_profile: config.audio_profile,
            dropdown,
        };

        let widgets = view_output!();

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let installed = run_command_blocking("which", &["easyeffects"]).await.is_ok();
                    out.emit(AudioCommandOutput::EeChecked(installed));
                })
                .drop_on_shutdown()
        });

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    match install_presets().await {
                        Ok(()) => out.emit(AudioCommandOutput::PresetsInstalled),
                        Err(e) => out.emit(AudioCommandOutput::Error(
                            t!("audio_preset_install_error", error = e).to_string(),
                        )),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: AudioMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            AudioMsg::ChangeProfile(idx) => {
                if idx == self.current_profile {
                    return;
                }

                if idx == CUSTOM_IDX {
                    let previous = self.current_profile;
                    self.current_profile = CUSTOM_IDX;

                    let sender_clone = sender.clone();
                    let dialog = gtk::FileDialog::builder()
                        .title(t!("audio_profile_custom").as_ref())
                        .accept_label("Open")
                        .build();
                    let filter = gtk::FileFilter::new();
                    filter.add_pattern("*.json");
                    filter.set_name(Some("JSON"));
                    let store = gio::ListStore::new::<gtk::FileFilter>();
                    store.append(&filter);
                    dialog.set_filters(Some(&store));

                    glib::spawn_future_local(async move {
                        match dialog.open_future(None::<&gtk::Window>).await {
                            Ok(file) => {
                                if let Some(path) = file.path() {
                                    sender_clone.input(AudioMsg::CustomPresetPathSelected(path));
                                } else {
                                    sender_clone.input(AudioMsg::CustomCancelled(previous));
                                }
                            }
                            Err(_) => {
                                sender_clone.input(AudioMsg::CustomCancelled(previous));
                            }
                        }
                    });
                    return;
                }

                self.previous_profile = self.current_profile;
                self.current_profile = idx;
                AppConfig::update(|c| c.audio_profile = idx);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = set_easyeffects_profile(idx, None).await {
                                out.emit(AudioCommandOutput::Error(e));
                                return;
                            }
                            out.emit(AudioCommandOutput::ProfileSet(idx));
                        })
                        .drop_on_shutdown()
                });
            }

            AudioMsg::CustomPresetPathSelected(path) => {
                if extract_file_stem(&path).is_err() {
                    sender.input(AudioMsg::CustomCancelled(self.previous_profile));
                    return;
                }

                AppConfig::update(|c| {
                    c.audio_profile = CUSTOM_IDX;
                });

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match load_custom_preset(path).await {
                                Ok(n) => out.emit(AudioCommandOutput::CustomPresetLoaded(n)),
                                Err(e) => out.emit(AudioCommandOutput::Error(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }

            AudioMsg::CustomCancelled(previous) => {
                self.current_profile = previous;
                self.dropdown.set_selected(previous);
                AppConfig::update(|c| c.audio_profile = previous);
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: AudioCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AudioCommandOutput::EeChecked(installed) => {
                self.ee_installed = installed;
            }
            AudioCommandOutput::PresetsInstalled => {}
            AudioCommandOutput::ProfileSet(idx) => {
                tracing::info!("{}", t!("audio_profile_set", profile = idx));
            }
            AudioCommandOutput::CustomPresetLoaded(name) => {
                tracing::info!("{}", t!("audio_profile_set", profile = name));
            }
            AudioCommandOutput::Error(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn ensure_easyeffects_running() {
    let daemon_running = run_command_blocking("pgrep", &["-x", "easyeffects"]).await.is_ok();

    if !daemon_running {
        if is_flatpak() {
            let _ = tokio::process::Command::new("flatpak-spawn")
                .args(["--host", "easyeffects", "--gapplication-service"])
                .spawn();
        } else {
            let _ = tokio::process::Command::new("easyeffects")
                .arg("--gapplication-service")
                .spawn();
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(
            EASYEFFECTS_STARTUP_DELAY_MS,
        ))
        .await;
    }
}

fn easyeffects_output_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    Ok(PathBuf::from(home).join(".config/easyeffects/output"))
}

fn extract_file_stem(path: &std::path::Path) -> Result<String, String> {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .ok_or_else(|| "Invalid file name".to_string())
}

async fn set_easyeffects_profile(idx: u32, custom_name: Option<String>) -> Result<(), String> {
    ensure_easyeffects_running().await;

    if idx == NONE_IDX {
        run_command_blocking("easyeffects", &["-b", "1"]).await?;
    } else if idx == CUSTOM_IDX {
        if let Some(name) = custom_name {
            run_command_blocking("easyeffects", &["-b", "2"]).await?;
            run_command_blocking("easyeffects", &["-l", &name]).await?;
        }
    } else {
        run_command_blocking("easyeffects", &["-b", "2"]).await?;
        let preset_idx = if idx < NONE_IDX { idx } else { idx - 1 } as usize;
        run_command_blocking("easyeffects", &["-l", PRESET_NAMES[preset_idx]]).await?;
    }

    Ok(())
}

async fn load_custom_preset(path: PathBuf) -> Result<String, String> {
    let name = extract_file_stem(&path)?;

    let dest = easyeffects_output_dir()?.join(format!("{name}.json"));
    tokio::fs::copy(&path, &dest)
        .await
        .map_err(|e| e.to_string())?;

    ensure_easyeffects_running().await;

    run_command_blocking("easyeffects", &["-b", "2"]).await?;
    run_command_blocking("easyeffects", &["-l", &name]).await?;

    Ok(name)
}

async fn install_presets() -> Result<(), String> {
    let dir = easyeffects_output_dir()?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    for (name, content) in PRESETS {
        let path = dir.join(format!("{}.json", name));
        if !path.exists() {
            tokio::fs::write(&path, content)
                .await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
