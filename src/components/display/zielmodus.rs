use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use super::helpers::{kwriteconfig_ausfuehren, qdbus_ausfuehren};
use crate::services::config::AppConfig;

pub struct ZielmodusModel {
    aktiv: bool,
    staerke: u32,
    kde_verfuegbar: bool,
}

#[derive(Debug)]
pub enum ZielmodusMsg {
    AktivSetzen(bool),
    StaerkeSetzen(u32),
}

#[derive(Debug)]
pub enum ZielmodusCommandOutput {
    AktivGesetzt(bool),
    StaerkeGesetzt(u32),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for ZielmodusModel {
    type Init = ();
    type Input = ZielmodusMsg;
    type Output = String;
    type CommandOutput = ZielmodusCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("zielmodus_group_title"),
            set_description: Some(&t!("zielmodus_group_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.kde_verfuegbar,
                set_label: &t!("zielmodus_kde_required"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::SwitchRow {
                set_title: &t!("zielmodus_switch_title"),
                set_subtitle: &t!("zielmodus_switch_subtitle"),

                #[watch]
                set_active: model.aktiv,
                #[watch]
                set_sensitive: model.kde_verfuegbar,

                connect_active_notify[sender] => move |switch| {
                    sender.input(ZielmodusMsg::AktivSetzen(switch.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("zielmodus_strength_title"),

                #[watch]
                set_sensitive: model.aktiv && model.kde_verfuegbar,

                add_suffix = &gtk::Scale {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_range: (0.0, 100.0),
                    set_increments: (5.0, 10.0),
                    set_round_digits: 0,
                    set_value: model.staerke as f64,
                    set_width_request: 200,
                    connect_value_changed[sender] => move |scale| {
                        sender.input(ZielmodusMsg::StaerkeSetzen(scale.value() as u32));
                    },
                },

                add_suffix = &gtk::Label {
                    #[watch]
                    set_label: &format!("{}%", model.staerke),
                    set_width_chars: 4,
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
        let mut config = AppConfig::load();
        let kde_verfuegbar = ist_kde();

        let (aktiv, staerke) = if kde_verfuegbar {
            let a =
                lese_kwin_bool("Plugins", "diminactiveEnabled").unwrap_or(config.zielmodus_aktiv);
            let s = lese_kwin_u32("Effect-DimInactive", "Strength", config.zielmodus_staerke)
                .unwrap_or(config.zielmodus_staerke);
            config.zielmodus_aktiv = a;
            config.zielmodus_staerke = s;
            config.save();
            (a, s)
        } else {
            (config.zielmodus_aktiv, config.zielmodus_staerke)
        };

        let model = ZielmodusModel {
            aktiv,
            staerke,
            kde_verfuegbar,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: ZielmodusMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ZielmodusMsg::AktivSetzen(aktiv) => {
                if aktiv == self.aktiv {
                    return;
                }
                self.aktiv = aktiv;
                AppConfig::update(|c| c.zielmodus_aktiv = aktiv);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match kwin_effekt_setzen(aktiv).await {
                                Ok(()) => out.emit(ZielmodusCommandOutput::AktivGesetzt(aktiv)),
                                Err(e) => out.emit(ZielmodusCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            ZielmodusMsg::StaerkeSetzen(staerke) => {
                if staerke == self.staerke {
                    return;
                }
                self.staerke = staerke;
                AppConfig::update(|c| c.zielmodus_staerke = staerke);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match kwin_staerke_setzen(staerke).await {
                                Ok(()) => out.emit(ZielmodusCommandOutput::StaerkeGesetzt(staerke)),
                                Err(e) => out.emit(ZielmodusCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: ZielmodusCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ZielmodusCommandOutput::AktivGesetzt(aktiv) => {
                eprintln!("{}", t!("zielmodus_aktiv_set", value = aktiv.to_string()));
            }
            ZielmodusCommandOutput::StaerkeGesetzt(staerke) => {
                eprintln!(
                    "{}",
                    t!("zielmodus_staerke_set", value = staerke.to_string())
                );
            }
            ZielmodusCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

async fn kwin_effekt_setzen(aktiv: bool) -> Result<(), String> {
    let wert = if aktiv { "true" } else { "false" };
    kwriteconfig_ausfuehren(&[
        "--file",
        "kwinrc",
        "--group",
        "Plugins",
        "--key",
        "diminactiveEnabled",
        "--type",
        "bool",
        wert,
    ])
    .await?;

    let method = if aktiv { "loadEffect" } else { "unloadEffect" };
    qdbus_ausfuehren(vec![
        "org.kde.KWin".to_string(),
        "/Effects".to_string(),
        method.to_string(),
        "diminactive".to_string(),
    ])
    .await
}

async fn kwin_staerke_setzen(staerke: u32) -> Result<(), String> {
    let staerke_str = staerke.to_string();
    kwriteconfig_ausfuehren(&[
        "--file",
        "kwinrc",
        "--group",
        "Effect-DimInactive",
        "--key",
        "Strength",
        &staerke_str,
    ])
    .await?;

    qdbus_ausfuehren(vec![
        "org.kde.KWin".to_string(),
        "/Effects".to_string(),
        "reconfigureEffect".to_string(),
        "diminactive".to_string(),
    ])
    .await
}

fn ist_kde() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
}

fn lese_kwin_bool(group: &str, key: &str) -> Option<bool> {
    let output = std::process::Command::new("kreadconfig6")
        .args([
            "--file",
            "kwinrc",
            "--group",
            group,
            "--key",
            key,
            "--default",
            "false",
        ])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_lowercase();
    Some(s == "true")
}

fn lese_kwin_u32(group: &str, key: &str, default: u32) -> Option<u32> {
    let default_str = default.to_string();
    let output = std::process::Command::new("kreadconfig6")
        .args([
            "--file",
            "kwinrc",
            "--group",
            group,
            "--key",
            key,
            "--default",
            &default_str,
        ])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}
