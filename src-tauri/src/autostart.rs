//! Start beim Anmelden. Im Flatpak geht das nur über das Hintergrund-Portal,
//! das sich den Zustand nicht abfragen lässt; er wird deshalb gespeichert.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tsa_core::settings::{get_value, set_value};

use crate::AppState;
use crate::error::{CommandError, Result};

const FLATPAK_KEY: &str = "autostart";

fn flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

pub fn enabled(app: &AppHandle, state: &AppState) -> Result<bool> {
    if flatpak() {
        return Ok(get_value(&state.db, FLATPAK_KEY)?.as_deref() == Some("1"));
    }
    app.autolaunch()
        .is_enabled()
        .map_err(|e| CommandError::new("autostart", e.to_string()))
}

pub async fn set(app: &AppHandle, state: &AppState, enabled: bool) -> Result<()> {
    if flatpak() {
        portal(enabled).await?;
        set_value(&state.db, FLATPAK_KEY, if enabled { "1" } else { "0" })?;
        return Ok(());
    }
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| CommandError::new("autostart", e.to_string()))
}

#[cfg(target_os = "linux")]
async fn portal(enabled: bool) -> Result<()> {
    use crate::MINIMIZED;
    use ashpd::desktop::background::Background;
    let error = |e: ashpd::Error| CommandError::new("autostart", e.to_string());
    let response = Background::request()
        .auto_start(enabled)
        .command(["tsa-sync", MINIMIZED])
        .dbus_activatable(false)
        .send()
        .await
        .map_err(error)?
        .response()
        .map_err(error)?;
    if enabled && !response.auto_start() {
        return Err(CommandError::new("autostartDenied", ""));
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
async fn portal(_enabled: bool) -> Result<()> {
    Ok(())
}
