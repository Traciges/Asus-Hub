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

use futures_util::StreamExt;
use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::components::display::helpers::DISPLAY_NAME;
use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

#[zbus::proxy(
    interface = "org.kde.Solid.PowerManagement.Actions.BrightnessControl",
    default_service = "org.kde.Solid.PowerManagement",
    default_path = "/org/kde/Solid/PowerManagement/Actions/BrightnessControl"
)]
trait BrightnessControl {
    #[zbus(signal, name = "brightnessChanged")]
    fn brightness_changed(&self, brightness: i32) -> zbus::Result<()>;
}

pub struct OledDimmingModel {
    brightness: u32,
}

#[derive(Debug)]
pub enum OledDimmingMsg {
    SetBrightness(u32),
}

#[derive(Debug)]
pub enum OledDimmingCommandOutput {
    Set(u32),
    Fehler(String),
    BrightnessChanged,
}

#[relm4::component(pub)]
impl Component for OledDimmingModel {
    type Init = ();
    type Input = OledDimmingMsg;
    type Output = String;
    type CommandOutput = OledDimmingCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("oled_dimming_group_title"),
            set_description: Some(&t!("oled_dimming_group_desc")),

            add = &gtk::Label {
                set_label: &t!("oled_dimming_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::ActionRow {
                set_title: &t!("oled_dimming_slider_title"),

                add_suffix = &gtk::Scale {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_range: (10.0, 100.0),
                    set_increments: (5.0, 10.0),
                    set_round_digits: 0,
                    set_value: model.brightness as f64,
                    set_width_request: 200,
                    connect_value_changed[sender] => move |scale| {
                        sender.input(OledDimmingMsg::SetBrightness(scale.value() as u32));
                    },
                },

                add_suffix = &gtk::Label {
                    #[watch]
                    set_label: &format!("{}%", model.brightness),
                    set_width_chars: 5,
                    set_xalign: 1.0,
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
        let brightness = config.oled_dc_dimming;

        let model = OledDimmingModel { brightness };
        let widgets = view_output!();

        if brightness < 100 {
            sender.command(move |out, shutdown| {
                shutdown
                    .register(async move {
                        match apply_dimming(brightness).await {
                            Ok(()) => out.emit(OledDimmingCommandOutput::Set(brightness)),
                            Err(e) => out.emit(OledDimmingCommandOutput::Fehler(e)),
                        }
                    })
                    .drop_on_shutdown()
            });
        }

        let out = sender.command_sender().clone();
        tokio::spawn(start_brightness_listener(out));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: OledDimmingMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            OledDimmingMsg::SetBrightness(value) => {
                if value == self.brightness {
                    return;
                }
                self.brightness = value;
                AppConfig::update(|c| c.oled_dc_dimming = value);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match apply_dimming(value).await {
                                Ok(()) => out.emit(OledDimmingCommandOutput::Set(value)),
                                Err(e) => out.emit(OledDimmingCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: OledDimmingCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            OledDimmingCommandOutput::Set(value) => {
                tracing::info!("{}", t!("oled_dimming_set", value = value.to_string()));
            }
            OledDimmingCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
            OledDimmingCommandOutput::BrightnessChanged => {
                let value = self.brightness;
                if value < 100 {
                    sender.command(move |out, shutdown| {
                        shutdown
                            .register(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                match apply_dimming(value).await {
                                    Ok(()) => out.emit(OledDimmingCommandOutput::Set(value)),
                                    Err(e) => out.emit(OledDimmingCommandOutput::Fehler(e)),
                                }
                            })
                            .drop_on_shutdown()
                    });
                }
            }
        }
    }
}

async fn apply_dimming(value: u32) -> Result<(), String> {
    let arg = format!("output.{}.dimming.{}", DISPLAY_NAME, value);
    run_command_blocking("kscreen-doctor", &[&arg]).await
}

async fn start_brightness_listener(out: relm4::Sender<OledDimmingCommandOutput>) {
    let conn = match zbus::Connection::session().await {
        Ok(c) => c,
        Err(_) => return,
    };
    let proxy = match BrightnessControlProxy::new(&conn).await {
        Ok(p) => p,
        Err(_) => return,
    };
    let mut stream = match proxy.receive_brightness_changed().await {
        Ok(s) => s,
        Err(_) => return,
    };
    while stream.next().await.is_some() {
        out.emit(OledDimmingCommandOutput::BrightnessChanged);
    }
}
