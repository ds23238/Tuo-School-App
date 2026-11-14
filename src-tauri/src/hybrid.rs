//! Isolated native browser views. Remote documents never receive app commands.
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Rect, Webview, WebviewUrl};
use tokio::sync::Mutex;
use tsa_core::{Account, Service, Settings};
use url::Url;

use crate::error::{CommandError, Result};
use crate::{AppState, inbox, launch, media};

#[cfg(target_os = "linux")]
#[path = "hybrid_linux.rs"]
mod linux;

#[derive(Default)]
pub struct Browsers {
    lock: Mutex<()>,
    generation: AtomicU64,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Site {
    Moodle,
    Untis,
}
impl Site {
    fn label(self) -> &'static str {
        match self {
            Self::Moodle => "hybrid-moodle",
            Self::Untis => "hybrid-untis",
        }
    }
    fn service(self) -> Service {
        match self {
            Self::Moodle => Service::Moodle,
            Self::Untis => Service::Untis,
        }
    }
}

#[derive(Clone, Copy, serde::Deserialize)]
pub struct Bounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}
impl Bounds {
    fn rect(self) -> Result<Rect> {
        if ![self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite())
            || self.x < 0.0
            || self.y < 0.0
            || self.width < 1.0
            || self.height < 1.0
        {
            return Err(CommandError::new("browser", "Ungültige Browsergröße"));
        }
        Ok(Rect {
            position: LogicalPosition::new(self.x, self.y).into(),
            size: LogicalSize::new(self.width, self.height).into(),
        })
    }
}

async fn position(view: &Webview, bounds: Bounds) -> Result<()> {
    bounds.rect()?;
    #[cfg(target_os = "linux")]
    {
        linux::position(view, bounds).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(view.set_bounds(bounds.rect()?)?)
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
    site: Site,
    loading: bool,
    error: Option<String>,
    downloaded: bool,
}
fn status(app: &AppHandle, site: Site, loading: bool, error: Option<String>, downloaded: bool) {
    let _ = app.emit_to(
        "main",
        "hybrid-status",
        Status {
            site,
            loading,
            error,
            downloaded,
        },
    );
}
fn local(caller: &Webview) -> Result<()> {
    if caller.label() != "main" {
        return Err(CommandError::new(
            "browser",
            "Nur die lokale Oberfläche darf Browser steuern",
        ));
    }
    Ok(())
}
fn web_url(url: &Url) -> bool {
    matches!(url.scheme(), "https" | "http")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && !matches!(
            url.host_str(),
            Some("tauri.localhost" | "ipc.localhost" | "asset.localhost" | "media.localhost")
        )
}

fn file_id(app: &AppHandle, site: Site, url: &Url) -> Option<i64> {
    if !matches!(site, Site::Moodle) {
        return None;
    }
    let state = app.state::<AppState>();
    let account = Account::get(&state.db, Service::Moodle).ok()??;
    let base = Url::parse(&account.base_url).ok()?;
    tsa_moodle::file_links::resolve(&state.db, &base, url)
        .ok()
        .flatten()
}
fn open_file(app: &AppHandle, site: Site, id: i64) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = async {
            let state = app.state::<AppState>();
            // Use the same lock as the sync worker: a click cannot race a download.
            let _guard = state.background.lock.lock().await;
            let settings = Settings::load(&state.db, &state.default_sync_dir)?;
            let path = tsa_moodle::Session::new(&state.db, state.secrets.as_ref())
                .local_file(id, &settings.sync_dir)
                .await?;
            launch::path(&app, &path).await
        }
        .await;
        status(
            &app,
            site,
            false,
            result.err().map(|e: CommandError| e.to_string()),
            false,
        );
    });
}

fn navigation(app: &AppHandle, site: Site, url: &Url) -> bool {
    // The development frontend is a trusted local app origin in Tauri too.
    // Never let a remote view turn into a local app view by navigating there.
    if app
        .config()
        .build
        .dev_url
        .as_ref()
        .is_some_and(|dev| dev.origin() == url.origin())
    {
        return false;
    }
    if url.as_str() == "about:blank" {
        return true;
    }
    if let Some(id) = file_id(app, site, url) {
        open_file(app, site, id);
        return false;
    }
    if url.scheme() == "mailto" {
        let app = app.clone();
        let url = url.to_string();
        tauri::async_runtime::spawn(async move {
            let _ = launch::url(&app, &url).await;
        });
        return false;
    }
    // HTTP(S) redirects remain possible for school identity providers.
    web_url(url)
}

