// Asus Hub - Unofficial Control Center for Asus Laptops
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

use crate::components::audio::SoundModesModel;
use crate::components::audio::VolumeModel;
use crate::components::display::ColorGamutModel;
use crate::components::display::OledCareModel;
use crate::components::display::OledDimmingModel;
use crate::components::display::TargetModeModel;
use crate::components::keyboard::AutoBeleuchtungModel;
use crate::components::keyboard::BacklightIdleModel;
use crate::components::keyboard::FnKeyModel;
use crate::components::keyboard::GesturenModel;
use crate::components::keyboard::TouchpadModel;
use crate::components::system::battery::BatteryModel;
use crate::components::system::fan::FanModel;
use crate::search::sorted_nav_items;
use crate::tray;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;
use std::rc::Rc;

#[derive(Debug)]
pub enum AppMsg {
    ShowWindow,
    QuitApp,
    Error(String),
    SetLanguage(String),
    ToggleAutostart(bool),
}

pub struct AppModel {
    start_hidden: bool,
    window: gtk4::glib::WeakRef<adw::ApplicationWindow>,
    toast_overlay: adw::ToastOverlay,
    _tray: ksni::Handle<tray::AsusTray>,
    battery: Controller<BatteryModel>,
    fan: Controller<FanModel>,
    oled_dimming: Controller<OledDimmingModel>,
    target_mode: Controller<TargetModeModel>,
    oled_care: Controller<OledCareModel>,
    color_gamut: Controller<ColorGamutModel>,
    fn_key: Controller<FnKeyModel>,
    gestures: Controller<GesturenModel>,
    touchpad: Controller<TouchpadModel>,
    auto_backlight: Controller<AutoBeleuchtungModel>,
    backlight_idle: Controller<BacklightIdleModel>,
    sound_modes: Controller<SoundModesModel>,
    volume_widget: Controller<VolumeModel>,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = bool;
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some(&t!("app_title")),
            set_default_size: (1200, 800),
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

    fn update(&mut self, message: AppMsg, _sender: ComponentSender<Self>) {
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
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_handler = |msg: String| AppMsg::Error(msg);
        let battery = BatteryModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let fan = FanModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let oled_dimming = OledDimmingModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let target_mode = TargetModeModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let oled_care = OledCareModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let color_gamut = ColorGamutModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let fn_key = FnKeyModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let gestures = GesturenModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let touchpad = TouchpadModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let auto_backlight = AutoBeleuchtungModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let backlight_idle = BacklightIdleModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let sound_modes = SoundModesModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);
        let volume_widget = VolumeModel::builder()
            .launch(())
            .forward(sender.input_sender(), error_handler);

        let tray_svc = ksni::TrayService::new(tray::AsusTray {
            app_sender: sender.input_sender().clone(),
        });
        let tray_handle = tray_svc.handle();
        tray_svc.spawn();

        let toast_overlay = adw::ToastOverlay::new();

        let model = AppModel {
            start_hidden: init,
            window: root.downgrade(),
            toast_overlay,
            _tray: tray_handle,
            battery,
            fan,
            oled_dimming,
            target_mode,
            oled_care,
            color_gamut,
            fn_key,
            gestures,
            touchpad,
            auto_backlight,
            backlight_idle,
            sound_modes,
            volume_widget,
        };

        let battery_widget = model.battery.widget();
        let fan_widget = model.fan.widget();
        let oled_dimming_widget = model.oled_dimming.widget();
        let target_mode_widget = model.target_mode.widget();
        let oled_care_widget = model.oled_care.widget();
        let color_gamut_widget = model.color_gamut.widget();
        let fn_key_widget = model.fn_key.widget();
        let gestures_widget = model.gestures.widget();
        let touchpad_widget = model.touchpad.widget();
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

        let keyboard_page = adw::PreferencesPage::new();
        keyboard_page.add(auto_backlight_widget);
        keyboard_page.add(backlight_idle_widget);
        keyboard_page.add(fn_key_widget);

