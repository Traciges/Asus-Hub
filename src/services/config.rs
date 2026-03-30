use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

fn default_aufhellung_schwelle() -> f64 {
    12.0
}
fn default_abdunklung_schwelle() -> f64 {
    35.0
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub farbskala_index: u32,
    pub zielmodus_aktiv: bool,
    pub oled_care_pixel_refresh: bool,
    pub oled_care_panel_autohide: bool,
    pub oled_care_transparenz: bool,
    pub fan_tiefschlaf_aktiv: bool,
    pub fan_profil: u32,
    pub input_gesten_aktiv: bool,
    pub input_fn_key_gesperrt: bool,
    #[serde(default)]
    pub kbd_aufhellung_aktiv: bool,
    #[serde(default)]
    pub kbd_abdunklung_aktiv: bool,
    #[serde(default)]
    pub kbd_timeout_modus: u32,
    #[serde(default)]
    pub kbd_timeout_akku_netz_index: u32,
    #[serde(default)]
    pub kbd_timeout_nur_akku_index: u32,
    #[serde(default = "default_aufhellung_schwelle")]
    pub kbd_aufhellung_schwelle: f64,
    #[serde(default = "default_abdunklung_schwelle")]
    pub kbd_abdunklung_schwelle: f64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            farbskala_index: 0,
            zielmodus_aktiv: false,
            oled_care_pixel_refresh: false,
            oled_care_panel_autohide: false,
            oled_care_transparenz: false,
            fan_tiefschlaf_aktiv: false,
            fan_profil: 0,
            input_gesten_aktiv: false,
            input_fn_key_gesperrt: false,
            kbd_aufhellung_aktiv: false,
            kbd_abdunklung_aktiv: false,
            kbd_timeout_modus: 0,
            kbd_timeout_akku_netz_index: 0,
            kbd_timeout_nur_akku_index: 0,
            kbd_aufhellung_schwelle: 12.0,
            kbd_abdunklung_schwelle: 35.0,
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Option<std::path::PathBuf> {
        ProjectDirs::from("", "", "zenbook-control").map(|dirs| dirs.config_dir().to_path_buf())
    }

    fn config_path() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|dir| dir.join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
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
}
