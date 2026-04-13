// Asus Hub - Unofficial Control Center for Asus Laptops
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

use rust_i18n::t;

static IS_FLATPAK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// Returns `true` when the process is running inside a Flatpak sandbox.
///
/// Checks for the presence of `/.flatpak-info`, which the Flatpak runtime always creates.
/// The result is cached after the first call.
pub(crate) fn is_flatpak() -> bool {
    *IS_FLATPAK.get_or_init(|| std::path::Path::new("/.flatpak-info").exists())
}

/// Runs a program with arguments on a blocking thread and returns success or an i18n error string.
///
/// Offloads the synchronous [`std::process::Command`] call to a `spawn_blocking` thread so it
/// does not stall the async runtime. Returns `Err` on spawn failure, non-zero exit code, or
/// if the blocking task itself panics.
///
/// Inside a Flatpak sandbox the command is transparently wrapped in `flatpak-spawn --host`
/// so it executes on the host system rather than inside the sandbox.
pub(crate) async fn run_command_blocking(program: &str, args: &[&str]) -> Result<(), String> {
    let program_display = program.to_string();

    let (run_program, run_args): (String, Vec<String>) = if is_flatpak() {
        let mut a = vec!["--host".to_string(), program.to_string()];
        a.extend(args.iter().map(|s| s.to_string()));
        ("flatpak-spawn".to_string(), a)
    } else {
        (
            program.to_string(),
            args.iter().map(|s| s.to_string()).collect(),
        )
    };

    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&run_program)
            .args(&run_args)
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(t!(
            "error_cmd_exit_code",
            cmd = program_display,
            code = status.code().unwrap_or(-1).to_string()
        )
        .to_string()),
        Ok(Err(e)) => Err(t!(
            "error_cmd_start",
            cmd = program_display,
            error = e.to_string()
        )
        .to_string()),
        Err(e) => Err(t!("error_spawn_blocking", error = e.to_string()).to_string()),
    }
}

/// Runs a shell command with elevated privileges via `pkexec sh -c <command>`.
///
/// Prompts the user for authentication through the system's PolicyKit agent.
/// Prefer this over embedding `sudo` calls directly in command strings.
pub(crate) async fn pkexec_shell(command: &str) -> Result<(), String> {
    run_command_blocking("pkexec", &["sh", "-c", command]).await
}

static QDBUS_PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Returns the path to the `qdbus` executable, resolved once and cached.
///
/// Searches `$PATH` for `qdbus`, `qdbus6`, and `qdbus-qt6` in order, then falls back to a
/// set of known fixed locations (Fedora `/usr/bin/qdbus6`, Arch Qt6/Qt5 lib paths).
/// Returns the bare string `"qdbus"` as a last resort if nothing is found.
pub(crate) fn resolve_qdbus_path() -> &'static str {
    QDBUS_PATH.get_or_init(|| {
        if is_flatpak() {
            // Search on the host via flatpak-spawn, mirroring the non-Flatpak logic below.
            for name in ["qdbus6", "qdbus", "qdbus-qt6"] {
                if std::process::Command::new("flatpak-spawn")
                    .args(["--host", "which", name])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    return name.to_owned();
                }
            }
            for fixed in [
                "/usr/bin/qdbus6",
                "/usr/lib/qt6/bin/qdbus6",
                "/usr/lib/qt6/bin/qdbus",
                "/usr/lib/qt5/bin/qdbus",
            ] {
                if std::process::Command::new("flatpak-spawn")
                    .args(["--host", "test", "-x", fixed])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
                {
                    return fixed.to_owned();
                }
            }
            return "qdbus6".to_owned();
        }
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(':') {
                for name in ["qdbus", "qdbus6", "qdbus-qt6"] {
                    let candidate = std::path::Path::new(dir).join(name);
                    if is_executable(&candidate) {
                        return candidate.to_string_lossy().into_owned();
                    }
                }
            }
        }
        for fixed in [
            "/usr/bin/qdbus6",
            "/usr/lib/qt6/bin/qdbus6",
            "/usr/lib/qt6/bin/qdbus",
            "/usr/lib/qt5/bin/qdbus",
        ] {
            if is_executable(std::path::Path::new(fixed)) {
                return fixed.to_owned();
            }
        }
        "qdbus".to_owned()
    })
}

fn is_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Reads a file from the host filesystem.
///
/// Inside a Flatpak sandbox, `/sys/` and similar kernel paths are inaccessible via native
/// file APIs. This function uses `flatpak-spawn --host cat` when running inside Flatpak,
/// and falls back to `tokio::fs::read_to_string` on a normal system.
pub(crate) async fn read_file_host(path: &str) -> std::io::Result<String> {
    if is_flatpak() {
        let path_owned = path.to_string();
        tokio::task::spawn_blocking(move || {
            let output = std::process::Command::new("flatpak-spawn")
                .args(["--host", "cat", &path_owned])
                .output()?;
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ))
            }
        })
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?
    } else {
        tokio::fs::read_to_string(path).await
    }
}

/// Returns `true` if the current desktop session is KDE Plasma.
///
/// Checks the `XDG_CURRENT_DESKTOP` environment variable for the substring `"KDE"` (case-insensitive).
pub(crate) fn is_kde_desktop() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
}

/// Returns `true` if the current desktop session is GNOME.
///
/// Checks the `XDG_CURRENT_DESKTOP` environment variable for the substring `"GNOME"` (case-insensitive).
pub(crate) fn is_gnome_desktop() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("GNOME"))
        .unwrap_or(false)
}
