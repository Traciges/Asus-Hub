// Ayuz - Unofficial Control Center for Asus Laptops
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

use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::components::audio::{SoundModesModel, SoundModesMsg, VolumeModel};
use crate::components::display::ColorGamutModel;
use crate::components::display::OledCareModel;
use crate::components::display::OledDimmingModel;
use crate::components::display::TargetModeModel;
use crate::components::home::{HomeModel, HomeOutput};
use crate::components::audio::volume::VolumeMsg;
use crate::components::display::color_gamut::ColorGamutMsg;
use crate::components::display::oled_care::OledCareMsg;
use crate::components::display::oled_dimming::OledDimmingMsg;
use crate::components::display::target_mode::TargetModeMsg;
use crate::components::keyboard::auto_backlight::AutoBacklightMsg;
use crate::components::keyboard::auto_backlight::AutoBacklightOutput;
use crate::components::keyboard::backlight_idle::BacklightIdleMsg;
use crate::components::keyboard::fn_key::FnKeyMsg;
use crate::components::touchpad::gestures::GesturesMsg;
use crate::components::touchpad::numberpad::NumberpadMsg;
use crate::components::touchpad::touchpad::TouchpadMsg;
use crate::components::touchpad::typing::TypingMsg;
use crate::components::system::apu_mem::ApuMemMsg;
use crate::components::system::battery::BatteryMsg;
use crate::components::system::fan::FanMsg;
use crate::components::system::gpu::GpuMsg;
use crate::services::dbus::FanProfile;
use crate::components::animatrix::{AnimatrixModel, AnimatrixMsg};
use crate::components::aura::AuraPageModel;
use crate::components::aura::AuraPageMsg;
use crate::components::keyboard::AutoBacklightModel;
use crate::components::keyboard::BacklightIdleModel;
use crate::components::keyboard::FnKeyModel;
use crate::components::touchpad::GesturesModel;
use crate::components::touchpad::NumberpadModel;
use crate::components::touchpad::TouchpadModel;
use crate::components::touchpad::TypingModel;
use crate::components::system::apu_mem::ApuMemModel;
use crate::components::system::battery::BatteryModel;
use crate::components::system::fan::FanModel;
use crate::components::system::gpu::GpuModel;
use crate::search::sorted_nav_items;
use crate::tray;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use std::rc::Rc;

/// Builds a Linux *abstract* (per-user-namespace) Unix socket address with the
/// given name. Returns None on non-Linux platforms where abstract sockets
/// are unsupported. Abstract addresses live in a separate namespace, do not
/// touch the filesystem, and are cleaned up automatically when the process
/// exits - exactly what we want for a tiny CLI -> running-GUI handshake.
pub(crate) fn abstract_socket_addr(name: &str) -> Option<std::os::unix::net::SocketAddr> {
    use std::os::linux::net::SocketAddrExt;
    std::os::unix::net::SocketAddr::from_abstract_name(name).ok()
}

macro_rules! launch_component {
    ($type:ty, $sender:expr) => {
        <$type>::builder()
            .launch(())
            .forward($sender.input_sender(), |msg: String| AppMsg::Error(msg))
    };
}

#[derive(Clone, Copy)]
enum AppPage {
    Home,
    Display,
    Keyboard,
    Aura,
    Animatrix,
    Touchpad,
    Audio,
    System,
    Search,
}

impl AppPage {
    fn as_str(self) -> &'static str {
        match self {
            AppPage::Home => "home",
            AppPage::Display => "display",
            AppPage::Keyboard => "keyboard",
            AppPage::Aura => "aura",
            AppPage::Animatrix => "animatrix",
            AppPage::Touchpad => "touchpad",
            AppPage::Audio => "audio",
            AppPage::System => "system",
            AppPage::Search => "search",
        }
    }
}

#[derive(Debug)]
pub enum AppMsg {
    ShowWindow,
    QuitApp,
    Error(String),
    SetLanguage(String),
    ToggleAutostart(bool),
    ActivateProfile(String),
    LegacyMigrationAccepted,
    LegacyMigrationDeclined,
    TriggerManualMigration,
    /// Ambient keyboard-backlight settings changed; rebuild the idle-timeout resume command.
    AmbientBacklightChanged,
}

