//! Zuletzt geladene Antworten, damit Stundenplan & Co. offline sichtbar bleiben.

use rusqlite::OptionalExtension;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tsa_core::Database;

use crate::Result;

pub fn store<T: Serialize>(db: &Database, key: &str, value: &T) -> Result<i64> {
    let now = crate::now();
    db.conn().execute(
        "INSERT INTO untis_cache (key, json, fetched_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (key) DO UPDATE SET json = excluded.json, fetched_at = excluded.fetched_at",
        (key, serde_json::to_string(value)?, now),
    )?;
    Ok(now)
}

/// Gespeicherter Wert und Zeitpunkt des Abrufs.
pub fn load<T: DeserializeOwned>(db: &Database, key: &str) -> Result<Option<(T, i64)>> {
    let row: Option<(String, i64)> = db
        .conn()
        .query_row(
            "SELECT json, fetched_at FROM untis_cache WHERE key = ?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match row {
        Some((json, at)) => Ok(serde_json::from_str(&json).ok().map(|v| (v, at))),
        None => Ok(None),
    }
}

pub fn clear(db: &Database) -> Result<()> {
    db.conn().execute_batch(
        "DELETE FROM untis_cache; DELETE FROM untis_subjects; DELETE FROM untis_homework_done;
         DELETE FROM untis_local_exams;",
    )?;
    Ok(())
}
