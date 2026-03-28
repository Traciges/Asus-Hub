use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::services::config::AppConfig;

pub struct FnKeyModel {
    gesperrt: bool,
    unterstuetzt: bool,
    check_gesperrt: gtk::CheckButton,
    check_normal: gtk::CheckButton,
    zeile_hinweis: adw::ActionRow,
    zeile_gesperrt: adw::ActionRow,
    zeile_normal: adw::ActionRow,
}

#[derive(Debug)]
pub enum FnKeyMsg {
    GesperrtUmschalten(bool),
}

#[derive(Debug)]
pub enum FnKeyCommandOutput {
    InitWert { gesperrt: bool, unterstuetzt: bool },
    Gesetzt(bool),
    Fehler(String),
}

const MODPROBE_PFAD: &str = "/etc/modprobe.d/asus_wmi.conf";
const SYSFS_PFAD: &str = "/sys/module/asus_wmi/parameters/fnlock_default";

#[relm4::component(pub)]
impl Component for FnKeyModel {
    type Init = ();
    type Input = FnKeyMsg;
    type Output = ();
    type CommandOutput = FnKeyCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "Funktionstaste",

            add = &model.zeile_hinweis.clone(),
            add = &model.zeile_gesperrt.clone(),
            add = &model.zeile_normal.clone(),
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let check_gesperrt = gtk::CheckButton::new();
        let check_normal = gtk::CheckButton::new();

        check_normal.set_group(Some(&check_gesperrt));
        check_normal.set_active(true);

        {
            let sender = sender.clone();
            check_gesperrt.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FnKeyMsg::GesperrtUmschalten(true));
                }
            });
        }
        {
            let sender = sender.clone();
            check_normal.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FnKeyMsg::GesperrtUmschalten(false));
                }
            });
        }

        let zeile_hinweis = adw::ActionRow::new();
        zeile_hinweis.set_title("Hinweis");
        zeile_hinweis.set_subtitle("Wird geprüft …");
        zeile_hinweis.set_selectable(false);

        let zeile_gesperrt = adw::ActionRow::new();
        zeile_gesperrt.set_title("Gesperrte Fn-Taste");
        zeile_gesperrt.set_subtitle(
            "Drücken Sie F1–F12, um die angegebene Schnelltasten-Funktion zu aktivieren.",
        );
        zeile_gesperrt.add_prefix(&check_gesperrt);
        zeile_gesperrt.set_activatable_widget(Some(&check_gesperrt));

        let zeile_normal = adw::ActionRow::new();
        zeile_normal.set_title("Normale Fn-Taste");
        zeile_normal
            .set_subtitle("Drücken Sie F1–F12, um die F1–F12-Funktionen zu verwenden.");
        zeile_normal.add_prefix(&check_normal);
        zeile_normal.set_activatable_widget(Some(&check_normal));

        let model = FnKeyModel {
            gesperrt: false,
            unterstuetzt: true,
            check_gesperrt,
            check_normal,
            zeile_hinweis,
            zeile_gesperrt,
            zeile_normal,
        };

        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    // Prüfen ob sysfs-Parameter beschreibbar ist (Live-Änderung möglich)
                    let unterstuetzt = std::fs::OpenOptions::new()
                        .write(true)
                        .open(SYSFS_PFAD)
                        .is_ok();

                    let gesperrt = match tokio::fs::read_to_string(MODPROBE_PFAD).await {
                        Ok(inhalt) => inhalt.contains("fnlock_default=1"),
                        Err(_) => false,
                    };

                    out.emit(FnKeyCommandOutput::InitWert { gesperrt, unterstuetzt });
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FnKeyMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FnKeyMsg::GesperrtUmschalten(gesperrt) => {
                if gesperrt == self.gesperrt {
                    return;
                }
                self.gesperrt = gesperrt;

                AppConfig::update(|c| c.fn_key_gesperrt = gesperrt);

                let wert = if gesperrt { 1 } else { 0 };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let result = tokio::task::spawn_blocking(move || {
                                // Zuerst Live-Änderung versuchen (2>/dev/null, Fehler ignorieren),
                                // dann Modprobe-Datei für Persistenz nach Neustart schreiben.
                                std::process::Command::new("pkexec")
                                    .args([
                                        "sh",
                                        "-c",
                                        &format!(
                                            "echo {wert} > {SYSFS_PFAD} 2>/dev/null; \
                                             echo 'options asus_wmi fnlock_default={wert}' > {MODPROBE_PFAD}"
                                        ),
                                    ])
                                    .status()
                            })
                            .await;

                            match result {
                                Ok(Ok(status)) if status.success() => {
                                    out.emit(FnKeyCommandOutput::Gesetzt(gesperrt));
                                }
                                Ok(Ok(status)) => {
                                    out.emit(FnKeyCommandOutput::Fehler(format!(
                                        "pkexec fehlgeschlagen mit Exit-Code: {}",
                                        status.code().unwrap_or(-1)
                                    )));
                                }
                                Ok(Err(e)) => {
                                    out.emit(FnKeyCommandOutput::Fehler(format!(
                                        "pkexec starten fehlgeschlagen: {e}"
                                    )));
                                }
                                Err(e) => {
                                    out.emit(FnKeyCommandOutput::Fehler(format!(
                                        "spawn_blocking fehlgeschlagen: {e}"
                                    )));
                                }
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: FnKeyCommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FnKeyCommandOutput::InitWert { gesperrt, unterstuetzt } => {
                self.gesperrt = gesperrt;
                self.unterstuetzt = unterstuetzt;

                if gesperrt {
                    self.check_gesperrt.set_active(true);
                } else {
                    self.check_normal.set_active(true);
                }

                if unterstuetzt {
                    self.zeile_hinweis.set_subtitle(
                        "Änderungen werden erst nach einem Systemneustart wirksam.",
                    );
                } else {
                    self.check_gesperrt.set_sensitive(false);
                    self.check_normal.set_sensitive(false);
                    self.zeile_gesperrt.set_sensitive(false);
                    self.zeile_normal.set_sensitive(false);
                    self.zeile_hinweis.set_subtitle(
                        "Diese Hardware unterstützt keine Software-Steuerung der Fn-Taste. \
                         Verwende Fn+Esc (physischer Toggle, falls vorhanden) oder installiere keyd.",
                    );
                }
            }
            FnKeyCommandOutput::Gesetzt(gesperrt) => {
                self.zeile_hinweis.set_subtitle(&format!(
                    "Fn-Taste {} gespeichert – wirksam nach Systemneustart.",
                    if gesperrt { "gesperrt" } else { "normal" }
                ));
            }
            FnKeyCommandOutput::Fehler(e) => {
                eprintln!("Fehler (FnKey): {e}");
                self.zeile_hinweis.set_subtitle(&format!("Fehler beim Speichern: {e}"));
            }
        }
    }
}
