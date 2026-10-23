//! Gemeinsames für Aktivitäten, die die App selbst anzeigt (Forum,
//! Datenbank, Abstimmung, Glossar): Zwischenspeicher für offline und
//! Dateien zum Öffnen.

use std::future::Future;
use std::path::{Path, PathBuf};

use rusqlite::OptionalExtension;
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::contents::{self, RemoteModule, now};
use crate::{Error, Result, Session, paths};

/// Daten einer Aktivität mit Stand; `offline`: Laden schlug fehl, gezeigt
/// wird der gespeicherte Stand.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cached<T> {
    pub data: T,
    pub fetched_at: i64,
    pub offline: bool,
}

/// Datei aus einem Beitrag oder Eintrag.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRef {
    pub name: String,
    pub url: String,
    pub size: i64,
}

impl FileRef {
    pub(crate) fn from_value(v: &serde_json::Value) -> Option<Self> {
        Some(Self {
            name: v["filename"].as_str()?.to_owned(),
            url: v["url"].as_str().or(v["fileurl"].as_str())?.to_owned(),
            size: v["filesize"].as_i64().unwrap_or(0),
        })
    }

    pub(crate) fn list(v: &serde_json::Value) -> Vec<Self> {
        v.as_array()
            .map(|a| a.iter().filter_map(Self::from_value).collect())
            .unwrap_or_default()
    }
}

impl Session<'_> {
    /// Lädt mit `fetch` und speichert das Ergebnis unter `key`; schlägt das
    /// Laden fehl, kommt der gespeicherte Stand.
    pub(crate) async fn cached<T, F, Fut>(&self, key: &str, fetch: F) -> Result<Cached<T>>
    where
        T: Serialize + DeserializeOwned,
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        match fetch().await {
            Ok(data) => {
                let at = now();
                self.db().conn().execute(
                    "INSERT OR REPLACE INTO moodle_activity_cache (key, json, fetched_at)
                     VALUES (?1, ?2, ?3)",
                    (key, serde_json::to_string(&data)?, at),
                )?;
                Ok(Cached {
                    data,
                    fetched_at: at,
                    offline: false,
                })
            }
            // Ohne gültige Anmeldung nicht still den alten Stand zeigen.
            Err(e @ Error::InvalidToken) => Err(e),
            Err(e) => {
                let stored: Option<(String, i64)> = self
                    .db()
                    .conn()
                    .query_row(
                        "SELECT json, fetched_at FROM moodle_activity_cache WHERE key = ?1",
                        [key],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?;
                let Some((json, at)) = stored else {
                    return Err(e);
                };
                Ok(Cached {
                    data: serde_json::from_str(&json)?,
                    fetched_at: at,
                    offline: true,
                })
            }
        }
    }

    /// Das Modul aus dem gespeicherten Kursinhalt.
    pub(crate) fn module(&self, course_id: i64, module_id: i64) -> Result<RemoteModule> {
        let (sections, _) = contents::load(self.db(), course_id)?.ok_or(Error::FileNotFound)?;
        sections
            .into_iter()
            .flat_map(|s| s.modules)
            .find(|m| m.id == module_id)
            .ok_or(Error::FileNotFound)
    }

    /// Lädt eine Datei dieser Moodle-Seite nach `dir` und liefert den Pfad.
    /// Das Token geht nur an Adressen der eigenen Seite.
    pub async fn download_file(&self, file: &FileRef, dir: &Path) -> Result<PathBuf> {
        let url = Url::parse(&file.url).map_err(|_| Error::InvalidUrl(file.url.clone()))?;
        let client = crate::Client::new(self.base()?);
        let path = dir.join(paths::component(&file.name));
        client
            .download_to(client.authorize(&url, &self.token()?), &path)
            .await?;
        Ok(path)
    }
}
