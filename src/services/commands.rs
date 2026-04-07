use rust_i18n::t;

/// Runs a program with arguments on a blocking thread and returns success or an i18n error string.
///
/// Offloads the synchronous [`std::process::Command`] call to a `spawn_blocking` thread so it
/// does not stall the async runtime. Returns `Err` on spawn failure, non-zero exit code, or
/// if the blocking task itself panics.
pub(crate) async fn run_command_blocking(program: &str, args: &[&str]) -> Result<(), String> {
    let program_name = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&program_name)
            .args(&args)
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => Ok(()),
        Ok(Ok(status)) => Err(t!(
            "error_cmd_exit_code",
            cmd = program,
            code = status.code().unwrap_or(-1).to_string()
        )
        .to_string()),
        Ok(Err(e)) => Err(t!("error_cmd_start", cmd = program, error = e.to_string()).to_string()),
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

/// Returns `true` if the current desktop session is KDE Plasma.
///
/// Checks the `XDG_CURRENT_DESKTOP` environment variable for the substring `"KDE"` (case-insensitive).
pub(crate) fn is_kde_desktop() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|v| v.to_uppercase().contains("KDE"))
        .unwrap_or(false)
}
