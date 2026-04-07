use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::components::display::helpers::run_qdbus;
use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

/// State for the touchpad enable/disable component.
pub struct TouchpadModel {
    /// Whether the touchpad is currently enabled.
    touchpad_active: bool,
    /// Remaining seconds on the disable-confirmation countdown (starts at 10).
    countdown: u8,
    /// When `true`, the confirmation row is shown and the auto-revert timer is running.
    confirmation_required: bool,
    /// Handle for the 10-second countdown task; abort it to cancel the revert.
    timer_handle: Option<tokio::task::JoinHandle<()>>,
}

/// Input messages for the touchpad component.
#[derive(Debug)]
pub enum TouchpadMsg {
    /// User toggled the enable/disable switch.
    ToggleTouchpad(bool),
    /// User pressed the confirm button within the 10-second window after disabling.
    ConfirmClicked,
}

/// Async command results for the touchpad component.
#[derive(Debug)]
pub enum TouchpadCommandOutput {
    /// An error message to forward as a toast notification.
    Fehler(String),
    /// Fired every second to decrement the on-screen countdown.
    CountdownTick,
    /// Fired when the 10-second confirmation window expires without user action,
    /// triggering an automatic re-enable of the touchpad.
    TimerElapsed,
}

#[relm4::component(pub)]
impl Component for TouchpadModel {
    type Init = ();
    type Input = TouchpadMsg;
    type Output = String;
    type CommandOutput = TouchpadCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("touchpad_group_title"),

            add = &gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",

                append = &adw::SwitchRow {
                    set_title: &t!("touchpad_enable_title"),
                    set_subtitle: &t!("touchpad_enable_subtitle"),

                    #[watch]
                    set_active: model.touchpad_active,

                    connect_active_notify[sender] => move |s| {
                        sender.input(TouchpadMsg::ToggleTouchpad(s.is_active()));
                    },
                },

                append = &adw::ActionRow {
                    #[watch]
                    set_visible: model.confirmation_required,

                    #[watch]
                    set_title: &t!("touchpad_countdown_title", seconds = model.countdown.to_string()),

                    add_suffix = &gtk::Button {
                        set_label: &t!("touchpad_confirm_label"),
                        add_css_class: "suggested-action",
                        set_valign: gtk::Align::Center,

                        connect_clicked[sender] => move |_| {
                            sender.input(TouchpadMsg::ConfirmClicked);
                        },
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
        let touchpad_active = AppConfig::load().touchpad_aktiv;
        let model = TouchpadModel {
            touchpad_active,
            countdown: 10,
            confirmation_required: false,
            timer_handle: None,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: TouchpadMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            TouchpadMsg::ToggleTouchpad(active) => {
                if active == self.touchpad_active {
                    return;
                }
                self.touchpad_active = active;

                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }

                if !active {
                    self.confirmation_required = true;
                    self.countdown = 10;

                    let cmd_sender = sender.command_sender().clone();
                    self.timer_handle = Some(tokio::spawn(async move {
                        for _ in 0..10 {
                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                            cmd_sender.emit(TouchpadCommandOutput::CountdownTick);
                        }
                        cmd_sender.emit(TouchpadCommandOutput::TimerElapsed);
                    }));
                } else {
                    self.confirmation_required = false;
                }

                AppConfig::update(|c| c.touchpad_aktiv = active);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = run_touchpad_command(active).await {
                                out.emit(TouchpadCommandOutput::Fehler(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadMsg::ConfirmClicked => {
                if let Some(handle) = self.timer_handle.take() {
                    handle.abort();
                }
                self.confirmation_required = false;
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: TouchpadCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            TouchpadCommandOutput::CountdownTick => {
                self.countdown = self.countdown.saturating_sub(1);
            }
            TouchpadCommandOutput::TimerElapsed => {
                if !self.confirmation_required {
                    return;
                }
                self.touchpad_active = true;
                self.confirmation_required = false;
                self.timer_handle = None;

                AppConfig::update(|c| c.touchpad_aktiv = true);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            if let Err(e) = run_touchpad_command(true).await {
                                out.emit(TouchpadCommandOutput::Fehler(e));
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            TouchpadCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

/// Enables or disables the touchpad using the appropriate desktop-environment API.
///
/// Uses `gsettings` on GNOME, `qdbus` on KDE, and returns an error on unsupported desktops.
async fn run_touchpad_command(active: bool) -> Result<(), String> {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_lowercase();

    if desktop.contains("gnome") {
        let value = if active { "enabled" } else { "disabled" };
        run_command_blocking(
            "gsettings",
            &[
                "set",
                "org.gnome.desktop.peripherals.touchpad",
                "send-events",
                value,
            ],
        )
        .await
    } else if desktop.contains("kde") {
        let method = if active {
            "org.kde.touchpad.enable"
        } else {
            "org.kde.touchpad.disable"
        };
        run_qdbus(vec![
            "org.kde.kglobalaccel".to_string(),
            "/modules/kded_touchpad".to_string(),
            method.to_string(),
        ])
        .await
        .map_err(|e| t!("error_touchpad_kde", error = e).to_string())
    } else {
        Err(t!("error_touchpad_unsupported_desktop", desktop = desktop).to_string())
    }
}
