use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use crate::services::dbus;

pub struct BatteryModel {
    wartungsmodus_aktiv: bool,
    volle_aufladung_aktiv: bool,
    timer_abbrechen: Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Debug)]
pub enum BatteryMsg {
    WartungsmodusUmschalten(bool),
    VolleAufladungUmschalten(bool),
}

#[derive(Debug)]
pub enum BatteryCommandOutput {
    LadelimitGesetzt(u8),
    Fehler(String),
    TimerAbgelaufen,
    InitWert(u8),
}

#[relm4::component(pub)]
impl Component for BatteryModel {
    type Init = ();
    type Input = BatteryMsg;
    type Output = String;
    type CommandOutput = BatteryCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "Energie &amp; Leistung",

            add = &adw::SwitchRow {
                set_title: "Akku-Wartungsmodus",
                set_subtitle: "Aktivieren Sie den Akku-Wartungsmodus, um die Akkuladung auf 80% der vollen Kapazität zu begrenzen und so die Lebensdauer des Akkus zu verbessern.",

                #[watch]
                set_active: model.wartungsmodus_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::WartungsmodusUmschalten(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: "Volle Aufladung sofort-Modus",
                set_subtitle: "Im Volle Aufladung sofort-Modus wird der Akku zu 100% aufgeladen. Der Akku-Wartungsmodus wird nach 24 Stunden wieder aktiviert.",

                #[watch]
                set_active: model.volle_aufladung_aktiv,

                #[watch]
                set_sensitive: model.wartungsmodus_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::VolleAufladungUmschalten(switch.is_active()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = BatteryModel {
            wartungsmodus_aktiv: false,
            volle_aufladung_aktiv: false,
            timer_abbrechen: None,
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match dbus::get_charge_limit().await {
                        Ok(val) => out.emit(BatteryCommandOutput::InitWert(val)),
                        Err(e) => out.emit(BatteryCommandOutput::Fehler(e)),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: BatteryMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            BatteryMsg::WartungsmodusUmschalten(aktiv) => {
                if aktiv == self.wartungsmodus_aktiv {
                    return;
                }
                self.wartungsmodus_aktiv = aktiv;

                if !aktiv {
                    self.volle_aufladung_aktiv = false;
                    if let Some(cancel) = self.timer_abbrechen.take() {
                        let _ = cancel.send(());
                    }
                    sender.command(|out, shutdown| {
                        shutdown
                            .register(async move {
                                emit_limit_result(&out, 100).await;
                            })
                            .drop_on_shutdown()
                    });
                } else {
                    sender.command(|out, shutdown| {
                        shutdown
                            .register(async move {
                                emit_limit_result(&out, 80).await;
                            })
                            .drop_on_shutdown()
                    });
                }
            }
            BatteryMsg::VolleAufladungUmschalten(aktiv) => {
                if aktiv == self.volle_aufladung_aktiv {
                    return;
                }
                self.volle_aufladung_aktiv = aktiv;

                if let Some(cancel) = self.timer_abbrechen.take() {
                    let _ = cancel.send(());
                }

                if aktiv {
                    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                    self.timer_abbrechen = Some(tx);

                    sender.command(|out, shutdown| {
                        shutdown
                            .register(async move {
                                emit_limit_result(&out, 100).await;

                                tokio::select! {
                                    _ = tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)) => {
                                        out.emit(BatteryCommandOutput::TimerAbgelaufen);
                                    }
                                    _ = rx => {}
                                }
                            })
                            .drop_on_shutdown()
                    });
                } else {
                    sender.command(|out, shutdown| {
                        shutdown
                            .register(async move {
                                emit_limit_result(&out, 80).await;
                            })
                            .drop_on_shutdown()
                    });
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: BatteryCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            BatteryCommandOutput::InitWert(val) => {
                self.wartungsmodus_aktiv = val <= 80;
                self.volle_aufladung_aktiv = false;
            }
            BatteryCommandOutput::LadelimitGesetzt(val) => {
                eprintln!("Ladelimit auf {val}% gesetzt");
            }
            BatteryCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
            BatteryCommandOutput::TimerAbgelaufen => {
                self.volle_aufladung_aktiv = false;
                self.timer_abbrechen = None;
                sender.command(|out, shutdown| {
                    shutdown
                        .register(async move {
                            emit_limit_result(&out, 80).await;
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }
}

async fn emit_limit_result(out: &relm4::Sender<BatteryCommandOutput>, value: u8) {
    match dbus::set_charge_limit(value).await {
        Ok(val) => out.emit(BatteryCommandOutput::LadelimitGesetzt(val)),
        Err(e) => out.emit(BatteryCommandOutput::Fehler(e)),
    }
}
