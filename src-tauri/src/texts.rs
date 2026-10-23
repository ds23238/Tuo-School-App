//! Texte, die das Backend selbst anzeigt (Tray, Benachrichtigungen).

use tsa_core::Settings;
use tsa_untis::timetable::Change;

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
    pub untis_title: fn(&str) -> String,
    pub changes_title: &'static str,
    pub change_line: fn(&Change) -> String,
}

/// `2026-10-21` → Wochentag 0 (Montag) bis 6.
fn weekday(date: &str) -> usize {
    use chrono::Datelike;
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_or(0, |d| d.weekday().num_days_from_monday() as usize)
}

/// `2026-10-21` → `21.10.`
fn day_month(date: &str) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    match parts.as_slice() {
        [_, m, d] => format!("{d}.{m}."),
        _ => date.to_owned(),
    }
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
    untis_title: |name| format!("Untis-Mitteilung von {name}"),
    changes_title: "Stundenplan geändert",
    change_line: |c| {
        const DAYS: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
        let what = if c.kind == "cancelled" {
            "entfällt".to_owned()
        } else if c.kind == "noTeacher" {
            "ohne Lehrkraft".to_owned()
        } else {
            c.detail.clone().unwrap_or_else(|| "geändert".into())
        };
        format!(
            "{} {}, {}. Std.: {} {}",
            DAYS[weekday(&c.date)],
            day_month(&c.date),
            c.period,
            c.subject,
            what
        )
    },
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
    untis_title: |name| format!("Untis message from {name}"),
    changes_title: "Timetable changed",
    change_line: |c| {
        const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        let what = if c.kind == "cancelled" {
            "cancelled".to_owned()
        } else if c.kind == "noTeacher" {
            "without teacher".to_owned()
        } else {
            c.detail.clone().unwrap_or_else(|| "changed".into())
        };
        format!(
            "{} {}, period {}: {} {}",
            DAYS[weekday(&c.date)],
            day_month(&c.date),
            c.period,
            c.subject,
            what
        )
    },
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
