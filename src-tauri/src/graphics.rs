//! Grafikbeschleunigung der Oberfläche unter Linux (WebKitGTK).
//!
//! Je nach Treiber rendert WebKitGTK langsam oder ruckelt, bekannt etwa mit
//! NVIDIA und dem DMA-BUF-Renderer. Die Einstellung erlaubt, das zu ändern.

/// Liest die Einstellung, bevor WebKit startet: Der Kompatibilitätsmodus
/// wirkt nur als Umgebungsvariable beim Start.
#[cfg(target_os = "linux")]
pub fn prepare(identifier: &str) -> String {
    let mode = read_mode(identifier).unwrap_or_else(|| "auto".into());
    if mode == "compat" && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // Sicher, solange noch keine weiteren Threads laufen (Programmstart).
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
    mode
}

#[cfg(target_os = "linux")]
fn read_mode(identifier: &str) -> Option<String> {
    let data = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".local/share"))
        })?;
    let path = data.join(identifier).join("tsa-sync.db");
    if !path.exists() {
        return None;
    }
    let db = tsa_core::Database::open(&path).ok()?;
    let json = tsa_core::settings::get_value(&db, "app").ok()??;
    let value: serde_json::Value = serde_json::from_str(&json).ok()?;
    value["graphics"].as_str().map(str::to_owned)
}

/// Setzt die Beschleunigungsstufe am Fenster.
#[cfg(target_os = "linux")]
pub fn apply(window: &tauri::WebviewWindow, mode: &str) {
    use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
    let policy = match mode {
        "always" | "compat" => HardwareAccelerationPolicy::Always,
        "never" => HardwareAccelerationPolicy::Never,
        _ => return,
    };
    let _ = window.with_webview(move |webview| {
        if let Some(settings) = WebViewExt::settings(&webview.inner()) {
            settings.set_hardware_acceleration_policy(policy);
        }
    });
}
