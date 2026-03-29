use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::backend::dbus;
use crate::backend::dbus::FanProfile;
use crate::services::commands::pkexec_shell;
use crate::services::config::AppConfig;

pub struct FanModel {
    aktuelles_profil: FanProfile,
    tiefschlaf_aktiv: bool,
    check_leistung: gtk::CheckButton,
    check_standard: gtk::CheckButton,
    check_fluester: gtk::CheckButton,
}

#[derive(Debug)]
pub enum FanMsg {
    ProfilWechseln(FanProfile),
    TiefschlafhilfeUmschalten(bool),
}

#[derive(Debug)]
pub enum FanCommandOutput {
    ProfilGesetzt(FanProfile),
    InitTiefschlaf(bool),
    TiefschlafGesetzt(bool),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for FanModel {
    type Init = ();
    type Input = FanMsg;
    type Output = ();
    type CommandOutput = FanCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "Lüftermodus",

            add = &adw::ActionRow {
                set_title: "Leistungsmodus",
                set_subtitle: "Maximiert dynamisch die Kühlleistung für rechenintensive Aufgaben.",
                add_prefix = &model.check_leistung.clone(),
                set_activatable_widget: Some(&model.check_leistung),
            },

            add = &adw::ActionRow {
                set_title: "Standardmodus",
                set_subtitle: "Wählt dynamisch die optimale Lüftergeschwindigkeit für den alltäglichen Gebrauch.",
                add_prefix = &model.check_standard.clone(),
                set_activatable_widget: Some(&model.check_standard),
            },

            add = &adw::ActionRow {
                set_title: "Flüstermodus",
                set_subtitle: "Minimiert dynamisch die Lüftergeschwindigkeit für eine leise Umgebung.",
                add_prefix = &model.check_fluester.clone(),
                set_activatable_widget: Some(&model.check_fluester),
            },

            add = &adw::SwitchRow {
                set_title: "Tiefschlafhilfe",
                set_subtitle: "Um den Akku zu schonen, versetzt die Tiefschlafhilfe das System in den Tiefschlafmodus, wenn es in einem festgelegten Zeitraum zu viel Strom verbraucht hat.",

                #[watch]
                set_active: model.tiefschlaf_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(FanMsg::TiefschlafhilfeUmschalten(switch.is_active()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let check_leistung = gtk::CheckButton::new();
        let check_standard = gtk::CheckButton::new();
        let check_fluester = gtk::CheckButton::new();

        check_standard.set_group(Some(&check_leistung));
        check_fluester.set_group(Some(&check_leistung));

        let config = AppConfig::load();
        let gespeichertes_profil = FanProfile::from(config.fan_profil);
        match gespeichertes_profil {
            FanProfile::Performance => check_leistung.set_active(true),
            FanProfile::Balanced => check_standard.set_active(true),
            FanProfile::Quiet => check_fluester.set_active(true),
        }

        for (btn, profile) in [
            (&check_leistung, FanProfile::Performance),
            (&check_standard, FanProfile::Balanced),
            (&check_fluester, FanProfile::Quiet),
        ] {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FanMsg::ProfilWechseln(profile));
                }
            });
        }

        let model = FanModel {
            aktuelles_profil: gespeichertes_profil,
            tiefschlaf_aktiv: false,
            check_leistung,
            check_standard,
            check_fluester,
        };

        let widgets = view_output!();

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    match dbus::set_fan_profile(gespeichertes_profil).await {
                        Ok(p) => out.emit(FanCommandOutput::ProfilGesetzt(p)),
                        Err(e) => out.emit(FanCommandOutput::Fehler(e)),
                    }
                })
                .drop_on_shutdown()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match tokio::fs::read_to_string("/sys/power/mem_sleep").await {
                        Ok(content) => {
                            let aktiv = content.contains("[deep]");
                            out.emit(FanCommandOutput::InitTiefschlaf(aktiv));
                        }
                        Err(e) => {
                            out.emit(FanCommandOutput::Fehler(format!(
                                "mem_sleep lesen fehlgeschlagen: {e}"
                            )));
                        }
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FanMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FanMsg::ProfilWechseln(profile) => {
                if profile == self.aktuelles_profil {
                    return;
                }
                self.aktuelles_profil = profile;
                AppConfig::update(|c| c.fan_profil = profile as u32);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_fan_profile(profile).await {
                                Ok(p) => out.emit(FanCommandOutput::ProfilGesetzt(p)),
                                Err(e) => out.emit(FanCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            FanMsg::TiefschlafhilfeUmschalten(aktiv) => {
                if aktiv == self.tiefschlaf_aktiv {
                    return;
                }
                self.tiefschlaf_aktiv = aktiv;

                AppConfig::update(|c| c.fan_tiefschlaf_aktiv = aktiv);

                let wert = if aktiv { "deep" } else { "s2idle" };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let cmd = format!("echo {wert} > /sys/power/mem_sleep");
                            match pkexec_shell(&cmd).await {
                                Ok(()) => out.emit(FanCommandOutput::TiefschlafGesetzt(aktiv)),
                                Err(e) => out.emit(FanCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: FanCommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FanCommandOutput::InitTiefschlaf(aktiv) => {
                self.tiefschlaf_aktiv = aktiv;
            }
            FanCommandOutput::ProfilGesetzt(profile) => {
                eprintln!("Lüfterprofil auf {:?} gesetzt", profile);
            }
            FanCommandOutput::TiefschlafGesetzt(aktiv) => {
                eprintln!(
                    "Tiefschlafhilfe auf {} gesetzt",
                    if aktiv { "deep" } else { "s2idle" }
                );
            }
            FanCommandOutput::Fehler(e) => {
                eprintln!("Fehler: {e}");
            }
        }
    }
}
