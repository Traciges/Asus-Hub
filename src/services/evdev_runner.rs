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

//! Shared helpers for evdev-based listener services (`fan_hotkey`,
//! `edge_gestures`, `numberpad`). Device enumeration criteria are common
//! enough that consumers share them; per-consumer behaviour lives in the
//! callers.

use evdev::{AbsoluteAxisCode, Device, EventStream, KeyCode};
use rust_i18n::t;

/// Converts a [`Device`] into an [`EventStream`], logging a warning and
/// returning `None` if the conversion fails.
pub fn open_event_stream(device: Device) -> Option<EventStream> {
    match device.into_event_stream() {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::warn!("{}", t!("error_event_read", error = e.to_string()));
            None
        }
    }
}

/// Scans `/dev/input/` for the first device whose name contains "touchpad"
/// and that reports both an X and Y absolute axis (either legacy `ABS_X/Y`
/// or multi-touch `ABS_MT_POSITION_X/Y`).
///
/// Our own virtual devices are skipped: the NumberPad relay
/// ([`crate::services::numberpad_pointer`]) clones the real touchpad closely
/// enough that a restarted service could otherwise grab its own output.
pub fn find_touchpad() -> Option<Device> {
    for (_, device) in evdev::enumerate() {
        let name = device.name().unwrap_or_default().to_lowercase();
        if !name.contains("touchpad") || name.starts_with("ayuz") {
            continue;
        }
        if let Some(axes) = device.supported_absolute_axes()
            && (axes.contains(AbsoluteAxisCode::ABS_X)
                || axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_X))
            && (axes.contains(AbsoluteAxisCode::ABS_Y)
                || axes.contains(AbsoluteAxisCode::ABS_MT_POSITION_Y))
        {
            return Some(device);
        }
    }
    None
}

/// Scans `/dev/input/` for every device that looks like a text-entry keyboard,
/// i.e. one that advertises the alphabetic block plus space and left shift.
///
/// Both the built-in and any external keyboard are returned, so callers that
/// watch for typing activity see all of them.
pub fn find_keyboards() -> Vec<Device> {
    evdev::enumerate()
        .filter_map(|(_, device)| {
            let keys = device.supported_keys()?;
            let is_keyboard = keys.contains(KeyCode::KEY_A)
                && keys.contains(KeyCode::KEY_Z)
                && keys.contains(KeyCode::KEY_SPACE)
                && keys.contains(KeyCode::KEY_LEFTSHIFT);
            is_keyboard.then_some(device)
        })
        .collect()
}

/// Reads the touchpad's absolute axis bounds. Prefers multi-touch
/// (`ABS_MT_POSITION_*`) and falls back to legacy (`ABS_X/Y`). Returns
/// `None` when either axis is missing or reports a non-positive maximum.
pub fn touchpad_abs_bounds(device: &Device) -> Option<(i32, i32)> {
    let abs_state = match device.get_abs_state() {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("{}", t!("error_abs_info", error = e.to_string()));
            return None;
        }
    };
    let x_max = {
        let mt = abs_state[AbsoluteAxisCode::ABS_MT_POSITION_X.0 as usize].maximum;
        if mt > 0 {
            mt
        } else {
            abs_state[AbsoluteAxisCode::ABS_X.0 as usize].maximum
        }
    };
    let y_max = {
        let mt = abs_state[AbsoluteAxisCode::ABS_MT_POSITION_Y.0 as usize].maximum;
        if mt > 0 {
            mt
        } else {
            abs_state[AbsoluteAxisCode::ABS_Y.0 as usize].maximum
        }
    };
    if x_max <= 0 || y_max <= 0 {
        return None;
    }
    Some((x_max, y_max))
}
