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

pub mod commands;
pub mod config;
pub mod dbus;
pub mod dbus_animatrix;
pub mod edge_gestures;
pub(crate) mod evdev_runner;
pub mod fan_hotkey;
pub mod kde_brightness;
pub mod migration;
pub mod numberpad;
pub mod numberpad_layouts;
pub mod numberpad_pointer;
pub mod touchpad_ctl;
pub mod typing_watch;
