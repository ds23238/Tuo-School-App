use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::{Database, Result};

const KEY: &str = "app";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// `None` = Systemsprache.
    pub locale: Option<String>,
    pub sync_dir: PathBuf,
}

impl Settings {
    pub fn defaults(default_sync_dir: &Path) -> Self {
        Self {
            locale: None,
            sync_dir: default_sync_dir.to_path_buf(),
        }
    }

    /// Lädt die Einstellungen; fehlende Felder bekommen Standardwerte.
    pub fn load(db: &Database, default_sync_dir: &Path) -> Result<Self> {
        let stored: Option<String> = db
            .conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [KEY], |r| {
                r.get(0)
            })
            .optional()?;
        let mut value = serde_json::to_value(Self::defaults(default_sync_dir))?;
        if let (Some(stored), Some(fields)) = (stored, value.as_object_mut()) {
            if let serde_json::Value::Object(stored) = serde_json::from_str(&stored)? {
                fields.extend(stored);
            }
        }
        Ok(serde_json::from_value(value)?)
    }

    pub fn save(&self, db: &Database) -> Result<()> {
        db.conn().execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            (KEY, serde_json::to_string(self)?),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_empty() {
        let db = Database::open_in_memory().unwrap();
        let s = Settings::load(&db, Path::new("/sync")).unwrap();
        assert_eq!(s, Settings::defaults(Path::new("/sync")));
    }

    #[test]
    fn roundtrip() {
        let db = Database::open_in_memory().unwrap();
        let s = Settings {
            locale: Some("en".into()),
            sync_dir: "/elsewhere".into(),
        };
        s.save(&db).unwrap();
        assert_eq!(Settings::load(&db, Path::new("/sync")).unwrap(), s);
    }

    #[test]
    fn missing_fields_get_defaults() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO settings (key, value) VALUES ('app', '{\"locale\":\"de\"}')",
                [],
            )
            .unwrap();
        let s = Settings::load(&db, Path::new("/sync")).unwrap();
        assert_eq!(s.locale.as_deref(), Some("de"));
        assert_eq!(s.sync_dir, Path::new("/sync"));
    }
}
