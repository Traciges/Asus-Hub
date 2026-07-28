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

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

use super::migration::LegacyAppConfig;

fn default_language() -> String {
    "en".to_string()
}
fn default_profile_icon() -> String {
    "computer-symbolic".to_string()
}
fn default_volume() -> f64 {
    100.0
}
fn default_aura_brightness() -> u32 {
    2
}
fn default_aura_colour_r() -> u8 {
    166
}
fn default_aura_speed() -> String {
    "Med".to_string()
}
fn default_aura_direction() -> String {
    "Right".to_string()
}
fn default_animatrix_enable() -> bool {
    true
}
fn default_animatrix_brightness() -> u32 {
    2
}
fn default_animatrix_builtins() -> bool {
    true
}
fn default_animatrix_boot_anim() -> String {
    "GlitchConstruction".to_string()
}
fn default_animatrix_awake_anim() -> String {
    "BinaryBannerScroll".to_string()
}
fn default_animatrix_sleep_anim() -> String {
    "BannerSwipe".to_string()
}
fn default_animatrix_shutdown_anim() -> String {
    "GlitchOut".to_string()
}
fn default_true() -> bool {
    true
}
fn default_typing_delay_ms() -> u32 {
    1000
}

fn generate_profile_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{:x}{:04x}", t.as_secs(), t.subsec_millis())
}

/// A named hardware + software preset that can be switched at runtime.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default = "default_profile_icon")]
    pub icon: String,

    // Display
    pub fan_profile: u32,
    pub oled_dc_dimming: u32,
    pub target_mode_active: bool,
    pub color_profile_index: u32,
    pub oled_care_pixel_refresh: bool,
    pub oled_care_panel_autohide: bool,
    pub oled_care_transparency: bool,

    // Audio
    pub audio_profile: u32,
    #[serde(default = "default_volume")]
    pub volume: f64,

    // Keyboard & input
    pub kbd_timeout_mode: u32,
    pub kbd_timeout_battery_ac_index: u32,
    pub kbd_timeout_battery_only_index: u32,
    pub kbd_brighten_active: bool,
    pub kbd_dim_active: bool,
    pub kbd_brighten_threshold: f64,
    pub kbd_dim_threshold: f64,
    pub touchpad_active: bool,
    pub input_gestures_active: bool,
    pub input_fn_key_locked: bool,
    #[serde(default)]
    pub numberpad_active: bool,
    /// Suppress touchpad edge gestures while typing.
    #[serde(default)]
    pub typing_disable_gestures: bool,
    /// Disable the whole touchpad while typing.
    #[serde(default)]
    pub typing_disable_touchpad: bool,
    /// Idle time after the last keystroke before gestures/touchpad come back.
    #[serde(default = "default_typing_delay_ms")]
    pub typing_reactivation_delay_ms: u32,

    // System
    pub battery_deep_sleep_active: bool,
    pub gpu_mode: u32,
    pub apu_mem: i32,

    // Keyboard RGB (Aura)
    #[serde(default)]
    pub aura_mode: u32,
    #[serde(default = "default_aura_brightness")]
    pub aura_brightness: u32,
    #[serde(default = "default_aura_colour_r")]
    pub aura_colour_r: u8,
    #[serde(default)]
    pub aura_colour_g: u8,
    #[serde(default)]
    pub aura_colour_b: u8,
    #[serde(default)]
    pub aura_zone: u32,
    #[serde(default)]
    pub aura_colour2_r: u8,
    #[serde(default)]
    pub aura_colour2_g: u8,
    #[serde(default)]
    pub aura_colour2_b: u8,
    #[serde(default = "default_aura_speed")]
    pub aura_speed: String,
    #[serde(default = "default_aura_direction")]
    pub aura_direction: String,

    // AniMatrix LED panel
    #[serde(default = "default_animatrix_enable")]
    pub animatrix_enable_display: bool,
    #[serde(default = "default_animatrix_brightness")]
    pub animatrix_brightness: u32,
    #[serde(default = "default_animatrix_builtins")]
    pub animatrix_builtins_enabled: bool,
    #[serde(default = "default_animatrix_boot_anim")]
    pub animatrix_boot_anim: String,
    #[serde(default = "default_animatrix_awake_anim")]
    pub animatrix_awake_anim: String,
    #[serde(default = "default_animatrix_sleep_anim")]
    pub animatrix_sleep_anim: String,
    #[serde(default = "default_animatrix_shutdown_anim")]
    pub animatrix_shutdown_anim: String,
    #[serde(default)]
    pub animatrix_off_when_unplugged: bool,
    #[serde(default)]
    pub animatrix_off_when_suspended: bool,
    #[serde(default)]
    pub animatrix_off_when_lid_closed: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Default".to_string(),
            icon: "computer-symbolic".to_string(),
            fan_profile: 0,
            oled_dc_dimming: 100,
            target_mode_active: false,
            color_profile_index: 0,
            oled_care_pixel_refresh: false,
            oled_care_panel_autohide: false,
            oled_care_transparency: false,
            audio_profile: 0,
            volume: 100.0,
            kbd_timeout_mode: 0,
            kbd_timeout_battery_ac_index: 0,
            kbd_timeout_battery_only_index: 0,
            kbd_brighten_active: false,
            kbd_dim_active: false,
            kbd_brighten_threshold: 12.0,
            kbd_dim_threshold: 35.0,
            touchpad_active: true,
            input_gestures_active: false,
            input_fn_key_locked: false,
            numberpad_active: false,
            typing_disable_gestures: false,
            typing_disable_touchpad: false,
            typing_reactivation_delay_ms: default_typing_delay_ms(),
            battery_deep_sleep_active: false,
            gpu_mode: 0,
            apu_mem: 0,
            aura_mode: 0,
            aura_brightness: 2,
            aura_colour_r: 166,
            aura_colour_g: 0,
            aura_colour_b: 0,
            aura_zone: 0,
            aura_colour2_r: 0,
            aura_colour2_g: 0,
            aura_colour2_b: 0,
            aura_speed: "Med".to_string(),
            aura_direction: "Right".to_string(),
            animatrix_enable_display: true,
            animatrix_brightness: 2,
            animatrix_builtins_enabled: true,
            animatrix_boot_anim: "GlitchConstruction".to_string(),
            animatrix_awake_anim: "BinaryBannerScroll".to_string(),
            animatrix_sleep_anim: "BannerSwipe".to_string(),
            animatrix_shutdown_anim: "GlitchOut".to_string(),
            animatrix_off_when_unplugged: false,
            animatrix_off_when_suspended: false,
            animatrix_off_when_lid_closed: false,
        }
    }
}

