//! Dateien und Links mit dem Standardprogramm öffnen.
//!
//! Unter Linux starten wir `xdg-open` selbst: Aus einer AppImage heraus erbt
//! es sonst Variablen wie `GTK_PATH` oder `GIO_MODULE_DIR`, die auf die
//! Bibliotheken der App zeigen. Das geöffnete Programm (PDF-Betrachter,
//! Browser) startet dann oft nicht, ohne dass ein Fehler zurückkommt.

use std::ffi::OsStr;
use std::path::Path;

use tauri::AppHandle;

use crate::error::{CommandError, Result};

fn failed(message: impl Into<String>) -> CommandError {
    CommandError::new("open", message)
}

pub async fn path(app: &AppHandle, path: &Path) -> Result<()> {
    open(app, path.as_os_str()).await
}

/// Öffnet nur Web- und Mail-Links, keine beliebigen Programme oder Dateien.
pub async fn url(app: &AppHandle, url: &str) -> Result<()> {
    let scheme = url
        .split(':')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(scheme.as_str(), "http" | "https" | "mailto") {
        return Err(failed(format!("Link wird nicht geöffnet: {url}")));
    }
    open(app, OsStr::new(url)).await
}

#[cfg(not(target_os = "linux"))]
async fn open(app: &AppHandle, target: &OsStr) -> Result<()> {
    use tauri_plugin_opener::OpenerExt;
    Ok(app
        .opener()
        .open_path(target.to_string_lossy(), None::<&str>)?)
}

#[cfg(target_os = "linux")]
async fn open(_app: &AppHandle, target: &OsStr) -> Result<()> {
    use std::process::Stdio;
    use std::time::Duration;

    let env = clean_env(std::env::vars_os());
    let mut last_error = None;
    for (program, args) in [("xdg-open", &[][..]), ("gio", &["open"][..])] {
        let child = tokio::process::Command::new(program)
            .args(args)
            .arg(target)
            .env_clear()
            .envs(env.iter().map(|(k, v)| (k, v)))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        let mut child = match child {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                last_error = Some(failed(format!("{program} nicht gefunden")));
                continue;
            }
            Err(e) => return Err(failed(e.to_string())),
        };
        // xdg-open kehrt normalerweise sofort zurück; manche Umgebungen
        // warten, bis das Programm geschlossen wird. Dann gilt es als offen.
        return match tokio::time::timeout(Duration::from_secs(5), child.wait()).await {
            Ok(Ok(status)) if !status.success() => Err(failed(format!(
                "Kein Programm zum Öffnen gefunden ({program}: {status})"
            ))),
            Ok(Err(e)) => Err(failed(e.to_string())),
            _ => Ok(()),
        };
    }
    Err(last_error.unwrap_or_else(|| failed("Kein Programm zum Öffnen gefunden")))
}

/// Variablen, die eine AppImage für ihre eigenen Bibliotheken setzt.
#[cfg(any(target_os = "linux", test))]
const APPIMAGE_VARS: &[&str] = &[
    "APPDIR",
    "APPIMAGE",
    "APPIMAGE_EXTRACT_AND_RUN",
    "ARGV0",
    "OWD",
    "GTK_THEME",
    "GTK_DATA_PREFIX",
    "GTK_EXE_PREFIX",
    "GTK_PATH",
    "GTK_IM_MODULE_FILE",
    "GDK_PIXBUF_MODULE_FILE",
    "GSETTINGS_SCHEMA_DIR",
    "GI_TYPELIB_PATH",
    "GIO_MODULE_DIR",
    "GIO_EXTRA_MODULES",
    "LD_PRELOAD",
    "PYTHONHOME",
    "PYTHONPATH",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];

/// Umgebung ohne die Spuren einer AppImage; außerhalb einer AppImage
/// unverändert.
#[cfg(any(target_os = "linux", test))]
fn clean_env(
    vars: impl Iterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
    let vars: Vec<_> = vars.collect();
    let Some(appdir) = vars
        .iter()
        .find(|(k, _)| k == "APPDIR")
        .map(|(_, v)| v.to_string_lossy().trim_end_matches('/').to_owned())
        .filter(|d| !d.is_empty())
    else {
        return vars;
    };
    let inside = |entry: &str| entry == appdir || entry.starts_with(&format!("{appdir}/"));
    vars.into_iter()
        .filter(|(k, _)| !APPIMAGE_VARS.iter().any(|v| k == v))
        .filter_map(|(k, v)| {
            if !matches!(
                k.to_str(),
                Some("PATH" | "XDG_DATA_DIRS" | "LD_LIBRARY_PATH")
            ) {
                return Some((k, v));
            }
            let kept: Vec<String> = v
                .to_string_lossy()
                .split(':')
                .filter(|e| !e.is_empty() && !inside(e))
                .map(str::to_owned)
                .collect();
            (!kept.is_empty()).then(|| (k, kept.join(":").into()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs
            .iter()
            .map(|(k, v)| (OsString::from(k), OsString::from(v)))
            .collect()
    }

    #[test]
    fn untouched_outside_appimage() {
        let vars = env(&[("PATH", "/usr/bin"), ("GTK_THEME", "Breeze")]);
        assert_eq!(clean_env(vars.clone().into_iter()), vars);
    }

    #[test]
    fn strips_appimage_traces() {
        let vars = env(&[
            ("APPDIR", "/tmp/.mount_TSA/"),
            ("GTK_PATH", "/tmp/.mount_TSA//usr/lib/gtk-3.0"),
            ("GIO_MODULE_DIR", "/tmp/.mount_TSA//usr/lib/gio/modules"),
            ("PATH", "/tmp/.mount_TSA/usr/bin:/usr/bin:/bin"),
            ("XDG_DATA_DIRS", "/tmp/.mount_TSA/usr/share:/usr/share:"),
            ("LD_LIBRARY_PATH", "/tmp/.mount_TSA/usr/lib"),
            ("HOME", "/home/anna"),
        ]);
        let cleaned = clean_env(vars.into_iter());
        assert_eq!(
            cleaned,
            env(&[
                ("PATH", "/usr/bin:/bin"),
                ("XDG_DATA_DIRS", "/usr/share"),
                ("HOME", "/home/anna"),
            ])
        );
    }
}
