//! Synchronisation im Hintergrund und auf Knopfdruck. Es läuft immer nur
//! ein Sync gleichzeitig; den Stand bekommt die Oberfläche als Event.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tsa_core::settings::{get_value, set_value};
use tsa_core::{Account, Service, Settings};
use tsa_moodle::session::CourseSync;
use tsa_moodle::{Course, Session};

use crate::{AppState, texts, tray};

const LAST_SYNC: &str = "last_sync";
pub const EVENT: &str = "sync-status";

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub running: bool,
    /// Unix-Zeit des letzten abgeschlossenen Syncs.
    pub last_at: Option<i64>,
    pub error: Option<String>,
    pub results: Vec<CourseSync>,
}

#[derive(Default)]
pub struct Background {
    lock: tokio::sync::Mutex<()>,
    status: Mutex<Status>,
    /// Weckt die Schleife, z. B. nach geänderten Einstellungen.
    wake: tokio::sync::Notify,
}

impl Background {
    pub fn status(&self) -> Status {
        self.status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn update(&self, app: &AppHandle, change: impl FnOnce(&mut Status)) {
        let status = {
            let mut status = self.status.lock().unwrap_or_else(|e| e.into_inner());
            change(&mut status);
            status.clone()
        };
        let _ = app.emit(EVENT, &status);
        tray::refresh(app);
    }

    pub fn wake(&self) {
        self.wake.notify_one();
    }
}

pub fn init(app: &AppHandle) {
    let state = app.state::<AppState>();
    let last_at = get_value(&state.db, LAST_SYNC)
        .ok()
        .flatten()
        .and_then(|v| v.parse().ok());
    state.background.status.lock().unwrap().last_at = last_at;
    let app = app.clone();
    tauri::async_runtime::spawn(async move { schedule(app).await });
}

async fn schedule(app: AppHandle) {
    // Kurz warten, damit der Start nicht mit dem ersten Sync konkurriert.
    tokio::time::sleep(Duration::from_secs(10)).await;
    loop {
        let state = app.state::<AppState>();
        let settings = Settings::load(&state.db, &state.default_sync_dir).ok();
        let interval = settings
            .as_ref()
            .map_or(30, |s| s.sync_interval_minutes.max(5));
        let due = settings.as_ref().is_some_and(|s| s.sync_enabled) && {
            let last = state.background.status().last_at.unwrap_or(0);
            tsa_moodle::contents::now() - last >= i64::from(interval) * 60
        };
        let signed_in = Account::get(&state.db, Service::Moodle)
            .ok()
            .flatten()
            .is_some();
        if due && signed_in {
            let _ = run(&app).await;
        }
        // Spätestens nach einer Minute neu prüfen; Einstellungen können sich ändern.
        let _ =
            tokio::time::timeout(Duration::from_secs(60), state.background.wake.notified()).await;
    }
}

/// Führt einen Sync aller ausgewählten Kurse aus. Läuft schon einer, wird
/// auf ihn gewartet und danach erneut synchronisiert.
pub async fn run(app: &AppHandle) -> Result<Vec<CourseSync>, String> {
    let state = app.state::<AppState>();
    let background = &state.background;
    let _guard = background.lock.lock().await;
    background.update(app, |s| {
        s.running = true;
        s.error = None;
    });

    let settings = Settings::load(&state.db, &state.default_sync_dir).map_err(|e| e.to_string());
    let session = Session::new(&state.db, state.secrets.as_ref());
    let updates_before = total_updates(&session);
    let result = match &settings {
        Ok(settings) => session
            .sync_all(&settings.sync_dir)
            .await
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.clone()),
    };

    let now = tsa_moodle::contents::now();
    let error = match &result {
        Ok(results) => results.iter().find_map(|r| r.error.clone()),
        Err(e) => Some(e.clone()),
    };
    if result.is_ok() {
        let _ = set_value(&state.db, LAST_SYNC, &now.to_string());
    }
    background.update(app, |s| {
        s.running = false;
        s.error = error;
        if let Ok(results) = &result {
            s.last_at = Some(now);
            s.results = results.clone();
        }
    });

    if let (Ok(settings), Ok(results)) = (&settings, &result) {
        if settings.notifications {
            notify(
                app,
                settings,
                results,
                total_updates(&session) - updates_before,
            );
        }
    }
    result
}

/// Synchronisiert einen einzelnen Kurs, ohne einem laufenden Sync in die
/// Quere zu kommen.
pub async fn run_course(app: &AppHandle, course_id: i64) -> Result<usize, String> {
    let state = app.state::<AppState>();
    let _guard = state.background.lock.lock().await;
    let settings = Settings::load(&state.db, &state.default_sync_dir).map_err(|e| e.to_string())?;
    Session::new(&state.db, state.secrets.as_ref())
        .sync_course(course_id, &settings.sync_dir)
        .await
        .map_err(|e| e.to_string())
}

fn total_updates(session: &Session) -> i64 {
    session
        .file_stats()
        .map(|s| s.values().map(|f| f.updates).sum())
        .unwrap_or(0)
}

fn notify(app: &AppHandle, settings: &Settings, results: &[CourseSync], new_updates: i64) {
    let t = texts::for_settings(settings);
    let downloaded: usize = results.iter().map(|r| r.downloaded).sum();
    if downloaded > 0 {
        let state = app.state::<AppState>();
        let names: HashMap<i64, String> = Course::list(&state.db)
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.full_name))
            .collect();
        let courses: Vec<&str> = results
            .iter()
            .filter(|r| r.downloaded > 0)
            .filter_map(|r| names.get(&r.course_id).map(String::as_str))
            .collect();
        let _ = app
            .notification()
            .builder()
            .title(t.new_files_title)
            .body((t.new_files_body)(downloaded, &courses.join(", ")))
            .show();
    }
    if new_updates > 0 {
        let _ = app
            .notification()
            .builder()
            .title(t.updates_title)
            .body((t.updates_body)(new_updates))
            .show();
    }
}
