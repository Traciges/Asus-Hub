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

use relm4::adw;
use relm4::adw::prelude::*;
use rust_i18n::t;
use std::collections::HashMap;
use std::rc::Rc;

const SCROLL_ANIMATION_DELAY_MS: u64 = 150;

// (icon, i18n-key, stack-name)
pub const NAV_ITEMS: [(&str, &str, &str); 8] = [
    ("go-home-symbolic", "tab_home", "home"),
    ("video-display-symbolic", "tab_display", "display"),
    ("input-keyboard-symbolic", "tab_keyboard", "keyboard"),
    ("preferences-color-symbolic", "tab_aura", "aura"),
    ("view-grid-symbolic", "tab_animatrix", "animatrix"),
    ("input-touchpad-symbolic", "tab_touchpad", "touchpad"),
    ("audio-headset-symbolic", "tab_audio", "audio"),
    ("preferences-system-symbolic", "tab_system", "system"),
];

pub fn sorted_nav_items() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut items: Vec<_> = NAV_ITEMS.iter().copied().collect();
    // Home is always pinned first; the rest are sorted alphabetically.
    let home_pos = items.iter().position(|&(_, _, n)| n == "home");
    if let Some(idx) = home_pos {
        let home = items.remove(idx);
        items.sort_by(|a, b| t!(a.1).as_ref().cmp(t!(b.1).as_ref()));
        items.insert(0, home);
    } else {
        items.sort_by(|a, b| t!(a.1).as_ref().cmp(t!(b.1).as_ref()));
    }
    items
}

struct SearchItem {
    title_key: &'static str,
    page_icon: &'static str,
    page_title_key: &'static str,
    page_name: &'static str,
    component_key: &'static str,
}

