use futures_util::StreamExt;
use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use tokio::sync::watch;

use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

// ──────────────────────────────────────────────────────────────────────────────
// iio-sensor-proxy D-Bus Proxy
// ──────────────────────────────────────────────────────────────────────────────

#[zbus::proxy(
    interface = "net.hadess.SensorProxy",
    default_service = "net.hadess.SensorProxy",
    default_path = "/net/hadess/SensorProxy"
)]
trait SensorProxy {
    fn claim_light(&self) -> zbus::Result<()>;
    fn release_light(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn light_level(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn has_ambient_light(&self) -> zbus::Result<bool>;
}

// ──────────────────────────────────────────────────────────────────────────────
// Automatische Tastaturhintergrundbeleuchtung
// ──────────────────────────────────────────────────────────────────────────────

pub struct AutoBeleuchtungModel {
    aufhellung_aktiv: bool,
    abdunklung_aktiv: bool,
    aufhellung_schwelle: f64,
    abdunklung_schwelle: f64,
    loop_tx: Option<watch::Sender<bool>>,
    aktuelle_lux: Option<f64>,
}

#[derive(Debug)]
pub enum AutoBeleuchtungMsg {
    AufhellungUmschalten(bool),
    AbdunklungUmschalten(bool),
    AufhellungSchwelleGeaendert(f64),
    AbdunklungSchwelleGeaendert(f64),
}

#[derive(Debug)]
pub enum AutoBeleuchtungCommandOutput {
    Fehler(String),
    LuxAktualisiert(f64),
}

#[relm4::component(pub)]
impl Component for AutoBeleuchtungModel {
    type Init = ();
    type Input = AutoBeleuchtungMsg;
    type Output = String;
    type CommandOutput = AutoBeleuchtungCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("backlight_group_title"),
            set_description: Some(&t!("backlight_group_desc")),

            add = &adw::ActionRow {
                set_title: &t!("backlight_light_level_title"),
                set_subtitle: &t!("backlight_light_level_subtitle"),

                #[watch]
                set_visible: model.aufhellung_aktiv || model.abdunklung_aktiv,

                add_suffix = &gtk::Label {
                    #[watch]
                    set_label: &match model.aktuelle_lux {
                        Some(lux) => format!("{lux:.1} lx"),
                        None => t!("backlight_no_lux").to_string(),
                    },
                    add_css_class: "numeric",
                    set_valign: gtk::Align::Center,
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("backlight_auto_on_title"),
                set_subtitle: &t!("backlight_auto_on_subtitle"),

                #[watch]
                set_active: model.aufhellung_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(AutoBeleuchtungMsg::AufhellungUmschalten(switch.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("backlight_threshold_on_title"),
                set_subtitle: &t!("backlight_threshold_on_subtitle"),

                #[watch]
                set_sensitive: model.aufhellung_aktiv,

                add_suffix = &gtk::SpinButton::with_range(0.0, 1000.0, 1.0) {
                    set_valign: gtk::Align::Center,

                    #[watch]
                    set_value: model.aufhellung_schwelle,

                    connect_value_changed[sender] => move |spin| {
                        sender.input(AutoBeleuchtungMsg::AufhellungSchwelleGeaendert(spin.value()));
                    },
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("backlight_auto_off_title"),
                set_subtitle: &t!("backlight_auto_off_subtitle"),

                #[watch]
                set_active: model.abdunklung_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(AutoBeleuchtungMsg::AbdunklungUmschalten(switch.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("backlight_threshold_off_title"),
                set_subtitle: &t!("backlight_threshold_off_subtitle"),

                #[watch]
                set_sensitive: model.abdunklung_aktiv,

                add_suffix = &gtk::SpinButton::with_range(0.0, 1000.0, 1.0) {
                    set_valign: gtk::Align::Center,

                    #[watch]
                    set_value: model.abdunklung_schwelle,

                    connect_value_changed[sender] => move |spin| {
                        sender.input(AutoBeleuchtungMsg::AbdunklungSchwelleGeaendert(spin.value()));
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
        let config = AppConfig::load();
        let aufhellung = config.kbd_aufhellung_aktiv;
        let abdunklung = config.kbd_abdunklung_aktiv;
        let aufhellung_schwelle = config.kbd_aufhellung_schwelle;
        let abdunklung_schwelle = config.kbd_abdunklung_schwelle;

        let loop_tx = if aufhellung || abdunklung {
            Some(start_sensor_loop(
                aufhellung,
                aufhellung_schwelle,
                abdunklung,
                abdunklung_schwelle,
                &sender,
            ))
        } else {
            None
        };

        let model = AutoBeleuchtungModel {
            aufhellung_aktiv: aufhellung,
            abdunklung_aktiv: abdunklung,
            aufhellung_schwelle,
            abdunklung_schwelle,
            loop_tx,
            aktuelle_lux: None,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: AutoBeleuchtungMsg,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AutoBeleuchtungMsg::AufhellungUmschalten(aktiv) => {
                self.aufhellung_aktiv = aktiv;
                AppConfig::update(|c| c.kbd_aufhellung_aktiv = aktiv);
                self.sensor_loop_aktualisieren(sender);
            }
            AutoBeleuchtungMsg::AbdunklungUmschalten(aktiv) => {
                self.abdunklung_aktiv = aktiv;
                AppConfig::update(|c| c.kbd_abdunklung_aktiv = aktiv);
                self.sensor_loop_aktualisieren(sender);
            }
            AutoBeleuchtungMsg::AufhellungSchwelleGeaendert(wert) => {
                if (wert - self.aufhellung_schwelle).abs() > f64::EPSILON {
                    self.aufhellung_schwelle = wert;
                    AppConfig::update(|c| c.kbd_aufhellung_schwelle = wert);
                    self.sensor_loop_aktualisieren(sender);
                }
            }
            AutoBeleuchtungMsg::AbdunklungSchwelleGeaendert(wert) => {
                if (wert - self.abdunklung_schwelle).abs() > f64::EPSILON {
                    self.abdunklung_schwelle = wert;
                    AppConfig::update(|c| c.kbd_abdunklung_schwelle = wert);
                    self.sensor_loop_aktualisieren(sender);
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: AutoBeleuchtungCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AutoBeleuchtungCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
            AutoBeleuchtungCommandOutput::LuxAktualisiert(lux) => {
                self.aktuelle_lux = Some(lux);
            }
        }
    }
}

impl AutoBeleuchtungModel {
    fn sensor_loop_aktualisieren(&mut self, sender: ComponentSender<Self>) {
        let aktiv = self.aufhellung_aktiv || self.abdunklung_aktiv;

        if aktiv {
            // Stoppe vorherigen Loop falls vorhanden
            if let Some(tx) = &self.loop_tx {
                let _ = tx.send(false);
            }
            self.loop_tx = Some(start_sensor_loop(
                self.aufhellung_aktiv,
                self.aufhellung_schwelle,
                self.abdunklung_aktiv,
                self.abdunklung_schwelle,
                &sender,
            ));
        } else {
            if let Some(tx) = self.loop_tx.take() {
                let _ = tx.send(false);
            }
            self.aktuelle_lux = None;
        }
    }
}

async fn kbd_helligkeit_setzen(wert: i32) -> bool {
    run_command_blocking(
        "busctl",
        &[
            "call",
            "--system",
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower/KbdBacklight",
            "org.freedesktop.UPower.KbdBacklight",
            "SetBrightness",
            "i",
            &wert.to_string(),
        ],
    )
    .await
    .is_ok()
}

async fn lichtsensor_logik(
    level: f64,
    aufhellung: bool,
    aufhellung_schwelle: f64,
    abdunklung: bool,
    abdunklung_schwelle: f64,
    mut aktuelle_helligkeit: i32,
) -> i32 {
    if aufhellung && level < aufhellung_schwelle && aktuelle_helligkeit != 3 {
        if kbd_helligkeit_setzen(3).await {
            aktuelle_helligkeit = 3;
        }
    } else if abdunklung && level > abdunklung_schwelle && aktuelle_helligkeit != 0 {
        if kbd_helligkeit_setzen(0).await {
            aktuelle_helligkeit = 0;
        }
    }
    aktuelle_helligkeit
}

fn start_sensor_loop(
    aufhellung: bool,
    aufhellung_schwelle: f64,
    abdunklung: bool,
    abdunklung_schwelle: f64,
    sender: &ComponentSender<AutoBeleuchtungModel>,
) -> watch::Sender<bool> {
    let (tx, mut rx) = watch::channel(true);
    let out = sender.command_sender().clone();

    tokio::spawn(async move {
        let conn = match zbus::Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                out.emit(AutoBeleuchtungCommandOutput::Fehler(
                    t!("error_dbus_connection", error = e.to_string()).to_string(),
                ));
                return;
            }
        };

        let proxy = match SensorProxyProxy::new(&conn).await {
            Ok(p) => p,
            Err(e) => {
                out.emit(AutoBeleuchtungCommandOutput::Fehler(
                    t!("error_sensor_proxy", error = e.to_string()).to_string(),
                ));
                return;
            }
        };

        if let Err(e) = proxy.claim_light().await {
            out.emit(AutoBeleuchtungCommandOutput::Fehler(
                t!("error_claim_light", error = e.to_string()).to_string(),
            ));
            return;
        }

        let level_stream = proxy.receive_light_level_changed().await;
        let mut aktuelle_helligkeit: i32 = -1;
        let mut letztes_level: f64 = -100.0;

        // Startwert einmalig auslesen und Logik anwenden
        match proxy.light_level().await {
            Ok(level) => {
                letztes_level = level;
                aktuelle_helligkeit = lichtsensor_logik(
                    level,
                    aufhellung,
                    aufhellung_schwelle,
                    abdunklung,
                    abdunklung_schwelle,
                    aktuelle_helligkeit,
                )
                .await;
                out.emit(AutoBeleuchtungCommandOutput::LuxAktualisiert(level));
            }
            Err(e) => eprintln!(
                "{}",
                t!("backlight_sensor_init_error", error = e.to_string())
            ),
        }

        tokio::pin!(level_stream);

        loop {
            tokio::select! {
                _ = rx.changed() => {
                    if !*rx.borrow() {
                        break;
                    }
                }
                maybe = level_stream.next() => {
                    if let Some(changed) = maybe {
                        match changed.get().await {
                            Ok(level) => {
                                if (level - letztes_level).abs() < 3.0 {
                                    continue;
                                }
                                letztes_level = level;
                                aktuelle_helligkeit = lichtsensor_logik(
                                    level,
                                    aufhellung,
                                    aufhellung_schwelle,
                                    abdunklung,
                                    abdunklung_schwelle,
                                    aktuelle_helligkeit,
                                )
                                .await;
                                out.emit(AutoBeleuchtungCommandOutput::LuxAktualisiert(level));
                            }
                            Err(e) => eprintln!(
                                "{}",
                                t!("backlight_sensor_read_error", error = e.to_string())
                            ),
                        }
                    } else {
                        // Stream beendet
                        break;
                    }
                }
            }
        }

        let _ = proxy.release_light().await;
    });

    tx
}
