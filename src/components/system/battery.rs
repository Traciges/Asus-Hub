use gtk4::glib;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::commands::pkexec_shell;
use crate::services::config::AppConfig;
use crate::services::dbus;

/// State for the battery settings component.
pub struct BatteryModel {
    /// Whether the `asusd` daemon is reachable; disables charge controls when `false`.
    asusd_available: bool,
    /// When `true`, the charge limit is capped at 80% to extend long-term battery health.
    maintenance_mode_active: bool,
    /// When `true`, the charge limit is temporarily raised to 100% for up to 24 hours.
    full_charge_active: bool,
    /// When `true`, suspend-to-RAM uses the `deep` sleep state instead of `s2idle`.
    deep_sleep_active: bool,
    /// Cancels the 24-hour full-charge revert timer when the user turns off full-charge mode early.
    timer_cancel: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Input messages for the battery component.
#[derive(Debug)]
pub enum BatteryMsg {
    /// Enable or disable the 80% charge maintenance mode.
    ToggleMaintenanceMode(bool),
    /// Enable or disable the temporary 100% full-charge mode (auto-reverts after 24 h).
    ToggleFullCharge(bool),
    /// Switch suspend-to-RAM between `s2idle` (`false`) and `deep` (`true`).
    ToggleDeepSleep(bool),
}

/// Async command results for the battery component.
#[derive(Debug)]
pub enum BatteryCommandOutput {
    /// Result of the initial `asusd` availability check.
    AsusdChecked(bool),
    /// Confirmation that the charge limit was successfully written.
    ChargeLimitSet(u8),
    /// An error message to forward as a toast notification.
    Fehler(String),
    /// Fired after the 24-hour full-charge timer expires, triggering a revert to 80%.
    TimerElapsed,
    /// Charge limit value read from `asusd` during initialisation.
    InitValue(u8),
    /// Deep-sleep state read from `/sys/power/mem_sleep` during initialisation.
    InitDeepSleep(bool),
    /// Confirmation that the `/sys/power/mem_sleep` write succeeded.
    DeepSleepSet(bool),
}

#[relm4::component(pub)]
impl Component for BatteryModel {
    type Init = ();
    type Input = BatteryMsg;
    type Output = String;
    type CommandOutput = BatteryCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &glib::markup_escape_text(&t!("battery_group_title")),
            set_description: Some(&t!("battery_group_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.asusd_available,
                set_label: &t!("asusd_missing_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::SwitchRow {
                set_title: &t!("battery_maintenance_title"),
                set_subtitle: &t!("battery_maintenance_subtitle"),

                #[watch]
                set_active: model.maintenance_mode_active,

                #[watch]
                set_sensitive: model.asusd_available,

                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::ToggleMaintenanceMode(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("battery_full_charge_title"),
                set_subtitle: &t!("battery_full_charge_subtitle"),

                #[watch]
                set_active: model.full_charge_active,

                #[watch]
                set_sensitive: model.asusd_available && model.maintenance_mode_active,

                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::ToggleFullCharge(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("battery_deep_sleep_title"),
                set_subtitle: &t!("battery_deep_sleep_subtitle"),

                #[watch]
                set_active: model.deep_sleep_active,

                connect_active_notify[sender] => move |switch| {
                    sender.input(BatteryMsg::ToggleDeepSleep(switch.is_active()));
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
            asusd_available: false,
            maintenance_mode_active: false,
            full_charge_active: false,
            deep_sleep_active: false,
            timer_cancel: None,
        };
        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let available = dbus::check_asusd_available().await;
                    out.emit(BatteryCommandOutput::AsusdChecked(available));
                })
                .drop_on_shutdown()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match dbus::get_charge_limit().await {
                        Ok(val) => out.emit(BatteryCommandOutput::InitValue(val)),
                        Err(e) => out.emit(BatteryCommandOutput::Fehler(e)),
                    }
                })
                .drop_on_shutdown()
        });

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    match tokio::fs::read_to_string("/sys/power/mem_sleep").await {
                        Ok(content) => {
                            let active = content.contains("[deep]");
                            out.emit(BatteryCommandOutput::InitDeepSleep(active));
                        }
                        Err(e) => {
                            out.emit(BatteryCommandOutput::Fehler(
                                t!("error_mem_sleep_read", error = e.to_string()).to_string(),
                            ));
                        }
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: BatteryMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            BatteryMsg::ToggleMaintenanceMode(active) => {
                if active == self.maintenance_mode_active {
                    return;
                }
                self.maintenance_mode_active = active;

                if !active {
                    self.full_charge_active = false;
                    if let Some(cancel) = self.timer_cancel.take() {
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
            BatteryMsg::ToggleDeepSleep(active) => {
                if active == self.deep_sleep_active {
                    return;
                }
                self.deep_sleep_active = active;
                AppConfig::update(|c| c.battery_tiefschlaf_aktiv = active);
                let value = if active { "deep" } else { "s2idle" };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let cmd = format!("echo {value} > /sys/power/mem_sleep");
                            match pkexec_shell(&cmd).await {
                                Ok(()) => out.emit(BatteryCommandOutput::DeepSleepSet(active)),
                                Err(e) => out.emit(BatteryCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            BatteryMsg::ToggleFullCharge(active) => {
                if active == self.full_charge_active {
                    return;
                }
                self.full_charge_active = active;

                if let Some(cancel) = self.timer_cancel.take() {
                    let _ = cancel.send(());
                }

                if active {
                    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                    self.timer_cancel = Some(tx);

                    sender.command(|out, shutdown| {
                        shutdown
                            .register(async move {
                                emit_limit_result(&out, 100).await;

                                tokio::select! {
                                    _ = tokio::time::sleep(std::time::Duration::from_secs(24 * 60 * 60)) => {
                                        out.emit(BatteryCommandOutput::TimerElapsed);
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
            BatteryCommandOutput::AsusdChecked(available) => {
                self.asusd_available = available;
            }
            BatteryCommandOutput::InitValue(val) => {
                self.maintenance_mode_active = val <= 80;
                self.full_charge_active = false;
            }
            BatteryCommandOutput::InitDeepSleep(active) => {
                self.deep_sleep_active = active;
            }
            BatteryCommandOutput::DeepSleepSet(active) => {
                let value = if active { "deep" } else { "s2idle" };
                tracing::info!("{}", t!("battery_deep_sleep_set", value = value));
            }
            BatteryCommandOutput::ChargeLimitSet(val) => {
                tracing::info!(
                    "{}",
                    t!("battery_charge_limit_set", value = val.to_string())
                );
            }
            BatteryCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
            BatteryCommandOutput::TimerElapsed => {
                self.full_charge_active = false;
                self.timer_cancel = None;
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

/// Calls [`dbus::set_charge_limit`] and emits either [`BatteryCommandOutput::ChargeLimitSet`]
/// or [`BatteryCommandOutput::Fehler`] depending on the outcome.
async fn emit_limit_result(out: &relm4::Sender<BatteryCommandOutput>, value: u8) {
    match dbus::set_charge_limit(value).await {
        Ok(val) => out.emit(BatteryCommandOutput::ChargeLimitSet(val)),
        Err(e) => out.emit(BatteryCommandOutput::Fehler(e)),
    }
}
