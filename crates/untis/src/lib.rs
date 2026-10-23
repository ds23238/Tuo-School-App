//! WebUntis: Stundenplan, Vertretungen, Hausaufgaben, Prüfungen, Mitteilungen.

pub mod cache;
pub mod client;
pub mod error;
pub mod homework;
pub mod messages;
pub mod session;
pub mod timetable;
pub mod totp;

pub use client::{Person, School, search_schools};
pub use error::{Error, Result};
pub use session::{LoginResult, Session, UntisState};

use tsa_core::Database;

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE untis_cache (
        key        TEXT PRIMARY KEY,
        json       TEXT NOT NULL,
        fetched_at INTEGER NOT NULL
    );
    CREATE TABLE untis_subjects (
        name      TEXT PRIMARY KEY,
        long_name TEXT NOT NULL
    );
    CREATE TABLE untis_homework_done (
        id INTEGER PRIMARY KEY
    );",
    "CREATE TABLE untis_local_exams (
        id      INTEGER PRIMARY KEY AUTOINCREMENT,
        date    TEXT NOT NULL,
        start   TEXT,
        subject TEXT,
        name    TEXT NOT NULL,
        note    TEXT
    );",
];

/// Legt die Tabellen des Moduls an bzw. aktualisiert sie.
pub fn init(db: &Database) -> tsa_core::Result<()> {
    db.migrate_module("untis", MIGRATIONS)
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Untis-Datum `20261021` ↔ ISO `2026-10-21`.
pub(crate) fn iso_date(untis: i64) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        untis / 10000,
        untis / 100 % 100,
        untis % 100
    )
}

/// Untis-Uhrzeit `745` → `07:45`.
pub(crate) fn time(untis: i64) -> String {
    format!("{:02}:{:02}", untis / 100, untis % 100)
}

pub(crate) fn untis_date(date: chrono::NaiveDate) -> String {
    date.format("%Y%m%d").to_string()
}

/// Reiner Text aus HTML, Absätze bleiben als Zeilenumbrüche.
pub(crate) fn plain(html: &str) -> String {
    let marked = html
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n");
    let text = ammonia::Builder::empty().clean(&marked).to_string();
    html_escape::decode_html_entities(text.trim()).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(iso_date(20261021), "2026-10-21");
        assert_eq!(time(745), "07:45");
        assert_eq!(time(1310), "13:10");
        assert_eq!(
            plain("<p>Hallo &amp; tschüss</p><p>B</p>"),
            "Hallo & tschüss\nB"
        );
    }
}
