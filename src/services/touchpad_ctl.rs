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

//! Desktop-environment agnostic touchpad enable/disable.
//!
//! Shared by the manual switch in [`crate::components::touchpad::touchpad`] and
//! the automatic "disable while typing" watcher in
//! [`crate::services::typing_watch`].

use rust_i18n::t;

use crate::components::display::helpers::run_qdbus;
use crate::services::commands::{is_gnome_desktop, is_kde_desktop, run_command_blocking};

/// Whether the current desktop environment supports touchpad toggling.
pub fn desktop_supported() -> bool {
    is_kde_desktop() || is_gnome_desktop()
}

/// Enables or disables the touchpad using the appropriate desktop-environment API.
///
/// Uses `gsettings` on GNOME, `qdbus` on KDE, and returns an error on unsupported desktops.
pub async fn set_enabled(active: bool) -> Result<(), String> {
    if is_gnome_desktop() {
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
    } else if is_kde_desktop() {
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
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_lowercase();
        Err(t!("error_touchpad_unsupported_desktop", desktop = desktop).to_string())
    }
}
