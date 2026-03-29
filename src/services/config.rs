use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Default, Serialize, Deserialize, Clone)]
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
}

impl AppConfig {
    pub fn config_dir() -> Option<std::path::PathBuf> {
        ProjectDirs::from("", "", "myasus-linux").map(|dirs| dirs.config_dir().to_path_buf())
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