pub struct AppModel {
    start_hidden: bool,
    window: gtk4::glib::WeakRef<adw::ApplicationWindow>,
    toast_overlay: adw::ToastOverlay,
    _tray: ksni::Handle<tray::AyuzTray>,
    home: Controller<HomeModel>,
    apu_mem: Controller<ApuMemModel>,
    battery: Controller<BatteryModel>,
    fan: Controller<FanModel>,
    gpu: Controller<GpuModel>,
    oled_dimming: Controller<OledDimmingModel>,
    target_mode: Controller<TargetModeModel>,
    oled_care: Controller<OledCareModel>,
    color_gamut: Controller<ColorGamutModel>,
    aura: Controller<AuraPageModel>,
    animatrix: Controller<AnimatrixModel>,
    fn_key: Controller<FnKeyModel>,
    gestures: Controller<GesturesModel>,
    numberpad: Controller<NumberpadModel>,
    touchpad: Controller<TouchpadModel>,
    typing: Controller<TypingModel>,
    auto_backlight: Controller<AutoBacklightModel>,
    backlight_idle: Controller<BacklightIdleModel>,
    sound_modes: Controller<SoundModesModel>,
    volume_widget: Controller<VolumeModel>,
}

impl AppModel {
    /// Broadcasts the active profile's settings to every sub-component.
    ///
    /// Each `LoadProfile` message is the canonical entry point that tells a
    /// component to overwrite its in-memory state with the values from
    /// `profile`. Grouped per page so the call sites stay readable.
    fn distribute_profile(&self, p: &crate::services::config::Profile) {
        // Display
        self.fan.sender().emit(FanMsg::LoadProfile(FanProfile::from(p.fan_profile)));
        self.oled_dimming.sender().emit(OledDimmingMsg::LoadProfile(p.oled_dc_dimming));
        self.target_mode.sender().emit(TargetModeMsg::LoadProfile(p.target_mode_active));
        self.color_gamut.sender().emit(ColorGamutMsg::LoadProfile(p.color_profile_index));
        self.oled_care.sender().emit(OledCareMsg::LoadProfile {
            pixel_refresh: p.oled_care_pixel_refresh,
            panel_autohide: p.oled_care_panel_autohide,
            transparency: p.oled_care_transparency,
        });

        // Audio
        self.sound_modes.sender().emit(SoundModesMsg::LoadProfile(p.audio_profile));
        self.volume_widget.sender().emit(VolumeMsg::LoadProfile(p.volume));

        // Keyboard & input
        self.auto_backlight.sender().emit(AutoBacklightMsg::LoadProfile {
            brighten: p.kbd_brighten_active,
            dim: p.kbd_dim_active,
            brighten_threshold: p.kbd_brighten_threshold,
            dim_threshold: p.kbd_dim_threshold,
        });
        self.backlight_idle.sender().emit(BacklightIdleMsg::LoadProfile {
            mode: p.kbd_timeout_mode,
            ac_index: p.kbd_timeout_battery_ac_index,
            battery_index: p.kbd_timeout_battery_only_index,
        });
        self.aura.sender().emit(AuraPageMsg::LoadProfile {
            mode: p.aura_mode,
            zone: p.aura_zone,
            brightness: p.aura_brightness,
            colour_r: p.aura_colour_r,
            colour_g: p.aura_colour_g,
            colour_b: p.aura_colour_b,
            colour2_r: p.aura_colour2_r,
            colour2_g: p.aura_colour2_g,
            colour2_b: p.aura_colour2_b,
            speed: p.aura_speed.clone(),
            direction: p.aura_direction.clone(),
        });
        self.animatrix.sender().emit(AnimatrixMsg::LoadProfile {
            enable_display: p.animatrix_enable_display,
            brightness: p.animatrix_brightness,
            builtins_enabled: p.animatrix_builtins_enabled,
            boot_anim: p.animatrix_boot_anim.clone(),
            awake_anim: p.animatrix_awake_anim.clone(),
            sleep_anim: p.animatrix_sleep_anim.clone(),
            shutdown_anim: p.animatrix_shutdown_anim.clone(),
            off_unplugged: p.animatrix_off_when_unplugged,
            off_suspended: p.animatrix_off_when_suspended,
            off_lid_closed: p.animatrix_off_when_lid_closed,
        });
        self.touchpad.sender().emit(TouchpadMsg::LoadProfile(p.touchpad_active));
        self.gestures.sender().emit(GesturesMsg::LoadProfile(p.input_gestures_active));
        self.numberpad.sender().emit(NumberpadMsg::LoadProfile(p.numberpad_active));
        self.typing.sender().emit(TypingMsg::LoadProfile {
            disable_gestures: p.typing_disable_gestures,
            disable_touchpad: p.typing_disable_touchpad,
            delay_ms: p.typing_reactivation_delay_ms,
        });
        self.fn_key.sender().emit(FnKeyMsg::LoadProfile(p.input_fn_key_locked));

        // System
        self.battery.sender().emit(BatteryMsg::LoadProfile(p.battery_deep_sleep_active));
        self.gpu.sender().emit(GpuMsg::LoadProfile(p.gpu_mode));
        self.apu_mem.sender().emit(ApuMemMsg::LoadProfile(p.apu_mem));
    }
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = bool;
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some(&t!("app_title")),
            set_default_size: (1300, 800),
            set_visible: !model.start_hidden,

