//! Moodle über die REST-API der Mobile-App (`moodle_mobile_app`).

pub mod client;
pub mod courses;
pub mod error;
pub mod session;

pub use client::{Client, PublicConfig, SiteInfo};
pub use courses::Course;
pub use error::{Error, Result};
pub use session::Session;

use tsa_core::Database;

const MIGRATIONS: &[&str] = &["CREATE TABLE moodle_courses (
        id          INTEGER PRIMARY KEY,
        fullname    TEXT NOT NULL,
        shortname   TEXT NOT NULL,
        visible     INTEGER NOT NULL,
        enrolled    INTEGER NOT NULL DEFAULT 1,
        sync        INTEGER NOT NULL DEFAULT 0,
        start_date  INTEGER,
        end_date    INTEGER,
        last_access INTEGER
    );"];

/// Moodle liefert Namen als HTML-Text, z. B. `Kunst &amp; Musik`.
pub(crate) fn plain(text: &str) -> String {
    html_escape::decode_html_entities(text.trim()).into_owned()
}

/// Legt die Moodle-Tabellen an bzw. aktualisiert sie.
pub fn init(db: &Database) -> tsa_core::Result<()> {
    db.migrate_module("moodle", MIGRATIONS)
}
