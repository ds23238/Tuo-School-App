//! Texte, die das Backend selbst anzeigt (Tray, Benachrichtigungen).

use tsa_core::Settings;

pub struct Texts {
    pub open: &'static str,
    pub sync_now: &'static str,
    pub auto_sync: &'static str,
    pub quit: &'static str,
    pub new_files_title: &'static str,
    pub new_files_body: fn(usize, &str) -> String,
    pub updates_title: &'static str,
    pub updates_body: fn(i64) -> String,
    pub chat_title: fn(&str) -> String,
    pub mail_title: fn(&str) -> String,
    pub notification_title: &'static str,
    pub messages_title: &'static str,
    pub messages_body: fn(usize) -> String,
}

const DE: Texts = Texts {
    open: "Öffnen",
    sync_now: "Jetzt synchronisieren",
    auto_sync: "Automatisch synchronisieren",
    quit: "Beenden",
    new_files_title: "Neue Dateien",
    new_files_body: |n, courses| {
        if n == 1 {
            format!("1 neue Datei in {courses}")
        } else {
            format!("{n} neue Dateien in {courses}")
        }
    },
    updates_title: "Neue Versionen verfügbar",
    updates_body: |n| {
        if n == 1 {
            "Eine Datei wurde online geändert.".into()
        } else {
            format!("{n} Dateien wurden online geändert.")
        }
    },
    chat_title: |name| format!("Nachricht von {name}"),
    mail_title: |name| format!("Mail von {name}"),
    notification_title: "Moodle-Benachrichtigung",
    messages_title: "Neue Mitteilungen",
    messages_body: |n| format!("{n} neue Nachrichten, Mails und Benachrichtigungen"),
};

const EN: Texts = Texts {
    open: "Open",
    sync_now: "Sync now",
    auto_sync: "Sync automatically",
    quit: "Quit",
    new_files_title: "New files",
    new_files_body: |n, courses| {
        if n == 1 {
            format!("1 new file in {courses}")
        } else {
            format!("{n} new files in {courses}")
        }
    },
    updates_title: "New versions available",
    updates_body: |n| {
        if n == 1 {
            "A file was changed online.".into()
        } else {
            format!("{n} files were changed online.")
        }
    },
    chat_title: |name| format!("Message from {name}"),
    mail_title: |name| format!("Mail from {name}"),
    notification_title: "Moodle notification",
    messages_title: "New messages",
    messages_body: |n| format!("{n} new messages, mails and notifications"),
};

/// Sprache aus den Einstellungen, sonst die des Systems.
pub fn for_settings(settings: &Settings) -> &'static Texts {
    let locale = settings
        .locale
        .clone()
        .or_else(sys_locale::get_locale)
        .unwrap_or_default();
    if locale.to_ascii_lowercase().starts_with("de") {
        &DE
    } else {
        &EN
    }
}
