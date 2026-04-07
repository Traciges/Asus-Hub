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

/// State for the ambient-light-based keyboard backlight automation component.
pub struct AutoBeleuchtungModel {
    /// Whether the `iio-sensor-proxy` D-Bus service is reachable.
    sensor_available: bool,
    /// When `true`, the keyboard backlight is raised to max when lux drops below `brighten_threshold`.
    auto_brighten: bool,
    /// When `true`, the keyboard backlight is turned off when lux exceeds `dim_threshold`.
    auto_dim: bool,
    /// Ambient light level (lux) below which auto-brighten triggers.
    brighten_threshold: f64,
    /// Ambient light level (lux) above which auto-dim triggers.
    dim_threshold: f64,
    /// Sender to shut down the running sensor loop (send `false` to stop).
    loop_tx: Option<watch::Sender<bool>>,
    /// Last reported ambient light level, displayed in the UI while the loop is active.
    current_lux: Option<f64>,
}

#[derive(Debug)]
pub enum AutoBeleuchtungMsg {
    ToggleAutoBrighten(bool),
    ToggleAutoDim(bool),
    BrightenThresholdChanged(f64),
    DimThresholdChanged(f64),
}

#[derive(Debug)]
pub enum AutoBeleuchtungCommandOutput {
    SensorChecked(bool),
    Fehler(String),
    LuxUpdated(f64),
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