/// Persistent application configuration stored as JSON at `~/.config/ayuz/config.json`.
#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    // ── Global (non-profile) settings ────────────────────────────────────────
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub skip_legacy_migration: bool,
    #[serde(default = "default_true", alias = "show_fan_osd")]
    pub fan_osd_enabled: bool,
    #[serde(default = "default_true")]
    pub fan_hotkey_enabled: bool,

    // ── Profile management ───────────────────────────────────────────────────
    #[serde(default)]
    pub active_profile_id: String,
    #[serde(default)]
    pub profiles: Vec<Profile>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: default_language(),
            skip_legacy_migration: false,
            fan_osd_enabled: true,
            fan_hotkey_enabled: true,
            active_profile_id: String::new(),
            profiles: vec![],
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Option<std::path::PathBuf> {
        ProjectDirs::from("", "", "ayuz").map(|dirs| dirs.config_dir().to_path_buf())
    }

    fn config_path() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            let mut c = Self::default();
            c.ensure_default_profile();
            return c;
        };
        let mut config: AppConfig = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        config.ensure_default_profile();
        config
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, json);
        }
    }

    pub fn update(f: impl FnOnce(&mut Self)) {
        let mut config = Self::load();
        f(&mut config);
        config.save();
    }

    /// Returns a reference to the currently active profile.
    pub fn active_profile(&self) -> &Profile {
        self.profiles
            .iter()
            .find(|p| p.id == self.active_profile_id)
            .or_else(|| self.profiles.first())
            .expect("profiles must not be empty after ensure_default_profile")
    }

    /// Returns a mutable reference to the currently active profile.
    pub fn active_profile_mut(&mut self) -> &mut Profile {
        let id = self.active_profile_id.clone();
        if let Some(idx) = self.profiles.iter().position(|p| p.id == id) {
            return &mut self.profiles[idx];
        }
        &mut self.profiles[0]
    }

    /// If `profiles` is empty (first run / upgrade from pre-profile config), creates a
    /// "Default" profile, seeding it from the legacy flat fields if any are present
    /// in the on-disk config, otherwise from [`Profile::default`].
    pub fn ensure_default_profile(&mut self) {
        if !self.profiles.is_empty() {
            return;
        }
        let id = generate_profile_id();
        let mut profile = Profile {
            id: id.clone(),
            name: "Default".to_string(),
            ..Profile::default()
        };
        if let Some(legacy) = LegacyAppConfig::try_load() {
            profile.fan_profile = legacy.fan_profile;
            profile.color_profile_index = legacy.color_profile_index;
            profile.target_mode_active = legacy.target_mode_active;
            profile.oled_care_pixel_refresh = legacy.oled_care_pixel_refresh;
            profile.oled_care_panel_autohide = legacy.oled_care_panel_autohide;
            profile.oled_care_transparency = legacy.oled_care_transparency;
            profile.audio_profile = legacy.audio_profile;
            profile.kbd_brighten_active = legacy.kbd_brighten_active;
            profile.kbd_dim_active = legacy.kbd_dim_active;
            profile.kbd_timeout_mode = legacy.kbd_timeout_mode;
            profile.kbd_timeout_battery_ac_index = legacy.kbd_timeout_battery_ac_index;
            profile.kbd_timeout_battery_only_index = legacy.kbd_timeout_battery_only_index;
            profile.input_gestures_active = legacy.input_gestures_active;
            profile.input_fn_key_locked = legacy.input_fn_key_locked;
            profile.battery_deep_sleep_active = legacy.battery_deep_sleep_active;
            profile.gpu_mode = legacy.gpu_mode;
            profile.apu_mem = legacy.apu_mem;
            if let Some(v) = legacy.oled_dc_dimming {
                profile.oled_dc_dimming = v;
            }
            if let Some(v) = legacy.kbd_brighten_threshold {
                profile.kbd_brighten_threshold = v;
            }
            if let Some(v) = legacy.kbd_dim_threshold {
                profile.kbd_dim_threshold = v;
            }
            if let Some(v) = legacy.touchpad_active {
                profile.touchpad_active = v;
            }
        }
        self.profiles.push(profile);
        self.active_profile_id = id;
        self.save();
    }
}
