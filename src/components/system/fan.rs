use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::dbus;
use crate::services::dbus::FanProfile;

/// State for the fan profile settings component.
pub struct FanModel {
    /// Whether the `asusd` daemon is reachable; disables all controls when `false`.
    asusd_available: bool,
    /// The currently active fan profile, used to suppress no-op toggle callbacks.
    current_profile: FanProfile,
    check_performance: gtk::CheckButton,
    check_balanced: gtk::CheckButton,
    check_quiet: gtk::CheckButton,
}

/// Input messages for the fan profile component.
#[derive(Debug)]
pub enum FanMsg {
    /// Switch to the given fan profile and persist the choice.
    ChangeProfile(FanProfile),
}

/// Async command results for the fan profile component.
#[derive(Debug)]
pub enum FanCommandOutput {
    /// Result of the initial `asusd` availability check.
    AsusdChecked(bool),
    /// Confirmation that the profile was successfully applied.
    ProfileSet(FanProfile),
    /// An error message to forward as a toast notification.
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for FanModel {
    type Init = ();
    type Input = FanMsg;
    type Output = String;
    type CommandOutput = FanCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("fan_group_title"),
            set_description: Some(&t!("fan_group_desc")),

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

            add = &adw::ActionRow {
                set_title: &t!("fan_performance_title"),
                set_subtitle: &t!("fan_performance_subtitle"),
                add_prefix = &model.check_performance.clone(),
                set_activatable_widget: Some(&model.check_performance),
                #[watch]
                set_sensitive: model.asusd_available,
            },

            add = &adw::ActionRow {
                set_title: &t!("fan_balanced_title"),
                set_subtitle: &t!("fan_balanced_subtitle"),
                add_prefix = &model.check_balanced.clone(),
                set_activatable_widget: Some(&model.check_balanced),
                #[watch]
                set_sensitive: model.asusd_available,
            },

            add = &adw::ActionRow {
                set_title: &t!("fan_quiet_title"),
                set_subtitle: &t!("fan_quiet_subtitle"),
                add_prefix = &model.check_quiet.clone(),
                set_activatable_widget: Some(&model.check_quiet),
                #[watch]
                set_sensitive: model.asusd_available,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let check_performance = gtk::CheckButton::new();
        let check_balanced = gtk::CheckButton::new();
        let check_quiet = gtk::CheckButton::new();

        check_balanced.set_group(Some(&check_performance));
        check_quiet.set_group(Some(&check_performance));

        let config = AppConfig::load();
        let saved_profile = FanProfile::from(config.fan_profil);
        match saved_profile {
            FanProfile::Performance => check_performance.set_active(true),
            FanProfile::Balanced => check_balanced.set_active(true),
            FanProfile::Quiet => check_quiet.set_active(true),
        }

        for (btn, profile) in [
            (&check_performance, FanProfile::Performance),
            (&check_balanced, FanProfile::Balanced),
            (&check_quiet, FanProfile::Quiet),
        ] {
            let sender = sender.clone();
            btn.connect_toggled(move |b| {
                if b.is_active() {
                    sender.input(FanMsg::ChangeProfile(profile));
                }
            });
        }

        let model = FanModel {
            asusd_available: false,
            current_profile: saved_profile,
            check_performance,
            check_balanced,
            check_quiet,
        };

        let widgets = view_output!();

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let available = dbus::check_asusd_available().await;
                    out.emit(FanCommandOutput::AsusdChecked(available));
                })
                .drop_on_shutdown()
        });

        sender.command(move |out, shutdown| {
            shutdown
                .register(async move {
                    match dbus::get_fan_profile().await {
                        Ok(current) if current == saved_profile => {
                            out.emit(FanCommandOutput::ProfileSet(current));
                        }
                        Ok(_) => match dbus::set_fan_profile(saved_profile).await {
                            Ok(p) => out.emit(FanCommandOutput::ProfileSet(p)),
                            Err(e) => out.emit(FanCommandOutput::Fehler(e)),
                        },
                        Err(e) => out.emit(FanCommandOutput::Fehler(e)),
                    }
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: FanMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FanMsg::ChangeProfile(profile) => {
                if profile == self.current_profile {
                    return;
                }
                self.current_profile = profile;
                AppConfig::update(|c| c.fan_profil = profile as u32);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_fan_profile(profile).await {
                                Ok(p) => out.emit(FanCommandOutput::ProfileSet(p)),
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
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            FanCommandOutput::AsusdChecked(available) => {
                self.asusd_available = available;
            }
            FanCommandOutput::ProfileSet(profile) => {
                tracing::info!(
                    "{}",
                    t!("fan_profile_set", profile = format!("{:?}", profile))
                );
            }
            FanCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}
