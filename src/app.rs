use crate::components::audio::SoundModesModel;
use crate::components::audio::VolumeModel;
use crate::components::display::FarbskalaModel;
use crate::components::display::OledCareModel;
use crate::components::display::OledDimmingModel;
use crate::components::display::ZielmodusModel;
use crate::components::keyboard::AutoBeleuchtungModel;
use crate::components::keyboard::FnKeyModel;
use crate::components::keyboard::GesturenModel;
use crate::components::keyboard::RuhezustandModel;
use crate::components::keyboard::TouchpadModel;
use crate::components::system::battery::BatteryModel;
use crate::components::system::fan::FanModel;
use crate::search::NAV_ITEMS;
use crate::tray;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AppMsg {
    ShowWindow,
    Fehler(String),
    SpracheSetzen(String),
}

pub struct AppModel {
    window: gtk4::glib::WeakRef<adw::ApplicationWindow>,
    toast_overlay: adw::ToastOverlay,
    _tray: ksni::Handle<tray::ZenbookTray>,
    battery: Controller<BatteryModel>,
    fan: Controller<FanModel>,
    oled_dimming: Controller<OledDimmingModel>,
    zielmodus: Controller<ZielmodusModel>,
    oled_care: Controller<OledCareModel>,
    farbskala: Controller<FarbskalaModel>,
    fn_key: Controller<FnKeyModel>,
    gesten: Controller<GesturenModel>,
    touchpad: Controller<TouchpadModel>,
    auto_beleuchtung: Controller<AutoBeleuchtungModel>,
    ruhezustand: Controller<RuhezustandModel>,
    sound_modes: Controller<SoundModesModel>,
    volume_widget: Controller<VolumeModel>,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::ApplicationWindow {
            set_title: Some(&t!("app_title")),
            set_default_size: (1200, 800),

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
            AppMsg::Fehler(text) => {
                eprintln!("{} {}", t!("error_prefix"), text);
                let toast = adw::Toast::new(&text);
                toast.set_timeout(5);
                self.toast_overlay.add_toast(toast);
            }
            AppMsg::SpracheSetzen(lang) => {
                crate::services::config::AppConfig::update(|c| {
                    c.language = lang.clone();
                });
                rust_i18n::set_locale(&lang);
                let toast = adw::Toast::new(&t!("lang_restart_toast"));
                toast.set_timeout(5);
                self.toast_overlay.add_toast(toast);
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let fehler = |msg: String| AppMsg::Fehler(msg);
        let battery = BatteryModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let fan = FanModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let oled_dimming = OledDimmingModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let zielmodus = ZielmodusModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let oled_care = OledCareModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let farbskala = FarbskalaModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let fn_key = FnKeyModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let gesten = GesturenModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let touchpad = TouchpadModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let auto_beleuchtung = AutoBeleuchtungModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let ruhezustand = RuhezustandModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let sound_modes = SoundModesModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);
        let volume_widget = VolumeModel::builder()
            .launch(())
            .forward(sender.input_sender(), fehler);

        let tray_svc = ksni::TrayService::new(tray::ZenbookTray {
            app_sender: sender.input_sender().clone(),
        });
        let tray_handle = tray_svc.handle();
        tray_svc.spawn();

        let toast_overlay = adw::ToastOverlay::new();

        let model = AppModel {
            window: root.downgrade(),
            toast_overlay,
            _tray: tray_handle,
            battery,
            fan,
            oled_dimming,
            zielmodus,
            oled_care,
            farbskala,
            fn_key,
            gesten,
            touchpad,
            auto_beleuchtung,
            ruhezustand,
            sound_modes,
            volume_widget,
        };

        let battery_widget = model.battery.widget();
        let fan_widget = model.fan.widget();
        let oled_dimming_widget = model.oled_dimming.widget();
        let zielmodus_widget = model.zielmodus.widget();
        let oled_care_widget = model.oled_care.widget();
        let farbskala_widget = model.farbskala.widget();
        let fn_key_widget = model.fn_key.widget();
        let gesten_widget = model.gesten.widget();
        let touchpad_widget = model.touchpad.widget();
        let auto_beleuchtung_widget = model.auto_beleuchtung.widget();
        let ruhezustand_widget = model.ruhezustand.widget();
        let sound_modes_widget = model.sound_modes.widget();
        let volume_widget = model.volume_widget.widget();

        // --- Content-Seiten ---

        let anzeige_page = adw::PreferencesPage::new();
        anzeige_page.add(oled_dimming_widget);
        anzeige_page.add(zielmodus_widget);
        anzeige_page.add(oled_care_widget);
        anzeige_page.add(farbskala_widget);

        let tastatur_page = adw::PreferencesPage::new();
        tastatur_page.add(auto_beleuchtung_widget);
        tastatur_page.add(ruhezustand_widget);
        tastatur_page.add(fn_key_widget);
        tastatur_page.add(touchpad_widget);
        tastatur_page.add(gesten_widget);

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
                sender_clone.input(AppMsg::SpracheSetzen(code.to_string()));
            }
        });

        lang_row.add_suffix(&lang_dropdown);
        lang_row.set_activatable_widget(Some(&lang_dropdown));
        lang_group.add(&lang_row);

        system_page.add(&lang_group);

        // --- Widget-Map für Scroll-to-Widget ---

        let widget_map = std::collections::HashMap::from([
            (
                "oled_dimming",
                oled_dimming_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "zielmodus",
                zielmodus_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "oled_care",
                oled_care_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "farbskala",
                farbskala_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "auto_beleuchtung",
                auto_beleuchtung_widget.clone().upcast::<gtk4::Widget>(),
            ),
            (
                "ruhezustand",
                ruhezustand_widget.clone().upcast::<gtk4::Widget>(),
            ),
            ("fn_key", fn_key_widget.clone().upcast::<gtk4::Widget>()),
            ("gesten", gesten_widget.clone().upcast::<gtk4::Widget>()),
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

        // --- ViewStack für den Content-Bereich ---

        let content_stack = adw::ViewStack::new();
        content_stack.set_transition_duration(250);
        content_stack.set_enable_transitions(true);
        content_stack.add_named(&anzeige_page, Some("display"));
        content_stack.add_named(&tastatur_page, Some("keyboard"));
        content_stack.add_named(&audio_page, Some("audio"));
        content_stack.add_named(&system_page, Some("system"));
        content_stack.set_visible_child_name("display");

        let content_header = adw::HeaderBar::new();
        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&content_header);
        content_toolbar.set_content(Some(&content_stack));
        let content_nav_page = adw::NavigationPage::new(&content_toolbar, &t!("tab_display"));

        // --- Sidebar ---

        let sidebar_list = gtk4::ListBox::new();
        sidebar_list.add_css_class("navigation-sidebar");
        sidebar_list.set_selection_mode(gtk4::SelectionMode::Single);

        for (icon_name, title_key, _page_name) in &NAV_ITEMS {
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

        // Seitenleisten-Auswahl → Stack-Seite + Header-Titel aktualisieren
        let stack_c = content_stack.clone();
        let nav_page_c = content_nav_page.clone();
        sidebar_list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                let idx = row.index() as usize;
                if let Some(&(_, title_key, page_name)) = NAV_ITEMS.get(idx) {
                    stack_c.set_visible_child_name(page_name);
                    nav_page_c.set_title(&t!(title_key));
                }
            }
        });

        if let Some(first_row) = sidebar_list.row_at_index(0) {
            sidebar_list.select_row(Some(&first_row));
        }

        // --- Suche ---

        let search_widgets =
            crate::search::setup(&content_stack, &content_nav_page, &sidebar_list, widget_map);
        content_stack.add_named(&search_widgets.scroll, Some("search"));

        let sidebar_header = adw::HeaderBar::new();
        sidebar_header.pack_end(&search_widgets.toggle);

        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(&sidebar_header);
        sidebar_toolbar.add_top_bar(&search_widgets.bar);
        sidebar_toolbar.set_content(Some(&sidebar_list));

        let sidebar_nav_page = adw::NavigationPage::new(&sidebar_toolbar, &t!("app_title"));

        // --- Widget-Baum erzeugen ---

        let widgets = view_output!();

        root.set_hide_on_close(true);

        ComponentParts { model, widgets }
    }
}