fn download(app: &AppHandle, site: Site, event: DownloadEvent<'_>) -> bool {
    match event {
        DownloadEvent::Requested { url, destination } => {
            if let Some(id) = file_id(app, site, &url) {
                open_file(app, site, id);
                return false;
            }
            // Unknown files use an explicit separate download directory, never
            // the sync tree. Unique destinations preserve existing downloads.
            let Ok(dir) = download_dir(app) else {
                return false;
            };
            let name = destination
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("download");
            let name = tsa_moodle::paths::component(name);
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos());
            *destination = dir.join(format!(
                "{time}-{}-{name}",
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
        }
        DownloadEvent::Finished { success, .. } => {
            status(
                app,
                site,
                false,
                (!success).then(|| "Download fehlgeschlagen".into()),
                success,
            );
        }
        _ => {}
    }
    true
}

fn popup(
    app: &AppHandle,
    site: Site,
    url: Url,
    features: tauri::webview::NewWindowFeatures,
) -> NewWindowResponse<tauri::Wry> {
    if let Some(id) = file_id(app, site, &url) {
        open_file(app, site, id);
        return NewWindowResponse::Deny;
    }
    if !web_url(&url) && url.as_str() != "about:blank" {
        return NewWindowResponse::Deny;
    }
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let label = format!(
        "{}-popup-{}",
        site.label(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let nav_app = app.clone();
    let download_app = app.clone();
    let popup_app = app.clone();
    // window_features shares the opener's WebKit context / WebView2
    // environment. Real popups retain window.opener and their browser session.
    let window = tauri::WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External(Url::parse("about:blank").unwrap()),
    )
    .title(match site {
        Site::Moodle => "Moodle",
        Site::Untis => "WebUntis",
    })
    .inner_size(1000., 720.)
    .window_features(features)
    .on_navigation(move |url| navigation(&nav_app, site, url))
    .on_download(move |_, event| download(&download_app, site, event))
    .on_new_window(move |url, features| popup(&popup_app, site, url, features))
    .build();
    match window {
        Ok(window) => NewWindowResponse::Create { window },
        Err(e) => {
            status(app, site, false, Some(e.to_string()), false);
            NewWindowResponse::Deny
        }
    }
}

fn builder(app: &AppHandle, site: Site, profile: std::path::PathBuf) -> WebviewBuilder<tauri::Wry> {
    let nav_app = app.clone();
    let popup_app = app.clone();
    let download_app = app.clone();
    let page_app = app.clone();
    WebviewBuilder::new(
        site.label(),
        WebviewUrl::External(Url::parse("about:blank").unwrap()),
    )
    .data_directory(profile)
    .on_navigation(move |url| navigation(&nav_app, site, url))
    .on_new_window(move |url, features| popup(&popup_app, site, url, features))
    .on_download(move |_, event| download(&download_app, site, event))
    .on_page_load(move |_, payload| {
        if payload.url().as_str() != "about:blank" {
            status(
                &page_app,
                site,
                payload.event() == PageLoadEvent::Started,
                None,
                false,
            );
            if payload.event() == PageLoadEvent::Finished {
                let app = page_app.clone();
                tauri::async_runtime::spawn(async move {
                    inbox::check(&app).await;
                });
            }
        }
    })
}

fn download_dir(app: &AppHandle) -> Result<std::path::PathBuf> {
    let dir = app.path().download_dir()?.join("TSA-Sync");
    std::fs::create_dir_all(&dir).map_err(|e| CommandError::new("browser", e.to_string()))?;
    Ok(dir)
}

fn profile(app: &AppHandle, site: Site) -> Result<std::path::PathBuf> {
    let state = app.state::<AppState>();
    let key = format!("hybrid.profile.{}", site.label());
    let generation = tsa_core::settings::get_value(&state.db, &key)?.unwrap_or_else(|| "0".into());
    Ok(app
        .path()
        .app_data_dir()?
        .join("browser")
        .join(site.label())
        .join(generation))
}

/// Clear both cookie storage and the view before changing/removing an account.
pub async fn reset(app: &AppHandle, site: Site) -> Result<()> {
    let browsers = app.state::<Browsers>();
    browsers.generation.fetch_add(1, Ordering::SeqCst);
    let _guard = browsers.lock.lock().await;
    let previous_profile = profile(app, site)?;
    let prefix = format!("{}-popup-", site.label());
    for (label, window) in app.webview_windows() {
        if label.starts_with(&prefix) {
            window.close()?;
        }
    }
    if let Some(view) = app.get_webview(site.label()) {
        view.hide()?;
        view.clear_all_browsing_data()?;
        view.close()?;
    } else if previous_profile.exists() {
        // Create an invisible view so a persisted profile from an earlier run
        // is cleared as well. No remote URL is loaded during this operation.
        let profile = previous_profile.clone();
        let window = app
            .get_window("main")
            .ok_or_else(|| CommandError::new("browser", "Hauptfenster fehlt"))?;
        let view = window.add_child(
            builder(app, site, profile),
            LogicalPosition::new(-10_000, -10_000),
            LogicalSize::new(1, 1),
        )?;
        view.hide()?;
        view.clear_all_browsing_data()?;
        view.close()?;
    }
    let generation = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| CommandError::new("browser", e.to_string()))?
        .as_nanos()
        .to_string();
    let state = app.state::<AppState>();
    tsa_core::settings::set_value(
        &state.db,
        &format!("hybrid.profile.{}", site.label()),
        &generation,
    )?;
    // Engines clear storage asynchronously. Never reuse the old directory,
    // even if Windows still has a file handle open during cleanup.
    let _ = std::fs::remove_dir_all(previous_profile);

    Ok(())
}

#[tauri::command]
pub async fn hybrid_open(
    app: AppHandle,
    caller: Webview,
    site: Site,
    target: Option<String>,
    course_id: Option<i64>,
    bounds: Bounds,
) -> Result<()> {
    local(&caller)?;
    bounds.rect()?;
    let browsers = app.state::<Browsers>();
    let generation = browsers.generation.fetch_add(1, Ordering::SeqCst) + 1;
    let _guard = browsers.lock.lock().await;
    let state = app.state::<AppState>();
    let account = Account::get(&state.db, site.service())?
        .ok_or_else(|| CommandError::new("browser", "Nicht angemeldet"))?;
    let base =
        Url::parse(&account.base_url).map_err(|e| CommandError::new("browser", e.to_string()))?;
    let target = match site {
        Site::Moodle => {
            let url = base
                .join(target.as_deref().unwrap_or("my/"))
                .map_err(|e| CommandError::new("browser", e.to_string()))?;
            if !web_url(&url)
                || url.origin() != base.origin()
                || !url.path().starts_with(base.path())
            {
                return Err(CommandError::new("browser", "Ungültiges Moodle-Ziel"));
            }
            let session = tsa_moodle::Session::new(&state.db, state.secrets.as_ref());
            if let Some(id) = course_id {
                let settings = Settings::load(&state.db, &state.default_sync_dir)?;
                // Index files even for courses that have not been selected for sync.
                let _ = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    session.refresh_course(id, &media::urls(), &settings.sync_dir),
                )
                .await;
            }
            url
        }
        Site::Untis => tsa_untis::Session::new(&state.db, state.secrets.as_ref(), &state.untis)
            .web_url()?
            .parse()
            .map_err(|e: url::ParseError| CommandError::new("browser", e.to_string()))?,
    };
    for site in [Site::Moodle, Site::Untis] {
        if let Some(view) = app.get_webview(site.label()) {
            view.hide()?;
        }
    }
    let existing = app.get_webview(site.label());
    let fresh = existing.is_none();
    let view = match existing {
        Some(view) => view,
        None => {
            let profile = profile(&app, site)?;
            let window = app
                .get_window("main")
                .ok_or_else(|| CommandError::new("browser", "Hauptfenster fehlt"))?;
            window.add_child(
                builder(&app, site, profile),
                LogicalPosition::new(-10_000, -10_000),
                LogicalSize::new(1, 1),
            )?
        }
    };
    #[cfg(target_os = "linux")]
    if let Ok(settings) = Settings::load(&state.db, &state.default_sync_dir) {
        crate::graphics::apply_browser(&view, &settings.graphics);
    }
    position(&view, bounds).await?;
    let mut url = target;
    if fresh && matches!(site, Site::Moodle) {
        let session = tsa_moodle::Session::new(&state.db, state.secrets.as_ref());
        if let Ok(Ok(link)) = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            session.autologin_url(url.as_str()),
        )
        .await
        {
            if let Ok(link) = Url::parse(&link) {
                if web_url(&link) {
                    url = link;
                }
            }
        }
    }
    if fresh && matches!(site, Site::Untis) {
        // Share the backend session with the browser without exposing it to Svelte.
        // A manual website login remains available if the school requires more.
        let session = tsa_untis::Session::new(&state.db, state.secrets.as_ref(), &state.untis);
        if let Ok(Ok(cookies)) =
            tokio::time::timeout(std::time::Duration::from_secs(8), session.browser_cookies()).await
        {
            for (name, value) in cookies {
                let http_only = name == "JSESSIONID";
                let cookie = tauri::webview::Cookie::build((name, value))
                    .domain(base.host_str().unwrap_or_default().to_owned())
                    .path("/WebUntis")
                    .http_only(http_only)
                    .secure(base.scheme() == "https")
                    .build();
                view.set_cookie(cookie)?;
            }
        }
    }
    if browsers.generation.load(Ordering::SeqCst) != generation {
        view.hide()?;
        return Ok(());
    }
    view.navigate(url)?;
    view.show()?;
    Ok(())
}