        let asus_key_hint_group = adw::PreferencesGroup::new();
        asus_key_hint_group.set_title(&t!("asus_key_hint_group_title"));
        let asus_key_hint_row = adw::ActionRow::new();
        asus_key_hint_row.set_title(&t!("asus_key_hint_row_title"));
        asus_key_hint_row.set_subtitle(&t!("asus_key_hint_row_subtitle"));
        asus_key_hint_group.add(&asus_key_hint_row);
        keyboard_page.add(&asus_key_hint_group);

        let touchpad_page = adw::PreferencesPage::new();
        touchpad_page.add(touchpad_widget);
        touchpad_page.add(gestures_widget);

        let audio_page = adw::PreferencesPage::new();
        audio_page.add(volume_widget);
        audio_page.add(sound_modes_widget);

        let system_page = adw::PreferencesPage::new();
        system_page.add(battery_widget);
        system_page.add(fan_widget);

        let lang_group = adw::PreferencesGroup::new();
        lang_group.set_title(&t!("app_settings_title"));

        let lang_row = adw::ActionRow::new();
        lang_row.set_title(&t!("language_title"));

        const SUPPORTED_LANGS: &[(&str, &str)] = &[("English", "en"), ("Deutsch", "de")];

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

        let sender_clone = sender.clone();
        lang_dropdown.connect_selected_notify(move |dd| {
            let idx = dd.selected() as usize;
            if let Some(&(_, code)) = SUPPORTED_LANGS.get(idx) {
                sender_clone.input(AppMsg::SetLanguage(code.to_string()));
            }
        });

        lang_row.add_suffix(&lang_dropdown);
        lang_row.set_activatable_widget(Some(&lang_dropdown));
        lang_group.add(&lang_row);

        let autostart_row = adw::SwitchRow::new();
        autostart_row.set_title(&t!("autostart_title"));
        autostart_row.set_active(crate::autostart::is_enabled());
        let sender_at = sender.clone();
        autostart_row.connect_active_notify(move |row| {
            sender_at.input(AppMsg::ToggleAutostart(row.is_active()));
        });
        lang_group.add(&autostart_row);

        system_page.add(&lang_group);

        // Widget map for scroll-to-widget

        let widget_map = std::collections::HashMap::from([
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
            ("touchpad", touchpad_widget.clone().upcast::<gtk4::Widget>()),
            ("volume", volume_widget.clone().upcast::<gtk4::Widget>()),
            (
                "sound_modes",
                sound_modes_widget.clone().upcast::<gtk4::Widget>(),
            ),
            ("battery", battery_widget.clone().upcast::<gtk4::Widget>()),
            ("fan", fan_widget.clone().upcast::<gtk4::Widget>()),
            ("lang", lang_group.clone().upcast::<gtk4::Widget>()),
        ]);

        // ViewStack for the content area

        let content_stack = adw::ViewStack::new();
        content_stack.set_transition_duration(250);
        content_stack.set_enable_transitions(true);
        content_stack.add_named(&display_page, Some("display"));
        content_stack.add_named(&keyboard_page, Some("keyboard"));
        content_stack.add_named(&touchpad_page, Some("touchpad"));
        content_stack.add_named(&audio_page, Some("audio"));
        content_stack.add_named(&system_page, Some("system"));
        content_stack.set_visible_child_name("display");

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
        content_stack.add_named(&search_widgets.scroll, Some("search"));

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

            let svg_bytes = include_bytes!("../assets/img/github.svg");
            let github_btn = gtk4::Button::new();
            github_btn.add_css_class("flat");
            github_btn.set_tooltip_text(Some("GitHub"));
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

            let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
            spacer.set_hexpand(true);

            let version_label = gtk4::Label::new(Some(concat!("v", env!("CARGO_PKG_VERSION"))));
            version_label.add_css_class("dim-label");

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

        ComponentParts { model, widgets }
    }
}
