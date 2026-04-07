use rust_i18n::t;

#[zbus::proxy(
    interface = "xyz.ljones.Platform",
    default_service = "xyz.ljones.Asusd",
    default_path = "/xyz/ljones"
)]
trait Platform {
    #[zbus(property)]
    fn charge_control_end_threshold(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn set_charge_control_end_threshold(&self, value: u8) -> zbus::Result<()>;

    #[zbus(property)]
    fn platform_profile(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn set_platform_profile(&self, value: u32) -> zbus::Result<()>;
}

/// Fan/platform power profile exposed by the `asusd` daemon.
///
/// Maps directly to the integer values used by the `platform_profile` D-Bus property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FanProfile {
    /// Balanced power and thermal performance (default, value `0`).
    Balanced = 0,
    /// Maximum CPU/GPU boost, higher fan speed (value `1`).
    Performance = 1,
    /// Reduced fan noise and power draw (value `2`).
    Quiet = 2,
}

impl From<u32> for FanProfile {
    fn from(value: u32) -> Self {
        match value {
            1 => Self::Performance,
            2 => Self::Quiet,
            _ => Self::Balanced,
        }
    }
}

/// Lazily-initialized singleton proxy to the `xyz.ljones.Asusd` D-Bus service.
///
/// The proxy is created once on first use and reused for all subsequent calls,
/// avoiding repeated connection overhead.
static PLATFORM_PROXY: tokio::sync::OnceCell<PlatformProxy<'static>> =
    tokio::sync::OnceCell::const_new();

/// Returns a reference to the shared [`PlatformProxy`], initialising it on first call.
async fn platform_proxy() -> Result<&'static PlatformProxy<'static>, String> {
    PLATFORM_PROXY
        .get_or_try_init(|| async {
            let conn = zbus::Connection::system()
                .await
                .map_err(|e| t!("error_dbus_connect", error = e.to_string()).to_string())?;
            PlatformProxy::new(&conn)
                .await
                .map_err(|e| t!("error_dbus_proxy_create", error = e.to_string()).to_string())
        })
        .await
}

/// Returns `true` if the `asusd` D-Bus service is reachable.
///
/// Opens a fresh system bus connection each time to avoid caching a stale result.
/// Does not initialise the shared [`PLATFORM_PROXY`].
pub async fn check_asusd_available() -> bool {
    let conn = match zbus::Connection::system().await {
        Ok(c) => c,
        Err(_) => return false,
    };
    PlatformProxy::new(&conn).await.is_ok()
}

/// Reads the current battery charge end-threshold from `asusd` (typically 80 or 100).
pub async fn get_charge_limit() -> Result<u8, String> {
    let proxy = platform_proxy().await?;
    proxy
        .charge_control_end_threshold()
        .await
        .map_err(|e| t!("error_charge_limit_read", error = e.to_string()).to_string())
}

/// Sets the battery charge end-threshold via `asusd` and returns the applied value.
///
/// Pass `80` for maintenance/health mode or `100` for a full charge.
pub async fn set_charge_limit(value: u8) -> Result<u8, String> {
    let proxy = platform_proxy().await?;
    proxy
        .set_charge_control_end_threshold(value)
        .await
        .map_err(|e| t!("error_charge_limit_write", error = e.to_string()).to_string())?;
    Ok(value)
}

/// Reads the active fan/platform profile from `asusd`.
pub async fn get_fan_profile() -> Result<FanProfile, String> {
    let proxy = platform_proxy().await?;
    proxy
        .platform_profile()
        .await
        .map(FanProfile::from)
        .map_err(|e| t!("error_fan_profile_read", error = e.to_string()).to_string())
}

/// Applies a fan/platform profile via `asusd` and returns the applied profile on success.
pub async fn set_fan_profile(profile: FanProfile) -> Result<FanProfile, String> {
    let proxy = platform_proxy().await?;
    proxy
        .set_platform_profile(profile as u32)
        .await
        .map_err(|e| t!("error_fan_profile_write", error = e.to_string()).to_string())?;
    Ok(profile)
}