static SEARCH_INDEX: &[SearchItem] = &[
    // Home
    SearchItem {
        title_key: "home_board_title",
        page_icon: "go-home-symbolic",
        page_title_key: "tab_home",
        page_name: "home",
        component_key: "home_info",
    },
    SearchItem {
        title_key: "home_bios_title",
        page_icon: "go-home-symbolic",
        page_title_key: "tab_home",
        page_name: "home",
        component_key: "home_info",
    },
    SearchItem {
        title_key: "home_serial_title",
        page_icon: "go-home-symbolic",
        page_title_key: "tab_home",
        page_name: "home",
        component_key: "home_info",
    },
    // Display
    SearchItem {
        title_key: "oled_dimming_group_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "oled_dimming",
    },
    SearchItem {
        title_key: "oled_dimming_slider_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "oled_dimming",
    },
    SearchItem {
        title_key: "target_mode_group_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "target_mode",
    },
    SearchItem {
        title_key: "target_mode_switch_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "target_mode",
    },
    SearchItem {
        title_key: "oled_care_pixel_refresh_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "oled_care",
    },
    SearchItem {
        title_key: "oled_care_panel_autohide_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "oled_care",
    },
    SearchItem {
        title_key: "oled_care_transparency_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "oled_care",
    },
    SearchItem {
        title_key: "color_gamut_title",
        page_icon: "video-display-symbolic",
        page_title_key: "tab_display",
        page_name: "display",
        component_key: "color_gamut",
    },
    // Aura
    SearchItem {
        title_key: "aura_group_title",
        page_icon: "preferences-color-symbolic",
        page_title_key: "tab_aura",
        page_name: "aura",
        component_key: "aura",
    },
    SearchItem {
        title_key: "aura_mode_title",
        page_icon: "preferences-color-symbolic",
        page_title_key: "tab_aura",
        page_name: "aura",
        component_key: "aura",
    },
    // AniMatrix
    SearchItem {
        title_key: "animatrix_group_title",
        page_icon: "view-grid-symbolic",
        page_title_key: "tab_animatrix",
        page_name: "animatrix",
        component_key: "animatrix",
    },
    SearchItem {
        title_key: "animatrix_enable_title",
        page_icon: "view-grid-symbolic",
        page_title_key: "tab_animatrix",
        page_name: "animatrix",
        component_key: "animatrix",
    },
    SearchItem {
        title_key: "animatrix_brightness_title",
        page_icon: "view-grid-symbolic",
        page_title_key: "tab_animatrix",
        page_name: "animatrix",
        component_key: "animatrix",
    },
    SearchItem {
        title_key: "animatrix_gallery_title",
        page_icon: "view-grid-symbolic",
        page_title_key: "tab_animatrix",
        page_name: "animatrix",
        component_key: "animatrix",
    },
    // Keyboard
    SearchItem {
        title_key: "backlight_auto_on_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "auto_backlight",
    },
    SearchItem {
        title_key: "backlight_auto_off_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "auto_backlight",
    },
    SearchItem {
        title_key: "backlight_threshold_on_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "auto_backlight",
    },
    SearchItem {
        title_key: "backlight_threshold_off_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "auto_backlight",
    },
    SearchItem {
        title_key: "sleep_group_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "backlight_idle",
    },
    SearchItem {
        title_key: "fn_key_group_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "fn_key",
    },
    SearchItem {
        title_key: "fan_hotkey_enable_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "asus_key_hint",
    },
    SearchItem {
        title_key: "fan_osd_enabled_title",
        page_icon: "input-keyboard-symbolic",
        page_title_key: "tab_keyboard",
        page_name: "keyboard",
        component_key: "asus_key_hint",
    },
    // Touchpad
    SearchItem {
        title_key: "gestures_group_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "gestures",
    },
    SearchItem {
        title_key: "touchpad_group_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "touchpad",
    },
    SearchItem {
        title_key: "numberpad_group_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "numberpad",
    },
    SearchItem {
        title_key: "numberpad_enable_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "numberpad",
    },
    SearchItem {
        title_key: "typing_group_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "typing",
    },
    SearchItem {
        title_key: "typing_disable_gestures_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "typing",
    },
    SearchItem {
        title_key: "typing_disable_touchpad_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "typing",
    },
    SearchItem {
        title_key: "typing_delay_title",
        page_icon: "input-touchpad-symbolic",
        page_title_key: "tab_touchpad",
        page_name: "touchpad",
        component_key: "typing",
    },
    // Audio
    SearchItem {
        title_key: "volume_booster_title",
        page_icon: "audio-headset-symbolic",
        page_title_key: "tab_audio",
        page_name: "audio",
        component_key: "volume",
    },
    SearchItem {
        title_key: "audio_profiles_title",
        page_icon: "audio-headset-symbolic",
        page_title_key: "tab_audio",
        page_name: "audio",
        component_key: "sound_modes",
    },
    // System
    SearchItem {
        title_key: "battery_maintenance_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "battery",
    },
    SearchItem {
        title_key: "battery_full_charge_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "battery",
    },
    SearchItem {
        title_key: "battery_deep_sleep_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "battery",
    },
    SearchItem {
        title_key: "fan_group_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "fan",
    },
    SearchItem {
        title_key: "language_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "lang",
    },
    SearchItem {
        title_key: "gpu_group_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "gpu",
    },
    SearchItem {
        title_key: "gpu_mode_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "gpu",
    },
    SearchItem {
        title_key: "apu_mem_group_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "apu_mem",
    },
    SearchItem {
        title_key: "apu_mem_title",
        page_icon: "preferences-system-symbolic",
        page_title_key: "tab_system",
        page_name: "system",
        component_key: "apu_mem",
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
    sorted_items: Vec<(&'static str, &'static str, &'static str)>,
    content_stack: &adw::ViewStack,
    content_nav_page: &adw::NavigationPage,
    sidebar_list: &gtk4::ListBox,
    widget_map: HashMap<&'static str, gtk4::Widget>,
) -> SearchWidgets {
    let widget_map = Rc::new(widget_map);
    let sorted_items = Rc::new(sorted_items);

    // Search results
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

    // Searchbar
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_placeholder_text(Some(&t!("search_placeholder")));
    search_entry.set_hexpand(true);

    let search_bar = gtk4::SearchBar::new();
    search_bar.set_child(Some(&search_entry));
    search_bar.connect_entry(&search_entry);

    let search_toggle = gtk4::ToggleButton::new();
    search_toggle.set_icon_name("system-search-symbolic");

    // Toggle bar
    let search_bar_t = search_bar.clone();
    search_toggle.connect_toggled(move |btn| {
        search_bar_t.set_search_mode(btn.is_active());
    });

    // Switch to Stack Page + Header Title
    let search_toggle_n = search_toggle.clone();
    let content_stack_n = content_stack.clone();
    let nav_page_n = content_nav_page.clone();
    let sidebar_list_n = sidebar_list.clone();
    let search_entry_n = search_entry.clone();
    let sorted_items_n = sorted_items.clone();
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
                if let Some(&(_, title_key, page_name)) = sorted_items_n.get(idx) {
                    content_stack_n.set_visible_child_name(page_name);
                    nav_page_n.set_title(&t!(title_key));
                }
            }
        }
    });

    // Filter results
    let results_list_c = search_results_list.clone();
    let search_bar_c = search_bar.clone();
    let sidebar_list_c = sidebar_list.clone();
    let widget_map_c = widget_map.clone();
    let sorted_items_c = sorted_items.clone();
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
                let page_idx = sorted_items_c
                    .iter()
                    .position(|&(_, _, name)| name == item.page_name)
                    .unwrap_or(0) as i32;
                let target_widget = widget_map_c.get(item.component_key).cloned();
                row.connect_activated(move |_| {
                    if let Some(r) = sidebar_i.row_at_index(page_idx) {
                        sidebar_i.select_row(Some(&r));
                    }
                    search_bar_i.set_search_mode(false);

                    if let Some(w) = target_widget.clone() {
                        gtk4::glib::timeout_add_local_once(
                            std::time::Duration::from_millis(SCROLL_ANIMATION_DELAY_MS),
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
