use relm4::adw;
use relm4::adw::prelude::*;
use rust_i18n::t;
use std::collections::HashMap;
use std::rc::Rc;

// (icon, i18n-key, stack-name)
pub const NAV_ITEMS: [(&str, &str, &str); 4] = [
    ("monitor-symbolic", "tab_display", "display"),
    ("input-keyboard-symbolic", "tab_keyboard", "keyboard"),
    ("audio-headset-symbolic", "tab_audio", "audio"),
    ("preferences-system-symbolic", "tab_system", "system"),
];

struct SearchItem {
    title_key: &'static str,
    page_icon: &'static str,
    page_title_key: &'static str,
    page_idx: i32,
    component_key: &'static str,
}

static SEARCH_INDEX: &[SearchItem] = &[
    // Anzeige
    SearchItem {
        title_key: "oled_dimming_group_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "oled_dimming",
    },
    SearchItem {
        title_key: "oled_dimming_slider_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "oled_dimming",
    },
    SearchItem {
        title_key: "zielmodus_group_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "zielmodus",
    },
    SearchItem {
        title_key: "zielmodus_switch_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "zielmodus",
    },
    SearchItem {
        title_key: "oled_care_pixel_refresh_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "oled_care",
    },
    SearchItem {
        title_key: "oled_care_panel_autohide_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "oled_care",
    },
    SearchItem {
        title_key: "oled_care_transparency_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "oled_care",
    },
    SearchItem {
        title_key: "farbskala_title",
        page_icon: "monitor-symbolic",
        page_title_key: "tab_display",
        page_idx: 0,
        component_key: "farbskala",
    },
    // Maus & Tastatur
    SearchItem {
        title_key: "backlight_auto_on_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "auto_beleuchtung",
    },
    SearchItem {
        title_key: "backlight_auto_off_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "auto_beleuchtung",
    },
    SearchItem {
        title_key: "backlight_threshold_on_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "auto_beleuchtung",
    },
    SearchItem {
        title_key: "backlight_threshold_off_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "auto_beleuchtung",
    },
    SearchItem {
        title_key: "sleep_group_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "ruhezustand",
    },
    SearchItem {
        title_key: "fn_key_group_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "fn_key",
    },
    SearchItem {
        title_key: "gestures_group_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "gesten",
    },
    SearchItem {
        title_key: "touchpad_group_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_idx: 1,
        component_key: "touchpad",
    },
    // Audio
    SearchItem {
        title_key: "volume_booster_title",
        page_icon: "audio-headset-symbolic",
        page_title_key: "tab_audio",
        page_idx: 2,
        component_key: "volume",
    },
    SearchItem {
        title_key: "audio_profiles_title",
        page_icon: "audio-headset-symbolic",
        page_title_key: "tab_audio",
        page_idx: 2,
        component_key: "sound_modes",
    },
    // System
    SearchItem {
        title_key: "battery_maintenance_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_idx: 3,
        component_key: "battery",
    },
    SearchItem {
        title_key: "battery_full_charge_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_idx: 3,
        component_key: "battery",
    },
    SearchItem {
        title_key: "battery_deep_sleep_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_idx: 3,
        component_key: "battery",
    },
    SearchItem {
        title_key: "fan_group_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_idx: 3,
        component_key: "fan",
    },
    SearchItem {
        title_key: "language_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_idx: 3,
        component_key: "lang",
    },
];

pub struct SearchWidgets {
    pub scroll: gtk4::ScrolledWindow,
    pub bar: gtk4::SearchBar,
    pub toggle: gtk4::ToggleButton,
}

fn scroll_to_widget(widget: &gtk4::Widget) {
    let mut current = widget.parent();
    let sw = loop {
        match current {
            None => break None,
            Some(ref w) => {
                if let Ok(sw) = w.clone().downcast::<gtk4::ScrolledWindow>() {
                    break Some(sw);
                }
                current = w.parent();
            }
        }
    };
    let sw = match sw {
        Some(sw) => sw,
        None => return,
    };
    if let Some(pt) = widget.compute_point(&sw, &gtk4::graphene::Point::new(0.0, 0.0)) {
        let adj = sw.vadjustment();
        let abs_y = pt.y() as f64 + adj.value();
        let target_y = abs_y.clamp(
            adj.lower(),
            (adj.upper() - adj.page_size()).max(adj.lower()),
        );

        let target = adw::PropertyAnimationTarget::new(&adj, "value");
        let animation = adw::TimedAnimation::new(&sw, adj.value(), target_y, 400, target);
        animation.set_easing(adw::Easing::EaseOutCubic);
        animation.play();
    }
}

