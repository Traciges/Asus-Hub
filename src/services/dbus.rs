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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum FanProfile {
    Balanced = 0,
    Performance = 1,
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

static PLATFORM_PROXY: tokio::sync::OnceCell<PlatformProxy<'static>> =
    tokio::sync::OnceCell::const_new();

async fn platform_proxy() -> Result<&'static PlatformProxy<'static>, String> {
    PLATFORM_PROXY
        .get_or_try_init(|| async {
            let conn = zbus::Connection::system()
                .await
                .map_err(|e| format!("D-Bus-Verbindung fehlgeschlagen: {e}"))?;
            PlatformProxy::new(&conn)
                .await
                .map_err(|e| format!("Proxy-Erstellung fehlgeschlagen: {e}"))
        })
        .await
}

pub async fn get_charge_limit() -> Result<u8, String> {
    let proxy = platform_proxy().await?;
    proxy
        .charge_control_end_threshold()
        .await
        .map_err(|e| format!("Ladelimit lesen fehlgeschlagen: {e}"))
}

pub async fn set_charge_limit(value: u8) -> Result<u8, String> {
    let proxy = platform_proxy().await?;
    proxy
        .set_charge_control_end_threshold(value)
        .await
        .map_err(|e| format!("Ladelimit setzen fehlgeschlagen: {e}"))?;
    Ok(value)
}

pub async fn get_fan_profile() -> Result<FanProfile, String> {
    let proxy = platform_proxy().await?;
    proxy
        .platform_profile()
        .await
        .map(FanProfile::from)
        .map_err(|e| format!("Lüfterprofil lesen fehlgeschlagen: {e}"))
}

pub async fn set_fan_profile(profile: FanProfile) -> Result<FanProfile, String> {
    let proxy = platform_proxy().await?;
    proxy
        .set_platform_profile(profile as u32)
        .await
        .map_err(|e| format!("Lüfterprofil setzen fehlgeschlagen: {e}"))?;
    Ok(profile)
}