            #[wrap(Some)]
            set_content = &model.toast_overlay.clone() -> adw::ToastOverlay {
                #[wrap(Some)]
                set_child = &adw::NavigationSplitView {
                    set_sidebar: Some(&sidebar_nav_page),
                    set_content: Some(&content_nav_page),
                    set_collapsed: false,
                },
            }
        }
    }

    fn update(&mut self, message: AppMsg, sender: ComponentSender<Self>) {
        match message {
            AppMsg::ShowWindow => {
                if let Some(window) = self.window.upgrade() {
                    window.set_visible(true);
                    window.present();
                }
            }
            AppMsg::QuitApp => {
                relm4::main_application().quit();
            }
            AppMsg::Error(text) => {
                tracing::warn!("{} {}", t!("error_prefix"), text);
                let toast = adw::Toast::new(&text);
                toast.set_timeout(5);
                self.toast_overlay.add_toast(toast);
            }
            AppMsg::SetLanguage(lang) => {
                crate::services::config::AppConfig::update(|c| {
                    c.language = lang.clone();
                });
                rust_i18n::set_locale(&lang);
                let toast = adw::Toast::new(&t!("lang_restart_toast"));
                toast.set_timeout(5);
                self.toast_overlay.add_toast(toast);
            }
            AppMsg::ToggleAutostart(state) => {
                crate::autostart::set_enabled(state);
            }
            AppMsg::ActivateProfile(id) => {
                crate::services::config::AppConfig::update(|c| c.active_profile_id = id.clone());
                let config = crate::services::config::AppConfig::load();
                self.distribute_profile(config.active_profile());
            }
            AppMsg::LegacyMigrationAccepted => {
                match crate::services::migration::perform_migration() {
                    Ok(()) => {}
                    Err(e) => {
                        let toast = adw::Toast::new(&e);
                        toast.set_timeout(5);
                        self.toast_overlay.add_toast(toast);
                    }
                }
            }
            AppMsg::LegacyMigrationDeclined => {
                crate::services::config::AppConfig::update(|c| {
                    c.skip_legacy_migration = true;
                });
            }
            AppMsg::AmbientBacklightChanged => {
                self.backlight_idle.sender().emit(BacklightIdleMsg::AmbientChanged);
            }
            AppMsg::TriggerManualMigration => {
                if !crate::services::migration::legacy_dir_exists() {
                    return;
                }
                let migration_sender = sender.input_sender().clone();
                let window_weak = self.window.clone();
                gtk4::glib::spawn_future_local(async move {
                    let dialog = adw::AlertDialog::builder()
                        .heading(t!("migration_dialog_heading").as_ref())
                        .body(t!("migration_dialog_body").as_ref())
                        .build();
                    dialog.add_response("no", &t!("migration_dialog_no"));
                    dialog.add_response("yes", &t!("migration_dialog_yes"));
                    dialog.set_default_response(Some("yes"));
                    dialog.set_close_response("no");
                    let response = dialog
                        .choose_future(window_weak.upgrade().as_ref())
                        .await;
                    if &*response == "yes" {
                        migration_sender.emit(AppMsg::LegacyMigrationAccepted);
                    } else {
                        migration_sender.emit(AppMsg::LegacyMigrationDeclined);
                    }
                });
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let home = HomeModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                HomeOutput::Error(e) => AppMsg::Error(e),
                HomeOutput::ActivateProfile(id) => AppMsg::ActivateProfile(id),
            });
        let apu_mem = launch_component!(ApuMemModel, sender);
        let battery = launch_component!(BatteryModel, sender);
        let fan = launch_component!(FanModel, sender);
        let gpu = launch_component!(GpuModel, sender);
        let oled_dimming = launch_component!(OledDimmingModel, sender);
        let target_mode = launch_component!(TargetModeModel, sender);
        let oled_care = launch_component!(OledCareModel, sender);
        let color_gamut = launch_component!(ColorGamutModel, sender);
        let aura = launch_component!(AuraPageModel, sender);
        let animatrix = launch_component!(AnimatrixModel, sender);
        let fn_key = launch_component!(FnKeyModel, sender);
        let gestures = launch_component!(GesturesModel, sender);
        let numberpad = launch_component!(NumberpadModel, sender);
        let touchpad = launch_component!(TouchpadModel, sender);
        let typing = launch_component!(TypingModel, sender);
        let auto_backlight = AutoBacklightModel::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                AutoBacklightOutput::Error(e) => AppMsg::Error(e),
                AutoBacklightOutput::AmbientChanged => AppMsg::AmbientBacklightChanged,
            });
        let backlight_idle = launch_component!(BacklightIdleModel, sender);
        let sound_modes = launch_component!(SoundModesModel, sender);
        let volume_widget = launch_component!(VolumeModel, sender);

        let tray_svc = ksni::TrayService::new(tray::AyuzTray {
            app_sender: sender.input_sender().clone(),
        });
        let tray_handle = tray_svc.handle();
        tray_svc.spawn();

        let fan_sender = fan.sender().clone();
        let initial_fan_hotkey_enabled =
            crate::services::config::AppConfig::load().fan_hotkey_enabled;
        let (fan_hotkey_tx, fan_hotkey_rx) =
            tokio::sync::watch::channel(initial_fan_hotkey_enabled);
        tokio::spawn(crate::services::fan_hotkey::run(fan_sender, fan_hotkey_rx));

        // Abstract Unix socket listener for `ayuz --toggle-numberpad`. The
        // CLI short-circuit in main.rs connects to "\0ayuz-numberpad" and
        // writes one byte; each byte received here flips the NumberPad
        // Active/Idle state at runtime without re-launching the GUI.
        let numberpad_sender = numberpad.sender().clone();
        tokio::spawn(async move {
            use tokio::io::AsyncReadExt;
            use tokio::net::UnixListener;
            let addr = match abstract_socket_addr("ayuz-numberpad") {
                Some(a) => a,
                None => return,
            };
            let listener = match std::os::unix::net::UnixListener::bind_addr(&addr)
                .and_then(|s| { s.set_nonblocking(true)?; Ok(s) })
                .and_then(UnixListener::from_std)
            {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!("NumberPad IPC: failed to bind socket: {}", e);
                    return;
                }
            };
            loop {
                match listener.accept().await {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 1];
                        if stream.read_exact(&mut buf).await.is_ok() {
                            numberpad_sender.emit(NumberpadMsg::ToggleActive);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("NumberPad IPC: accept failed: {}", e);
                    }
                }
            }
        });

        let toast_overlay = adw::ToastOverlay::new();

        let model = AppModel {
            start_hidden: init,
            window: root.downgrade(),
            toast_overlay,
            _tray: tray_handle,
            home,
            apu_mem,
            battery,
            fan,
            gpu,
            oled_dimming,
            target_mode,
            oled_care,
            color_gamut,
            aura,
            animatrix,
            fn_key,
            gestures,
            numberpad,
            touchpad,
            typing,
            auto_backlight,
            backlight_idle,
            sound_modes,
            volume_widget,
        };

        let home_widget = model.home.widget();
        let apu_mem_widget = model.apu_mem.widget();
        let battery_widget = model.battery.widget();
        let fan_widget = model.fan.widget();
        let gpu_widget = model.gpu.widget();
        let oled_dimming_widget = model.oled_dimming.widget();
        let target_mode_widget = model.target_mode.widget();
        let oled_care_widget = model.oled_care.widget();
        let color_gamut_widget = model.color_gamut.widget();
        let aura_widget = model.aura.widget();
        let animatrix_widget = model.animatrix.widget();
        let fn_key_widget = model.fn_key.widget();
        let gestures_widget = model.gestures.widget();
        let numberpad_widget = model.numberpad.widget();
        let touchpad_widget = model.touchpad.widget();
        let typing_widget = model.typing.widget();
        let auto_backlight_widget = model.auto_backlight.widget();
        let backlight_idle_widget = model.backlight_idle.widget();
        let sound_modes_widget = model.sound_modes.widget();
        let volume_widget = model.volume_widget.widget();

        // Content pages

        let display_page = adw::PreferencesPage::new();
        display_page.add(oled_dimming_widget);
        display_page.add(target_mode_widget);
        display_page.add(oled_care_widget);
        display_page.add(color_gamut_widget);

        // Aura page hosts a Box of dynamic per-device PreferencesGroups
        // (built by AuraPageModel). PreferencesPage can't host non-PreferencesGroup
        // children directly, so we replicate its clamping/scrolling layout manually.
        let aura_inner = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(24)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(12)
            .margin_end(12)
            .build();
        aura_inner.append(aura_widget);
        let aura_clamp = adw::Clamp::builder()
            .maximum_size(600)
            .tightening_threshold(400)
            .child(&aura_inner)
            .build();
        let aura_page = gtk4::ScrolledWindow::builder()
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vexpand(true)
            .child(&aura_clamp)
            .build();

        let animatrix_page = adw::PreferencesPage::new();
        animatrix_page.add(animatrix_widget);

        let keyboard_page = adw::PreferencesPage::new();
        keyboard_page.add(auto_backlight_widget);
        keyboard_page.add(backlight_idle_widget);
        keyboard_page.add(fn_key_widget);

        let asus_key_hint_group = build_asus_key_hint_group(fan_hotkey_tx);
        keyboard_page.add(&asus_key_hint_group);

        let touchpad_page = adw::PreferencesPage::new();
        touchpad_page.add(touchpad_widget);
        touchpad_page.add(numberpad_widget);
        touchpad_page.add(gestures_widget);
        touchpad_page.add(typing_widget);

        let audio_page = adw::PreferencesPage::new();
        audio_page.add(volume_widget);
        audio_page.add(sound_modes_widget);

        let system_page = adw::PreferencesPage::new();
        system_page.add(battery_widget);
        system_page.add(fan_widget);
        system_page.add(gpu_widget);
        system_page.add(apu_mem_widget);

        let lang_group = build_language_and_autostart_group(&sender);
        system_page.add(&lang_group);

        let legacy_group = build_legacy_migration_group(&sender);
        system_page.add(&legacy_group);

        // Widget map for scroll-to-widget

        let widget_map = std::collections::HashMap::from([
            ("home_info", home_widget.clone().upcast::<gtk4::Widget>()),
            (
                "oled_dimming",
                oled_dimming_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "target_mode",
                target_mode_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "oled_care",
                oled_care_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "color_gamut",
                color_gamut_widget.clone().upcast::<gtk4::Widget>(),
            ),
            ("aura", aura_widget.clone().upcast::<gtk4::Widget>()),
            ("animatrix", animatrix_widget.clone().upcast::<gtk4::Widget>()),
            (
                "auto_backlight",
                auto_backlight_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "backlight_idle",
                backlight_idle_widget.clone().upcast::<gtk4::Widget>(),
            ),
            ("fn_key", fn_key_widget.clone().upcast::<gtk4::Widget>()),
            ("gestures", gestures_widget.clone().upcast::<gtk4::Widget>()),
            ("numberpad", numberpad_widget.clone().upcast::<gtk4::Widget>()),
            ("touchpad", touchpad_widget.clone().upcast::<gtk4::Widget>()),
            ("typing", typing_widget.clone().upcast::<gtk4::Widget>()),
            ("volume", volume_widget.clone().upcast::<gtk4::Widget>()),
            (
                "sound_modes",
                sound_modes_widget.clone().upcast::<gtk4::Widget>(),
            ),
            ("apu_mem", apu_mem_widget.clone().upcast::<gtk4::Widget>()),
            ("battery", battery_widget.clone().upcast::<gtk4::Widget>()),
            ("fan", fan_widget.clone().upcast::<gtk4::Widget>()),
            (
                "asus_key_hint",
                asus_key_hint_group.clone().upcast::<gtk4::Widget>(),
            ),
            ("gpu", gpu_widget.clone().upcast::<gtk4::Widget>()),
            ("lang", lang_group.clone().upcast::<gtk4::Widget>()),
        ]);

        // ViewStack for the content area

        let home_scroll = gtk4::ScrolledWindow::new();
        home_scroll.set_child(Some(home_widget));
        home_scroll.set_vexpand(true);

        let content_stack = adw::ViewStack::new();
        content_stack.set_transition_duration(250);
        content_stack.set_enable_transitions(true);
        content_stack.add_named(&home_scroll, Some(AppPage::Home.as_str()));
        content_stack.add_named(&display_page, Some(AppPage::Display.as_str()));
        content_stack.add_named(&keyboard_page, Some(AppPage::Keyboard.as_str()));
        content_stack.add_named(&aura_page, Some(AppPage::Aura.as_str()));
        content_stack.add_named(&animatrix_page, Some(AppPage::Animatrix.as_str()));
        content_stack.add_named(&touchpad_page, Some(AppPage::Touchpad.as_str()));
        content_stack.add_named(&audio_page, Some(AppPage::Audio.as_str()));
        content_stack.add_named(&system_page, Some(AppPage::System.as_str()));
        content_stack.set_visible_child_name(AppPage::Home.as_str());

        let content_header = adw::HeaderBar::new();
        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&content_header);
        content_toolbar.set_content(Some(&content_stack));
        let content_nav_page = adw::NavigationPage::new(&content_toolbar, &t!("tab_display"));

        // Sidebar

        let sidebar_list = gtk4::ListBox::new();
        sidebar_list.add_css_class("navigation-sidebar");
        sidebar_list.set_selection_mode(gtk4::SelectionMode::Single);

        let sorted_nav = Rc::new(sorted_nav_items());

        sidebar_list.set_header_func(|row, _before| {
            if row.index() == 1 {
                let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                sep.set_margin_top(4);
                sep.set_margin_bottom(4);
                sep.add_css_class("nav-separator");
                row.set_header(Some(&sep));
            } else {
                row.set_header(None::<&gtk4::Widget>);
            }
        });

        for (icon_name, title_key, _page_name) in sorted_nav.iter() {
            let row = gtk4::ListBoxRow::new();
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
            hbox.set_margin_top(10);
            hbox.set_margin_bottom(10);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);
            let icon = gtk4::Image::from_icon_name(icon_name);
            icon.set_pixel_size(16);
            let label = gtk4::Label::new(Some(t!(*title_key).as_ref()));
            label.set_halign(gtk4::Align::Start);
            hbox.append(&icon);
            hbox.append(&label);
            row.set_child(Some(&hbox));
            sidebar_list.append(&row);
        }

        let stack_c = content_stack.clone();
        let nav_page_c = content_nav_page.clone();
        let sorted_nav_c = sorted_nav.clone();
        sidebar_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let idx = row.index() as usize;
                if let Some(&(_, title_key, page_name)) = sorted_nav_c.get(idx) {
                    stack_c.set_visible_child_name(page_name);
                    nav_page_c.set_title(&t!(title_key));
                }
            }
        });

        if let Some(first_row) = sidebar_list.row_at_index(0) {
            sidebar_list.select_row(Some(&first_row));
        }

        // Search

        let search_widgets = crate::search::setup(
            (*sorted_nav).clone(),
            &content_stack,
            &content_nav_page,
            &sidebar_list,
            widget_map,
        );
        content_stack.add_named(&search_widgets.scroll, Some(AppPage::Search.as_str()));

        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.pack_end(&search_widgets.toggle);

        {
            let title_label = gtk4::Label::new(Some(&t!("app_title")));
            title_label.add_css_class("title");
            sidebar_header.set_title_widget(Some(&title_label));
        }

        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&sidebar_header);
        sidebar_toolbar.add_top_bar(&search_widgets.bar);
        sidebar_toolbar.set_content(Some(&sidebar_list));

        // Bottom bar: GitHub + "Made by Guido" + version
        {
            let bottom_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            bottom_box.set_margin_top(6);
            bottom_box.set_margin_bottom(6);
            bottom_box.set_margin_start(10);
            bottom_box.set_margin_end(10);

            let github_btn = gtk4::Button::new();
            github_btn.add_css_class("flat");
            github_btn.set_tooltip_text(Some("GitHub"));
            let svg_bytes = include_bytes!("../assets/img/github.svg");
            let glib_bytes = gtk4::glib::Bytes::from_static(svg_bytes);
            if let Ok(texture) = gtk4::gdk::Texture::from_bytes(&glib_bytes) {
                let gh_icon = gtk4::Image::from_paintable(Some(&texture));
                gh_icon.set_pixel_size(16);
                github_btn.set_child(Some(&gh_icon));
            }
            github_btn.connect_clicked(|_| {
                let _ = Command::new("xdg-open")
                    .arg("https://github.com/Traciges")
                    .process_group(0)
                    .spawn();
            });

            let made_by_label = gtk4::Label::new(Some("Made by Guido"));
            made_by_label.add_css_class("dim-label");
            made_by_label.set_margin_start(6);
            made_by_label.set_valign(gtk4::Align::Center);

            let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            spacer.set_hexpand(true);

            let version_label = gtk4::Label::new(Some(concat!("v", env!("CARGO_PKG_VERSION"))));
            version_label.add_css_class("dim-label");
            version_label.set_valign(gtk4::Align::Center);

            bottom_box.append(&github_btn);
            bottom_box.append(&made_by_label);
            bottom_box.append(&spacer);
            bottom_box.append(&version_label);

            sidebar_toolbar.add_bottom_bar(&bottom_box);
        }

        let sidebar_nav_page = adw::NavigationPage::new(&sidebar_toolbar, &t!("app_title"));

        // Build widget tree

        let widgets = view_output!();

        root.set_hide_on_close(true);

        // Relm4 calls window.present() internally after init() completes, which
        // overrides set_visible: false and forces the window to show. We schedule
        // a hide in the same GTK frame via idle_add_local_once so the window is
        // invisible again before a single pixel is drawn to the screen.
        if init {
            let win = root.clone();
            gtk4::glib::idle_add_local_once(move || {
                win.set_visible(false);
            });
        }

        if crate::services::migration::should_prompt() {
            let migration_sender = sender.input_sender().clone();
            let window_weak = root.downgrade();
            gtk4::glib::spawn_future_local(async move {
                let dialog = adw::AlertDialog::builder()
                    .heading(t!("migration_dialog_heading").as_ref())
                    .body(t!("migration_dialog_body").as_ref())
                    .build();
                dialog.add_response("no", &t!("migration_dialog_no"));
                dialog.add_response("yes", &t!("migration_dialog_yes"));
                dialog.set_default_response(Some("yes"));
                dialog.set_close_response("no");
                let response = dialog
                    .choose_future(window_weak.upgrade().as_ref())
                    .await;
                if &*response == "yes" {
                    migration_sender.emit(AppMsg::LegacyMigrationAccepted);
                } else {
                    migration_sender.emit(AppMsg::LegacyMigrationDeclined);
                }
            });
        }

        ComponentParts { model, widgets }
    }
}