pub fn setup(
    content_stack: &adw::ViewStack,
    content_nav_page: &adw::NavigationPage,
    sidebar_list: &gtk4::ListBox,
    widget_map: HashMap<&'static str, gtk4::Widget>,
) -> SearchWidgets {
    let widget_map = Rc::new(widget_map);

    // --- Suchergebnisse-Liste ---
    let search_results_list = gtk4::ListBox::new();
    search_results_list.add_css_class("boxed-list");
    search_results_list.set_selection_mode(gtk4::SelectionMode::None);
    search_results_list.set_margin_top(12);
    search_results_list.set_margin_bottom(12);
    search_results_list.set_margin_start(12);
    search_results_list.set_margin_end(12);

    let search_scroll = gtk4::ScrolledWindow::new();
    search_scroll.set_child(Some(&search_results_list));
    search_scroll.set_vexpand(true);

    // --- Suchleiste ---
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_placeholder_text(Some(&t!("search_placeholder")));
    search_entry.set_hexpand(true);

    let search_bar = gtk4::SearchBar::new();
    search_bar.set_child(Some(&search_entry));
    search_bar.connect_entry(&search_entry);

    let search_toggle = gtk4::ToggleButton::new();
    search_toggle.set_icon_name("system-search-symbolic");

    // Toggle-Schaltfläche ↔ SearchBar
    let search_bar_t = search_bar.clone();
    search_toggle.connect_toggled(move |btn| {
        search_bar_t.set_search_mode(btn.is_active());
    });

    // SearchBar-Modus → Stack-Seite + Header-Titel wechseln
    let search_toggle_n = search_toggle.clone();
    let content_stack_n = content_stack.clone();
    let nav_page_n = content_nav_page.clone();
    let sidebar_list_n = sidebar_list.clone();
    let search_entry_n = search_entry.clone();
    search_bar.connect_notify_local(Some("search-mode-enabled"), move |bar, _| {
        let active = bar.is_search_mode();
        search_toggle_n.set_active(active);
        if active {
            content_stack_n.set_visible_child_name("search");
            nav_page_n.set_title(&t!("search_results_title"));
        } else {
            search_entry_n.set_text("");
            if let Some(row) = sidebar_list_n.selected_row() {
                let idx = row.index() as usize;
                if let Some(&(_, title_key, page_name)) = NAV_ITEMS.get(idx) {
                    content_stack_n.set_visible_child_name(page_name);
                    nav_page_n.set_title(&t!(title_key));
                }
            }
        }
    });

    // Sucheingabe → Ergebnisse filtern
    let results_list_c = search_results_list.clone();
    let search_bar_c = search_bar.clone();
    let sidebar_list_c = sidebar_list.clone();
    let widget_map_c = widget_map.clone();
    search_entry.connect_search_changed(move |entry| {
        let text = entry.text().to_lowercase();
        results_list_c.remove_all();

        if text.is_empty() {
            return;
        }

        let mut found = false;
        for item in SEARCH_INDEX {
            let title = t!(item.title_key);
            if title.to_lowercase().contains(&text) {
                found = true;
                let row = adw::ActionRow::new();
                row.set_title(&gtk4::glib::markup_escape_text(title.as_ref()));
                row.set_subtitle(&gtk4::glib::markup_escape_text(&t!(item.page_title_key)));
                let icon = gtk4::Image::from_icon_name(item.page_icon);
                icon.set_pixel_size(16);
                row.add_prefix(&icon);
                row.set_activatable(true);

                let search_bar_i = search_bar_c.clone();
                let sidebar_i = sidebar_list_c.clone();
                let page_idx = item.page_idx;
                let target_widget = widget_map_c.get(item.component_key).cloned();
                row.connect_activated(move |_| {
                    if let Some(r) = sidebar_i.row_at_index(page_idx) {
                        sidebar_i.select_row(Some(&r));
                    }
                    search_bar_i.set_search_mode(false);

                    if let Some(w) = target_widget.clone() {
                        gtk4::glib::timeout_add_local_once(
                            std::time::Duration::from_millis(150),
                            move || scroll_to_widget(&w),
                        );
                    }
                });

                results_list_c.append(&row);
            }
        }

        if !found {
            let row = adw::ActionRow::new();
            row.set_title(&t!("search_no_results"));
            row.set_sensitive(false);
            results_list_c.append(&row);
        }
    });

    SearchWidgets {
        scroll: search_scroll,
        bar: search_bar,
        toggle: search_toggle,
    }
}