            add = &gtk::Label {
                #[watch]
                set_visible: !model.sensor_available,
                set_label: &t!("backlight_sensor_missing_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &adw::ActionRow {
                set_title: &t!("backlight_light_level_title"),
                set_subtitle: &t!("backlight_light_level_subtitle"),

                #[watch]
                set_visible: model.sensor_available && (model.auto_brighten || model.auto_dim),

                add_suffix = &gtk::Label {
                    #[watch]
                    set_label: &match model.current_lux {
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
                set_sensitive: model.sensor_available,
                #[watch]
                set_active: model.auto_brighten,

                connect_active_notify[sender] => move |switch| {
                    sender.input(AutoBeleuchtungMsg::ToggleAutoBrighten(switch.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("backlight_threshold_on_title"),
                set_subtitle: &t!("backlight_threshold_on_subtitle"),

                #[watch]
                set_sensitive: model.sensor_available && model.auto_brighten,

                add_suffix = &gtk::SpinButton::with_range(0.0, 1000.0, 1.0) {
                    set_valign: gtk::Align::Center,

                    #[watch]
                    set_value: model.brighten_threshold,

                    connect_value_changed[sender] => move |spin| {
                        sender.input(AutoBeleuchtungMsg::BrightenThresholdChanged(spin.value()));
                    },
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("backlight_auto_off_title"),
                set_subtitle: &t!("backlight_auto_off_subtitle"),

                #[watch]
                set_sensitive: model.sensor_available,
                #[watch]
                set_active: model.auto_dim,

                connect_active_notify[sender] => move |switch| {
                    sender.input(AutoBeleuchtungMsg::ToggleAutoDim(switch.is_active()));
                },
            },

            add = &adw::ActionRow {
                set_title: &t!("backlight_threshold_off_title"),
                set_subtitle: &t!("backlight_threshold_off_subtitle"),

                #[watch]
                set_sensitive: model.sensor_available && model.auto_dim,

                add_suffix = &gtk::SpinButton::with_range(0.0, 1000.0, 1.0) {
                    set_valign: gtk::Align::Center,

                    #[watch]
                    set_value: model.dim_threshold,

                    connect_value_changed[sender] => move |spin| {
                        sender.input(AutoBeleuchtungMsg::DimThresholdChanged(spin.value()));
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

        let model = AutoBeleuchtungModel {
            sensor_available: false,
            auto_brighten: config.kbd_aufhellung_aktiv,
            auto_dim: config.kbd_abdunklung_aktiv,
            brighten_threshold: config.kbd_aufhellung_schwelle,
            dim_threshold: config.kbd_abdunklung_schwelle,
            loop_tx: None,
            current_lux: None,
        };

        let widgets = view_output!();

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    let available = is_sensor_available().await;
                    out.emit(AutoBeleuchtungCommandOutput::SensorChecked(available));
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(
        &mut self,
        msg: AutoBeleuchtungMsg,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            AutoBeleuchtungMsg::ToggleAutoBrighten(active) => {
                self.auto_brighten = active;
                AppConfig::update(|c| c.kbd_aufhellung_aktiv = active);
                self.update_sensor_loop(sender);
            }
            AutoBeleuchtungMsg::ToggleAutoDim(active) => {
                self.auto_dim = active;
                AppConfig::update(|c| c.kbd_abdunklung_aktiv = active);
                self.update_sensor_loop(sender);
            }
            AutoBeleuchtungMsg::BrightenThresholdChanged(value) => {
                if (value - self.brighten_threshold).abs() > f64::EPSILON {
                    self.brighten_threshold = value;
                    AppConfig::update(|c| c.kbd_aufhellung_schwelle = value);
                    self.update_sensor_loop(sender);
                }
            }
            AutoBeleuchtungMsg::DimThresholdChanged(value) => {
                if (value - self.dim_threshold).abs() > f64::EPSILON {
                    self.dim_threshold = value;
                    AppConfig::update(|c| c.kbd_abdunklung_schwelle = value);
                    self.update_sensor_loop(sender);
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
            AutoBeleuchtungCommandOutput::SensorChecked(available) => {
                self.sensor_available = available;
                if available && (self.auto_brighten || self.auto_dim) {
                    self.loop_tx = Some(start_sensor_loop(
                        self.auto_brighten,
                        self.brighten_threshold,
                        self.auto_dim,
                        self.dim_threshold,
                        &sender,
                    ));
                }
            }
            AutoBeleuchtungCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
            AutoBeleuchtungCommandOutput::LuxUpdated(lux) => {
                self.current_lux = Some(lux);
            }
        }
    }
}

impl AutoBeleuchtungModel {
    fn update_sensor_loop(&mut self, sender: ComponentSender<Self>) {
        let active = self.auto_brighten || self.auto_dim;

        if active {
            if let Some(tx) = &self.loop_tx {
                let _ = tx.send(false);
            }
            self.loop_tx = Some(start_sensor_loop(
                self.auto_brighten,
                self.brighten_threshold,
                self.auto_dim,
                self.dim_threshold,
                &sender,
            ));
        } else {
            if let Some(tx) = self.loop_tx.take() {
                let _ = tx.send(false);
            }
            self.current_lux = None;
        }
    }
}

/// Returns `true` if the `iio-sensor-proxy` D-Bus service is reachable and reports an ambient light sensor.
async fn is_sensor_available() -> bool {
    let conn = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(_) => return false,
    };
    let proxy = match SensorProxyProxy::new(&conn).await {
        Ok(p) => p,
        Err(_) => return false,
    };
    proxy.has_ambient_light().await.is_ok()
}

/// Sets the keyboard backlight brightness level (0–3) via the UPower D-Bus interface.
///
/// Returns `true` on success; failures are treated as non-fatal by callers.
async fn set_kbd_brightness(value: i32) -> bool {
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
            &value.to_string(),
        ],
    )
    .await
    .is_ok()
}

/// Applies auto-brighten/dim rules for a given lux reading and returns the new brightness level.
///
/// Only adjusts brightness if it would actually change, preventing redundant D-Bus calls.
async fn light_sensor_logic(
    level: f64,
    auto_brighten: bool,
    brighten_threshold: f64,
    auto_dim: bool,
    dim_threshold: f64,
    mut current_brightness: i32,
) -> i32 {
    if auto_brighten && level < brighten_threshold && current_brightness != 3 {
        if set_kbd_brightness(3).await {
            current_brightness = 3;
        }
    } else if auto_dim && level > dim_threshold && current_brightness != 0 {
        if set_kbd_brightness(0).await {
            current_brightness = 0;
        }
    }
    current_brightness
}

/// Spawns a background task that monitors the ambient light sensor and adjusts the keyboard
/// backlight in response to lux changes.
///
/// A 3-lux hysteresis is applied — changes smaller than 3 lux are ignored to avoid
/// flickering from sensor noise. The task shuts down when `false` is sent on the returned
/// [`watch::Sender`].
fn start_sensor_loop(
    auto_brighten: bool,
    brighten_threshold: f64,
    auto_dim: bool,
    dim_threshold: f64,
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
        let mut current_brightness: i32 = -1;
        let mut last_level: f64 = -100.0;

        match proxy.light_level().await {
            Ok(level) => {
                last_level = level;
                current_brightness = light_sensor_logic(
                    level,
                    auto_brighten,
                    brighten_threshold,
                    auto_dim,
                    dim_threshold,
                    current_brightness,
                )
                .await;
                out.emit(AutoBeleuchtungCommandOutput::LuxUpdated(level));
            }
            Err(e) => tracing::warn!(
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
                                if (level - last_level).abs() < 3.0 {
                                    continue;
                                }
                                last_level = level;
                                current_brightness = light_sensor_logic(
                                    level,
                                    auto_brighten,
                                    brighten_threshold,
                                    auto_dim,
                                    dim_threshold,
                                    current_brightness,
                                )
                                .await;
                                out.emit(AutoBeleuchtungCommandOutput::LuxUpdated(level));
                            }
                            Err(e) => tracing::warn!(
                                "{}",
                                t!("backlight_sensor_read_error", error = e.to_string())
                            ),
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        let _ = proxy.release_light().await;
    });

    tx
}