// ── PreferencesGroup builders ────────────────────────────────────────────────
//
// Each builder constructs one `adw::PreferencesGroup` together with the rows
// inside it and wires up the relevant signals. They are called from `init()`
// so that the top-level component code stays focused on page composition.

/// Hint group hosted on the keyboard page. Surfaces the ASUS hardware-key
/// shortcut help row plus two app-level toggles (fan-profile hotkey + OSD).
///
/// `fan_hotkey_tx` is the `watch::Sender` that `services::fan_hotkey::run`
/// subscribes to — toggling the row pushes the new state to the running task.
fn build_asus_key_hint_group(
    fan_hotkey_tx: tokio::sync::watch::Sender<bool>,
) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    group.set_title(&t!("asus_key_hint_group_title"));

    let hint_row = adw::ActionRow::new();
    hint_row.set_title(&t!("asus_key_hint_row_title"));
    hint_row.set_subtitle(&t!("asus_key_hint_row_subtitle"));
    group.add(&hint_row);

    let cfg = crate::services::config::AppConfig::load();

    let fan_hotkey_row = adw::SwitchRow::new();
    fan_hotkey_row.set_title(&t!("fan_hotkey_enable_title"));
    fan_hotkey_row.set_subtitle(&t!("fan_hotkey_enable_subtitle"));
    fan_hotkey_row.set_active(cfg.fan_hotkey_enabled);
    fan_hotkey_row.connect_active_notify(move |row| {
        let active = row.is_active();
        crate::services::config::AppConfig::update(|c| c.fan_hotkey_enabled = active);
        let _ = fan_hotkey_tx.send(active);
    });
    group.add(&fan_hotkey_row);

    let fan_osd_row = adw::SwitchRow::new();
    fan_osd_row.set_title(&t!("fan_osd_enabled_title"));
    fan_osd_row.set_subtitle(&t!("fan_osd_enabled_subtitle"));
    fan_osd_row.set_active(cfg.fan_osd_enabled);
    fan_osd_row.connect_active_notify(|row| {
        let active = row.is_active();
        crate::services::config::AppConfig::update(|c| c.fan_osd_enabled = active);
    });
    group.add(&fan_osd_row);

    group
}

