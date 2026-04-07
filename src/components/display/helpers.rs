//! Helpers for ICC color profile management and KDE D-Bus utilities.
//!
//! ICC profiles are embedded in the binary at compile time and extracted to
//! `~/.config/asus-hub/icm/` on first use. Profiles are applied via `kscreen-doctor`.

use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;
use rust_i18n::t;

/// `kscreen-doctor` output name for the built-in laptop display.
pub(crate) const DISPLAY_NAME: &str = "eDP-1";

const SRGB_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_sRGB.icm");
const DCIP3_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_DCIP3.icm");
const DISPLAYP3_ICM: &[u8] = include_bytes!("../../../assets/icm/ASUS_DisplayP3.icm");

/// Extracts the bundled ICM files to `~/.config/asus-hub/icm/` and returns that directory path.
///
/// Each file is only written if it does not already exist, making this safe to call on every
/// startup without unnecessary disk writes.
pub(crate) async fn setup_icm_profiles() -> Result<std::path::PathBuf, String> {
    let base = AppConfig::config_dir()
        .ok_or_else(|| t!("error_config_dir").to_string())?
        .join("icm");

    let base_clone = base.clone();
    tokio::task::spawn_blocking(move || {
        std::fs::create_dir_all(&base_clone)
            .map_err(|e| t!("error_icm_dir_create", error = e.to_string()).to_string())?;

        for (name, data) in [
            ("ASUS_sRGB.icm", SRGB_ICM),
            ("ASUS_DCIP3.icm", DCIP3_ICM),
            ("ASUS_DisplayP3.icm", DISPLAYP3_ICM),
        ] {
            let path = base_clone.join(name);
            if !path.exists() {
                std::fs::write(&path, data).map_err(|e| {
                    t!("error_icm_write", name = name, error = e.to_string()).to_string()
                })?;
            }
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| t!("error_spawn_blocking", error = e.to_string()).to_string())??;

    Ok(base)
}

/// Resets the display color profile to the monitor's built-in EDID default via `kscreen-doctor`.
pub(crate) async fn reset_icm_profile() -> Result<(), String> {
    let arg = format!("output.{}.colorProfileSource.EDID", DISPLAY_NAME);
    run_command_blocking("kscreen-doctor", &[&arg]).await
}

/// Applies an ICC profile file to [`DISPLAY_NAME`] via `kscreen-doctor`.
///
/// The argument format is `output.<display>.iccprofile.<absolute_path>`.
pub(crate) async fn apply_icm_profile(
    filename: &str,
    base_path: &std::path::Path,
) -> Result<(), String> {
    let arg = format!(
        "output.{}.iccprofile.{}",
        DISPLAY_NAME,
        base_path.join(filename).display()
    );
    run_command_blocking("kscreen-doctor", &[&arg]).await
}

/// Invokes a D-Bus method via the `qdbus` command-line tool with Qt5/Qt6 fallback.
///
/// Tries `qdbus-qt6` first; if it is not found (`ENOENT`), falls back to `qdbus` (Qt5).
/// This handles distros that ship only one of the two variants.
pub(crate) async fn run_qdbus(args: Vec<String>) -> Result<(), String> {
    let result = tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("qdbus-qt6").args(&args).status();
        match status {
            Ok(s) => Ok(("qdbus-qt6", s)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                std::process::Command::new("qdbus")
                    .args(&args)
                    .status()
                    .map(|s| ("qdbus", s))
            }
            Err(e) => Err(e),
        }
    })
    .await;

    match result {
        Ok(Ok((_, status))) if status.success() => Ok(()),
        Ok(Ok((cmd, status))) => Err(t!(
            "error_cmd_exit_code",
            cmd = cmd,
            code = status.code().unwrap_or(-1).to_string()
        )
        .to_string()),
        Ok(Err(e)) => Err(t!("error_qdbus_start", error = e.to_string()).to_string()),
        Err(e) => Err(t!("error_spawn_blocking", error = e.to_string()).to_string()),
    }
}
