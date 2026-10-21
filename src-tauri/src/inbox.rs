//! Mitteilungen im Hintergrund: zählt Ungelesenes jede Minute, meldet Neues
//! als Desktop-Benachrichtigung und schickt die Zahlen an die Oberfläche.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tsa_core::{Account, Service, Settings};
use tsa_moodle::Session;
use tsa_moodle::messages::{Counts, News};
use tsa_untis::timetable::Change;

use crate::{AppState, texts};

pub const EVENT: &str = "message-counts";
const UNTIS_MESSAGES_EVERY: Duration = Duration::from_secs(5 * 60);
const UNTIS_CHANGES_EVERY: Duration = Duration::from_secs(15 * 60);

/// Ungelesenes aus Moodle und WebUntis.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Totals {
    #[serde(flatten)]
    pub moodle: Counts,
    /// `None` ohne WebUntis-Konto.
    pub untis: Option<i64>,
}

#[derive(Default)]
struct Due {
    messages: Option<Instant>,
    changes: Option<Instant>,
}

#[derive(Default)]
pub struct Inbox {
    totals: Mutex<Totals>,
    lock: tokio::sync::Mutex<()>,
    untis: Mutex<Due>,
}

impl Inbox {
    pub fn counts(&self) -> Totals {
        self.totals
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

pub fn init(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        loop {
            let _ = check(&app).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// Wie [`check`], fragt aber WebUntis sofort ab (z. B. nach dem Lesen).
pub async fn check_untis(app: &AppHandle) -> Totals {
    {
        let state = app.state::<AppState>();
        let mut due_at = state.inbox.untis.lock().unwrap_or_else(|e| e.into_inner());
        due_at.messages = None;
    }
    check(app).await
}

fn has_account(state: &AppState, service: Service) -> bool {
    Account::get(&state.db, service).ok().flatten().is_some()
}

/// Ob seit dem letzten Mal `every` vergangen ist; merkt sich dann den Zeitpunkt.
fn due(slot: &mut Option<Instant>, every: Duration) -> bool {
    let due = slot.is_none_or(|t| t.elapsed() >= every);
    if due {
        *slot = Some(Instant::now());
    }
    due
}

/// Prüft sofort, z. B. nachdem etwas gelesen wurde. WebUntis wird seltener
/// abgefragt (Mitteilungen alle 5, Stundenplan alle 15 Minuten).
pub async fn check(app: &AppHandle) -> Totals {
    let state = app.state::<AppState>();
    let inbox = &state.inbox;
    let _guard = inbox.lock.lock().await;
    let mut totals = inbox.counts();
    let mut news = Vec::new();
    let mut changes = Vec::new();

    if has_account(&state, Service::Moodle) {
        let session = Session::new(&state.db, state.secrets.as_ref());
        if let Ok((counts, found)) = session.check_messages(&state.mail).await {
            totals.moodle = counts;
            news = found;
        }
    } else {
        totals.moodle = Counts::default();
    }

    if has_account(&state, Service::Untis) {
        let (messages_due, changes_due) = {
            let mut due_at = inbox.untis.lock().unwrap_or_else(|e| e.into_inner());
            (
                due(&mut due_at.messages, UNTIS_MESSAGES_EVERY),
                due(&mut due_at.changes, UNTIS_CHANGES_EVERY),
            )
        };
        let untis = tsa_untis::Session::new(&state.db, state.secrets.as_ref(), &state.untis);
        if messages_due {
            if let Ok((unread, found)) = untis.check_messages().await {
                totals.untis = Some(unread);
                news.extend(found.into_iter().map(|m| News {
                    kind: "untis",
                    title: m.sender,
                    body: m.subject,
                }));
            }
        }
        if changes_due {
            let today = chrono::Local::now().date_naive();
            changes = untis.check_changes(today).await.unwrap_or_default();
        }
    } else {
        totals.untis = None;
    }

    *inbox.totals.lock().unwrap_or_else(|e| e.into_inner()) = totals.clone();
    let _ = app.emit(EVENT, &totals);
    if let Ok(settings) = Settings::load(&state.db, &state.default_sync_dir) {
        if settings.notifications {
            notify(app, &settings, &news);
            notify_changes(app, &settings, &changes);
        }
    }
    totals
}

fn notify_changes(app: &AppHandle, settings: &Settings, changes: &[Change]) {
    if changes.is_empty() {
        return;
    }
    let t = texts::for_settings(settings);
    let lines: Vec<String> = changes.iter().take(4).map(|c| (t.change_line)(c)).collect();
    let mut body = lines.join("\n");
    if changes.len() > 4 {
        body.push_str(&format!("\n+{}", changes.len() - 4));
    }
    let _ = app
        .notification()
        .builder()
        .title(t.changes_title)
        .body(body)
        .show();
}

fn notify(app: &AppHandle, settings: &Settings, news: &[News]) {
    let t = texts::for_settings(settings);
    // Bei vielen Neuigkeiten auf einmal (z. B. nach längerer Pause) nur eine
    // zusammengefasste Meldung.
    if news.len() > 3 {
        let _ = app
            .notification()
            .builder()
            .title(t.messages_title)
            .body((t.messages_body)(news.len()))
            .show();
        return;
    }
    for n in news {
        let title = match n.kind {
            "chat" => (t.chat_title)(&n.title),
            "mail" => (t.mail_title)(&n.title),
            "untis" => (t.untis_title)(&n.title),
            _ => t.notification_title.to_owned(),
        };
        let body = if n.kind == "notification" {
            n.title.clone()
        } else {
            n.body.clone()
        };
        let _ = app.notification().builder().title(title).body(body).show();
    }
}
