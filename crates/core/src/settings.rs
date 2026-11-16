use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::{Database, Result};

const KEY: &str = "app";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    Native,
    Web,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// `None` = Systemsprache.
    pub locale: Option<String>,
    pub sync_dir: PathBuf,
    /// Automatisch im Hintergrund synchronisieren, solange die App läuft.
    pub sync_enabled: bool,
    pub sync_interval_minutes: u32,
    /// Systembenachrichtigungen bei neuen Dateien.
    pub notifications: bool,
    /// Grafikbeschleunigung der Oberfläche (nur Linux): `auto`, `always`,
    /// `never` oder `compat` (ohne DMA-BUF, hilft bei NVIDIA). Wirkt nach
    /// einem Neustart.
    pub graphics: String,
    /// Farbschema: `system`, `light` oder `dark`.
    pub theme: String,
    /// Eigene Farben je Untis-Fach (Kürzel → `#rrggbb`).
    pub subject_colors: BTreeMap<String, String>,
    pub course_display: DisplayMode,
    pub moodle_messages_display: DisplayMode,
    pub untis_messages_display: DisplayMode,
}

impl Settings {
    pub fn defaults(default_sync_dir: &Path) -> Self {
        Self {
            locale: None,
            sync_dir: default_sync_dir.to_path_buf(),
            sync_enabled: true,
            sync_interval_minutes: 30,
            notifications: true,
            graphics: "auto".into(),
            theme: "system".into(),
            subject_colors: BTreeMap::new(),
            course_display: DisplayMode::Web,
            moodle_messages_display: DisplayMode::Native,
            untis_messages_display: DisplayMode::Web,
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

/// Einzelner gespeicherter Wert außerhalb der Einstellungen, z. B. der
/// Zeitpunkt des letzten Syncs. Getrennt, damit das Speichern der
/// Einstellungen aus der Oberfläche ihn nicht überschreibt.
pub fn get_value(db: &Database, key: &str) -> Result<Option<String>> {
    Ok(db
        .conn()
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn set_value(db: &Database, key: &str, value: &str) -> Result<()> {
    db.conn().execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        (key, value),
    )?;
    Ok(())
}

pub fn delete_value(db: &Database, key: &str) -> Result<()> {
    db.conn()
        .execute("DELETE FROM settings WHERE key = ?1", [key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_preferences_upgrade_old_settings_and_persist() {
        let db = Database::open_in_memory().unwrap();
        set_value(&db, KEY, r#"{"theme":"dark"}"#).unwrap();
        let mut settings = Settings::load(&db, Path::new("/sync")).unwrap();
        assert_eq!(settings.course_display, DisplayMode::Web);
        assert_eq!(settings.moodle_messages_display, DisplayMode::Native);
        assert_eq!(settings.untis_messages_display, DisplayMode::Web);
        settings.course_display = DisplayMode::Native;
        settings.moodle_messages_display = DisplayMode::Web;
        settings.save(&db).unwrap();
        let loaded = Settings::load(&db, Path::new("/sync")).unwrap();
        assert_eq!(loaded, settings);
    }

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
            sync_enabled: false,
            sync_interval_minutes: 60,
            notifications: false,
            graphics: "never".into(),
            theme: "dark".into(),
            subject_colors: [("M".to_owned(), "#ff0000".to_owned())].into(),
            course_display: DisplayMode::Native,
            moodle_messages_display: DisplayMode::Web,
            untis_messages_display: DisplayMode::Native,
        };
        s.save(&db).unwrap();
        assert_eq!(Settings::load(&db, Path::new("/sync")).unwrap(), s);
    }

    #[test]
    fn values_are_separate() {
        let db = Database::open_in_memory().unwrap();
        set_value(&db, "last_sync", "42").unwrap();
        Settings::defaults(Path::new("/s")).save(&db).unwrap();
        assert_eq!(get_value(&db, "last_sync").unwrap().as_deref(), Some("42"));
        assert_eq!(get_value(&db, "nope").unwrap(), None);
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