#[tauri::command]
pub async fn hybrid_bounds(
    app: AppHandle,
    caller: Webview,
    site: Site,
    bounds: Bounds,
) -> Result<()> {
    local(&caller)?;
    if let Some(view) = app.get_webview(site.label()) {
        position(&view, bounds).await?;
    }
    Ok(())
}
#[tauri::command]
pub async fn hybrid_hide(app: AppHandle, caller: Webview, site: Site) -> Result<()> {
    local(&caller)?;
    let browsers = app.state::<Browsers>();
    browsers.generation.fetch_add(1, Ordering::SeqCst);
    if let Some(view) = app.get_webview(site.label()) {
        view.hide()?;
    }
    let _guard = browsers.lock.lock().await;
    if let Some(view) = app.get_webview(site.label()) {
        view.hide()?;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        inbox::check(&app).await;
    });
    Ok(())
}
#[tauri::command]
pub async fn hybrid_action(
    app: AppHandle,
    caller: Webview,
    site: Site,
    action: String,
) -> Result<()> {
    local(&caller)?;
    let view = app
        .get_webview(site.label())
        .ok_or_else(|| CommandError::new("browser", "Browser ist geschlossen"))?;
    match action.as_str() {
        "back" => view.eval("history.back()")?,
        "forward" => view.eval("history.forward()")?,
        "reload" => view.reload()?,
        "external" => {
            let url = view.url()?;
            if web_url(&url) {
                launch::url(&app, url.as_str()).await?;
            }
        }
        "downloads" => launch::path(&app, &download_dir(&app)?).await?,
        _ => return Err(CommandError::new("browser", "Unbekannte Browseraktion")),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_navigation_rejects_local_protocols_and_credentials() {
        for url in [
            "file:///etc/passwd",
            "tauri://localhost",
            "http://ipc.localhost/a",
            "javascript:alert(1)",
            "https://user:password@school.example",
        ] {
            assert!(!web_url(&Url::parse(url).unwrap()), "{url}");
        }
        assert!(web_url(
            &Url::parse("https://school.example/login").unwrap()
        ));
    }
    #[test]
    fn bounds_must_be_finite_and_visible() {
        assert!(
            Bounds {
                x: 230.,
                y: 100.,
                width: 800.,
                height: 500.
            }
            .rect()
            .is_ok()
        );
        assert!(
            Bounds {
                x: f64::NAN,
                y: 0.,
                width: 800.,
                height: 500.
            }
            .rect()
            .is_err()
        );
    }
}
