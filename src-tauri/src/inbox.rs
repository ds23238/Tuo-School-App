//! Mitteilungen im Hintergrund: zählt Ungelesenes jede Minute, meldet Neues
//! als Desktop-Benachrichtigung und schickt die Zahlen an die Oberfläche.

use std::sync::Mutex;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tsa_core::{Account, Service, Settings};
use tsa_moodle::Session;
use tsa_moodle::messages::{Counts, News};

use crate::{AppState, texts};

pub const EVENT: &str = "message-counts";

#[derive(Default)]
pub struct Inbox {
    counts: Mutex<Counts>,
    lock: tokio::sync::Mutex<()>,
}

impl Inbox {
    pub fn counts(&self) -> Counts {
        self.counts
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

/// Prüft sofort, z. B. nachdem etwas gelesen wurde.
pub async fn check(app: &AppHandle) -> Counts {
    let state = app.state::<AppState>();
    let inbox = &state.inbox;
    let _guard = inbox.lock.lock().await;
    let signed_in = Account::get(&state.db, Service::Moodle)
        .ok()
        .flatten()
        .is_some();
    if !signed_in {
        return Counts::default();
    }
    let session = Session::new(&state.db, state.secrets.as_ref());
    let Ok((counts, news)) = session.check_messages(&state.mail).await else {
        return inbox.counts();
    };
    *inbox.counts.lock().unwrap_or_else(|e| e.into_inner()) = counts.clone();
    let _ = app.emit(EVENT, &counts);
    if let Ok(settings) = Settings::load(&state.db, &state.default_sync_dir) {
        if settings.notifications {
            notify(app, &settings, &news);
        }
    }
    counts
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
