//! Moodle über die REST-API der Mobile-App (`moodle_mobile_app`).

pub mod activities;
pub mod assign;
pub mod board;
pub mod choice;
pub mod client;
pub mod contents;
pub mod courses;
pub mod data;
pub mod deadlines;
pub mod error;
pub mod forum;
pub mod glossary;
pub mod html;
pub mod mail;
pub mod messages;
pub mod pages;
pub mod paths;
pub mod session;
pub mod sync;

pub use client::{Client, PublicConfig, SiteInfo};
pub use courses::Course;
pub use error::{Error, Result};
pub use session::Session;

use tsa_core::Database;

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE moodle_courses (
        id          INTEGER PRIMARY KEY,
        fullname    TEXT NOT NULL,
        shortname   TEXT NOT NULL,
        visible     INTEGER NOT NULL,
        enrolled    INTEGER NOT NULL DEFAULT 1,
        sync        INTEGER NOT NULL DEFAULT 0,
        start_date  INTEGER,
        end_date    INTEGER,
        last_access INTEGER
    );",
    "CREATE TABLE moodle_course_contents (
        course_id  INTEGER PRIMARY KEY,
        json       TEXT NOT NULL,
        fetched_at INTEGER NOT NULL
    );",
    "CREATE TABLE moodle_files (
        id                  INTEGER PRIMARY KEY,
        course_id           INTEGER NOT NULL,
        module_id           INTEGER NOT NULL,
        remote_path         TEXT NOT NULL,
        file_url            TEXT NOT NULL,
        remote_modified     INTEGER,
        remote_size         INTEGER NOT NULL,
        target_path         TEXT NOT NULL,
        local_path          TEXT,
        local_hash          TEXT,
        local_mtime         INTEGER,
        local_size          INTEGER,
        downloaded_modified INTEGER,
        downloaded_size     INTEGER,
        removed             INTEGER NOT NULL DEFAULT 0,
        UNIQUE (module_id, remote_path)
    );
    CREATE INDEX moodle_files_course ON moodle_files (course_id);
    CREATE TABLE moodle_file_versions (
        id         INTEGER PRIMARY KEY,
        file_id    INTEGER NOT NULL REFERENCES moodle_files (id) ON DELETE CASCADE,
        local_path TEXT NOT NULL,
        created_at INTEGER NOT NULL
    );",
    "ALTER TABLE moodle_courses ADD COLUMN progress REAL;
    ALTER TABLE moodle_courses ADD COLUMN image_url TEXT;",
    "CREATE TABLE moodle_pages (
        module_id  INTEGER PRIMARY KEY,
        json       TEXT NOT NULL,
        fetched_at INTEGER NOT NULL
    );",
    "ALTER TABLE moodle_files ADD COLUMN downloaded_at INTEGER;",
    "CREATE TABLE moodle_activity_cache (
        key        TEXT PRIMARY KEY,
        json       TEXT NOT NULL,
        fetched_at INTEGER NOT NULL
    );",
];

/// Moodle liefert Namen als HTML-Text, z. B. `Kunst &amp; Musik`.
///
/// Ergebnis ist reiner Text: Tags (auch maskierte wie `&lt;br&gt;`) fallen
/// weg, Zeilenumbrüche werden zu Leerzeichen.
pub(crate) fn plain(text: &str) -> String {
    let decoded = html_escape::decode_html_entities(text);
    // Umbrüche vorher mit Leerzeichen trennen, sonst kleben Wörter zusammen.
    let spaced = decoded
        .replace("<br", " <br")
        .replace("<BR", " <BR")
        .replace("</p>", "</p> ");
    let stripped = ammonia::Builder::empty().clean(&spaced).to_string();
    let text = html_escape::decode_html_entities(&stripped);
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Legt die Moodle-Tabellen an bzw. aktualisiert sie.
pub fn init(db: &Database) -> tsa_core::Result<()> {
    db.migrate_module("moodle", MIGRATIONS)
}

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn plain_text() {
        assert_eq!(plain(" Kunst &amp; Musik "), "Kunst & Musik");
        assert_eq!(plain("Zeile 1<br>Zeile 2"), "Zeile 1 Zeile 2");
        assert_eq!(plain("Zeile 1&lt;br /&gt;Zeile 2"), "Zeile 1 Zeile 2");
        assert_eq!(plain("<span lang=\"de\">Bio</span> 10b"), "Bio 10b");
        assert_eq!(plain("a &lt; b"), "a < b");
    }
}