/// "App settings" group on the system page: language dropdown + autostart switch.
fn build_language_and_autostart_group(
    sender: &ComponentSender<AppModel>,
) -> adw::PreferencesGroup {
    const SUPPORTED_LANGS: &[(&str, &str)] = &[
        ("English", "en"),
        ("Deutsch", "de"),
        ("Português Brasileiro", "pt-br"),
    ];

    let group = adw::PreferencesGroup::new();
    group.set_title(&t!("app_settings_title"));

    let lang_row = adw::ActionRow::new();
    lang_row.set_title(&t!("language_title"));

    let display_names: Vec<&str> = SUPPORTED_LANGS.iter().map(|(name, _)| *name).collect();
    let lang_dropdown = gtk4::DropDown::from_strings(&display_names);
    lang_dropdown.set_valign(gtk4::Align::Center);

    let current_lang = crate::services::config::AppConfig::load().language;
    if let Some(idx) = SUPPORTED_LANGS
        .iter()
        .position(|(_, code)| *code == current_lang)
    {
        lang_dropdown.set_selected(idx as u32);
    }

    let sender_lang = sender.clone();
    lang_dropdown.connect_selected_notify(move |dd| {
        let idx = dd.selected() as usize;
        if let Some(&(_, code)) = SUPPORTED_LANGS.get(idx) {
            sender_lang.input(AppMsg::SetLanguage(code.to_string()));
        }
    });

    lang_row.add_suffix(&lang_dropdown);
    lang_row.set_activatable_widget(Some(&lang_dropdown));
    group.add(&lang_row);

    let autostart_row = adw::SwitchRow::new();
    autostart_row.set_title(&t!("autostart_title"));
    autostart_row.set_active(crate::autostart::is_enabled());
    let sender_at = sender.clone();
    autostart_row.connect_active_notify(move |row| {
        sender_at.input(AppMsg::ToggleAutostart(row.is_active()));
    });
    group.add(&autostart_row);

    group
}

/// One-shot migration helper shown on the system page when an
/// `ayuz-old` config directory exists from a pre-1.0 install.
fn build_legacy_migration_group(sender: &ComponentSender<AppModel>) -> adw::PreferencesGroup {
    let available = crate::services::migration::legacy_dir_exists();

    let group = adw::PreferencesGroup::new();
    group.set_title(&t!("legacy_migration_group_title"));
    group.set_description(Some(&t!("legacy_migration_group_desc")));

    let row = adw::ActionRow::new();
    row.set_title(&t!("legacy_migration_row_title"));
    row.set_subtitle(&t!(if available {
        "legacy_migration_row_subtitle_available"
    } else {
        "legacy_migration_row_subtitle_unavailable"
    }));

    let btn = gtk4::Button::with_label(&t!("legacy_migration_button"));
    btn.add_css_class("suggested-action");
    btn.set_valign(gtk4::Align::Center);
    btn.set_sensitive(available);

    let sender_clone = sender.clone();
    btn.connect_clicked(move |_| {
        sender_clone.input(AppMsg::TriggerManualMigration);
    });

    row.add_suffix(&btn);
    group.add(&row);
    group
}
