//! Abgleich der Kursdateien mit dem Sync-Ordner.
//!
//! Grundsatz: Lokale Dateien werden nie überschrieben. Ändert sich eine
//! Datei online, wird das nur angezeigt; erst [`Files::update`] holt die neue
//! Version und benennt die alte um.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{OptionalExtension, Row};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use tsa_core::Database;

use crate::contents::{RemoteSection, now};
use crate::paths::{component, with_suffix};
use crate::{Error, Result, plain};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
    /// Noch nicht heruntergeladen.
    Remote,
    Current,
    /// Online gibt es eine neuere Version.
    UpdateAvailable,
    /// Heruntergeladen, aber lokal gelöscht oder verschoben.
    Missing,
    /// Online entfernt; die lokale Kopie bleibt.
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileState {
    pub id: i64,
    pub status: FileStatus,
    /// Die lokale Kopie wurde seit dem Herunterladen verändert.
    pub modified_locally: bool,
}

/// Eine Datei, wie sie in den Kursinhalten steht.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteFile {
    pub module_id: i64,
    /// Pfad innerhalb des Moduls, z. B. `/Unterordner/Blatt.pdf`.
    pub remote_path: String,
    pub url: String,
    pub modified: Option<i64>,
    pub size: i64,
    /// Vorgesehener Ort im Sync-Ordner, relativ.
    pub target: PathBuf,
}

#[derive(Debug, Clone)]
struct Record {
    id: i64,
    module_id: i64,
    remote_path: String,
    file_url: String,
    remote_modified: Option<i64>,
    remote_size: i64,
    local_path: Option<String>,
    local_hash: Option<String>,
    local_mtime: Option<i64>,
    local_size: Option<i64>,
    downloaded_modified: Option<i64>,
    downloaded_size: Option<i64>,
    removed: bool,
}

impl Record {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            module_id: row.get("module_id")?,
            remote_path: row.get("remote_path")?,
            file_url: row.get("file_url")?,
            remote_modified: row.get("remote_modified")?,
            remote_size: row.get("remote_size")?,
            local_path: row.get("local_path")?,
            local_hash: row.get("local_hash")?,
            local_mtime: row.get("local_mtime")?,
            local_size: row.get("local_size")?,
            downloaded_modified: row.get("downloaded_modified")?,
            downloaded_size: row.get("downloaded_size")?,
            removed: row.get("removed")?,
        })
    }

    fn remote_changed(&self) -> bool {
        self.downloaded_modified != self.remote_modified
            || self.downloaded_size != Some(self.remote_size)
    }
}

/// Ordner eines Kurses im Sync-Ordner, relativ.
pub fn course_dir(course_name: &str) -> PathBuf {
    PathBuf::from(component(&plain(course_name)))
}

/// Dateien eines Kurses aus den Rohdaten, mit Zielpfad
/// `Kurs/Abschnitt[/Ordner/Unterordner]/Datei`.
pub fn collect(course_name: &str, sections: &[RemoteSection]) -> Vec<RemoteFile> {
    let course_dir = course_dir(course_name);
    let mut files = Vec::new();
    for section in sections {
        let name = plain(&section.name);
        let section_dir =
            course_dir.join(component(if name.is_empty() { "Abschnitt" } else { &name }));
        for module in &section.modules {
            let module_dir = match module.modname.as_str() {
                "resource" => section_dir.clone(),
                "folder" => section_dir.join(component(&plain(&module.name))),
                _ => continue,
            };
            for content in module.contents.iter().filter(|c| c.kind == "file") {
                let Some(url) = &content.fileurl else {
                    continue;
                };
                let folder = content.filepath.as_deref().unwrap_or("/");
                let mut target = module_dir.clone();
                for part in folder.split('/').filter(|p| !p.is_empty()) {
                    target.push(component(part));
                }
                target.push(component(&content.filename));
                files.push(RemoteFile {
                    module_id: module.id,
                    remote_path: format!("{folder}{}", content.filename),
                    url: url.clone(),
                    modified: content.timemodified,
                    size: content.filesize,
                    target,
                });
            }
        }
    }
    files
}

/// Übernimmt den Stand der Kursinhalte: neue Dateien eintragen, geänderte
/// aktualisieren, verschwundene als entfernt markieren.
pub fn record(db: &Database, course_id: i64, files: &[RemoteFile]) -> Result<()> {
    let mut conn = db.conn();
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE moodle_files SET removed = 1 WHERE course_id = ?1",
        [course_id],
    )?;
    for f in files {
        tx.execute(
            "INSERT INTO moodle_files
                 (course_id, module_id, remote_path, file_url, remote_modified, remote_size, target_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (module_id, remote_path) DO UPDATE SET
                 course_id = excluded.course_id,
                 file_url = excluded.file_url,
                 remote_modified = excluded.remote_modified,
                 remote_size = excluded.remote_size,
                 target_path = excluded.target_path,
                 removed = 0",
            (
                course_id,
                f.module_id,
                &f.remote_path,
                &f.url,
                f.modified,
                f.size,
                f.target.to_string_lossy(),
            ),
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Anzahl Dateien je Kurs, ohne die Festplatte anzufassen.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseFiles {
    pub total: i64,
    pub downloaded: i64,
    pub updates: i64,
}

pub fn course_stats(db: &Database) -> Result<HashMap<i64, CourseFiles>> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT course_id,
                COUNT(*),
                SUM(local_path IS NOT NULL),
                SUM(local_path IS NOT NULL AND (
                    downloaded_modified IS NOT remote_modified
                    OR downloaded_size IS NOT remote_size))
         FROM moodle_files WHERE removed = 0 GROUP BY course_id",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            CourseFiles {
                total: r.get(1)?,
                downloaded: r.get(2)?,
                updates: r.get(3)?,
            },
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn clear(db: &Database) -> Result<()> {
    db.conn().execute("DELETE FROM moodle_files", [])?;
    Ok(())
}

fn mtime(meta: &std::fs::Metadata) -> Option<i64> {
    let since = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(since.as_secs() as i64)
}

async fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Kalenderdatum `JJJJ-MM-TT` (UTC) zu einer Unix-Zeit.
fn date(unix: i64) -> String {
    // Algorithmus „civil_from_days“ von Howard Hinnant.
    let z = unix.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Zugriff auf die Dateien im Sync-Ordner.
pub struct Files<'a> {
    pub db: &'a Database,
    pub root: &'a Path,
}

impl Files<'_> {
    fn get(&self, id: i64) -> Result<Record> {
        self.db
            .conn()
            .query_row(
                "SELECT * FROM moodle_files WHERE id = ?1",
                [id],
                Record::from_row,
            )
            .optional()?
            .ok_or(Error::FileNotFound)
    }

    fn records(&self, course_id: i64) -> Result<Vec<Record>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare("SELECT * FROM moodle_files WHERE course_id = ?1")?;
        let rows = stmt.query_map([course_id], Record::from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Zustand der lokalen Kopie; prüft den Inhalt nur, wenn Größe oder
    /// Änderungszeit abweichen.
    async fn state(&self, r: &Record) -> FileState {
        let state = |status, modified_locally| FileState {
            id: r.id,
            status,
            modified_locally,
        };
        let Some(local) = &r.local_path else {
            return state(FileStatus::Remote, false);
        };
        let path = self.root.join(local);
        let Ok(meta) = tokio::fs::metadata(&path).await else {
            return state(FileStatus::Missing, false);
        };
        let unchanged_meta =
            Some(meta.len() as i64) == r.local_size && mtime(&meta) == r.local_mtime;
        let modified = !unchanged_meta
            && match hash_file(&path).await {
                Ok(hash) => Some(hash) != r.local_hash,
                Err(_) => false,
            };
        let status = if r.removed {
            FileStatus::Removed
        } else if r.remote_changed() {
            FileStatus::UpdateAvailable
        } else {
            FileStatus::Current
        };
        state(status, modified)
    }

    /// Zustände aller Dateien eines Kurses, nach Modul und Pfad.
    pub async fn states(&self, course_id: i64) -> Result<HashMap<(i64, String), FileState>> {
        let mut out = HashMap::new();
        for r in self.records(course_id)? {
            let state = self.state(&r).await;
            out.insert((r.module_id, r.remote_path), state);
        }
        Ok(out)
    }

    /// Freier Pfad für eine neue Datei: belegt ist er, wenn dort schon etwas
    /// liegt oder eine andere Datei ihn beansprucht.
    fn free_path(&self, target: &str, own_id: i64) -> Result<String> {
        let taken = |candidate: &str| -> Result<bool> {
            if self.root.join(candidate).exists() {
                return Ok(true);
            }
            let other: Option<i64> = self
                .db
                .conn()
                .query_row(
                    "SELECT id FROM moodle_files WHERE local_path = ?1 AND id != ?2",
                    (candidate, own_id),
                    |r| r.get(0),
                )
                .optional()?;
            Ok(other.is_some())
        };
        let path = PathBuf::from(target);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut candidate = target.to_owned();
        let mut n = 2;
        while taken(&candidate)? {
            candidate = path
                .with_file_name(with_suffix(&name, &n.to_string()))
                .to_string_lossy()
                .into_owned();
            n += 1;
        }
        Ok(candidate)
    }

    /// Lädt eine Datei an ihren Platz und merkt sich den Stand.
    async fn fetch<F, Fut>(&self, r: &Record, download: &F) -> Result<PathBuf>
    where
        F: Fn(String, PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let local = match &r.local_path {
            Some(p) => p.clone(),
            None => {
                let target: String = self.db.conn().query_row(
                    "SELECT target_path FROM moodle_files WHERE id = ?1",
                    [r.id],
                    |row| row.get(0),
                )?;
                self.free_path(&target, r.id)?
            }
        };
        let path = self.root.join(&local);
        download(r.file_url.clone(), path.clone()).await?;
        let meta = tokio::fs::metadata(&path).await.map_err(Error::Io)?;
        let hash = hash_file(&path).await.map_err(Error::Io)?;
        self.db.conn().execute(
            "UPDATE moodle_files SET local_path = ?2, local_hash = ?3, local_mtime = ?4,
                 local_size = ?5, downloaded_modified = remote_modified,
                 downloaded_size = remote_size
             WHERE id = ?1",
            (r.id, &local, hash, mtime(&meta), meta.len() as i64),
        )?;
        Ok(path)
    }

    /// Lädt neue und lokal fehlende Dateien eines Kurses. Geänderte Dateien
    /// bleiben unangetastet. Liefert die Zahl der geladenen Dateien.
    pub async fn sync<F, Fut>(&self, course_id: i64, download: F) -> Result<usize>
    where
        F: Fn(String, PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let mut count = 0;
        for r in self.records(course_id)? {
            if r.removed {
                continue;
            }
            let status = self.state(&r).await.status;
            if matches!(status, FileStatus::Remote | FileStatus::Missing) {
                self.fetch(&r, &download).await?;
                count += 1;
            }
        }
        Ok(count)
    }

    /// Pfad der lokalen Kopie; lädt sie bei Bedarf zuerst.
    pub async fn open<F, Fut>(&self, id: i64, download: F) -> Result<PathBuf>
    where
        F: Fn(String, PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let r = self.get(id)?;
        match self.state(&r).await.status {
            FileStatus::Remote | FileStatus::Missing => self.fetch(&r, &download).await,
            _ => Ok(self.root.join(r.local_path.unwrap_or_default())),
        }
    }

    /// Holt die neue Version. Die bisherige Datei bleibt unter einem neuen
    /// Namen erhalten, z. B. `Blatt (lokal 2026-10-21).pdf`.
    pub async fn update<F, Fut>(&self, id: i64, download: F) -> Result<PathBuf>
    where
        F: Fn(String, PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        let r = self.get(id)?;
        let state = self.state(&r).await;
        if let (Some(local), FileStatus::UpdateAvailable | FileStatus::Current) =
            (&r.local_path, state.status)
        {
            let label = if state.modified_locally {
                "lokal"
            } else {
                "Version"
            };
            let path = PathBuf::from(local);
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut suffix = format!("{label} {}", date(now()));
            let mut n = 2;
            let mut old = path.with_file_name(with_suffix(&name, &suffix));
            while self.root.join(&old).exists() {
                suffix = format!("{label} {} {n}", date(now()));
                old = path.with_file_name(with_suffix(&name, &suffix));
                n += 1;
            }
            tokio::fs::rename(self.root.join(local), self.root.join(&old))
                .await
                .map_err(Error::Io)?;
            self.db.conn().execute(
                "INSERT INTO moodle_file_versions (file_id, local_path, created_at)
                 VALUES (?1, ?2, ?3)",
                (r.id, old.to_string_lossy(), now()),
            )?;
        }
        self.fetch(&r, &download).await
    }

    /// Frühere, umbenannte Versionen einer Datei, neueste zuerst.
    pub fn versions(&self, id: i64) -> Result<Vec<PathBuf>> {
        let conn = self.db.conn();
        let mut stmt = conn.prepare(
            "SELECT local_path FROM moodle_file_versions WHERE file_id = ?1 ORDER BY id DESC",
        )?;
        let rows = stmt.query_map([id], |r| r.get::<_, String>(0))?;
        Ok(rows
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|p| self.root.join(p))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    fn db() -> Database {
        let db = Database::open_in_memory().unwrap();
        crate::init(&db).unwrap();
        db
    }

    fn sections(modified: i64) -> Vec<RemoteSection> {
        serde_json::from_value(json!([
            {"id": 1, "name": "Fotosynthese: Grundlagen", "section": 1, "modules": [
                {"id": 10, "name": "Blatt", "modname": "resource", "contents": [
                    {"type": "file", "filename": "Blatt.pdf", "filepath": "/", "filesize": 3,
                     "fileurl": "https://m/blatt", "timemodified": modified}]},
                {"id": 11, "name": "Material", "modname": "folder", "contents": [
                    {"type": "file", "filename": "a.txt", "filepath": "/Teil 1/", "filesize": 1,
                     "fileurl": "https://m/a", "timemodified": 1}]},
                {"id": 12, "name": "Seite", "modname": "page", "contents": [
                    {"type": "file", "filename": "index.html", "filepath": "/", "filesize": 1,
                     "fileurl": "https://m/page", "timemodified": 1}]}
            ]}
        ]))
        .unwrap()
    }

    /// Simuliert Downloads: schreibt den Inhalt aus `content` je URL.
    #[derive(Clone, Default)]
    struct Fake(Arc<Mutex<HashMap<String, String>>>, Arc<Mutex<usize>>);

    impl Fake {
        fn set(&self, url: &str, body: &str) {
            self.0.lock().unwrap().insert(url.into(), body.into());
        }
        fn calls(&self) -> usize {
            *self.1.lock().unwrap()
        }
        fn download(&self) -> impl Fn(String, PathBuf) -> std::future::Ready<Result<()>> + '_ {
            move |url, path| {
                *self.1.lock().unwrap() += 1;
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                let body = self
                    .0
                    .lock()
                    .unwrap()
                    .get(&url)
                    .cloned()
                    .unwrap_or_default();
                std::fs::write(&path, body).unwrap();
                std::future::ready(Ok(()))
            }
        }
    }

    fn setup(modified: i64) -> (Database, tempfile::TempDir, Fake) {
        let db = db();
        let dir = tempfile::tempdir().unwrap();
        record(&db, 7, &collect("Bio 10b", &sections(modified))).unwrap();
        let fake = Fake::default();
        fake.set("https://m/blatt", "v1!");
        fake.set("https://m/a", "a");
        (db, dir, fake)
    }

    #[test]
    fn collects_targets() {
        let files = collect("Bio 10b", &sections(1));
        let targets: Vec<_> = files.iter().map(|f| f.target.clone()).collect();
        assert_eq!(
            targets,
            [
                PathBuf::from("Bio 10b/Fotosynthese_ Grundlagen/Blatt.pdf"),
                PathBuf::from("Bio 10b/Fotosynthese_ Grundlagen/Material/Teil 1/a.txt"),
            ]
        );
        assert_eq!(files[1].remote_path, "/Teil 1/a.txt");
    }

    #[tokio::test]
    async fn sync_downloads_once() {
        let (db, dir, fake) = setup(1);
        let files = Files {
            db: &db,
            root: dir.path(),
        };
        assert_eq!(files.sync(7, fake.download()).await.unwrap(), 2);
        assert_eq!(files.sync(7, fake.download()).await.unwrap(), 0);
        let blatt = dir
            .path()
            .join("Bio 10b/Fotosynthese_ Grundlagen/Blatt.pdf");
        assert_eq!(std::fs::read_to_string(&blatt).unwrap(), "v1!");
        let states = files.states(7).await.unwrap();
        assert!(
            states
                .values()
                .all(|s| s.status == FileStatus::Current && !s.modified_locally)
        );
        assert_eq!(
            course_stats(&db).unwrap()[&7],
            CourseFiles {
                total: 2,
                downloaded: 2,
                updates: 0
            }
        );
    }

    #[tokio::test]
    async fn remote_change_is_only_flagged() {
        let (db, dir, fake) = setup(1);
        let files = Files {
            db: &db,
            root: dir.path(),
        };
        files.sync(7, fake.download()).await.unwrap();
        let blatt = dir
            .path()
            .join("Bio 10b/Fotosynthese_ Grundlagen/Blatt.pdf");
        std::fs::write(&blatt, "meine Notizen").unwrap();

        record(&db, 7, &collect("Bio 10b", &sections(2))).unwrap();
        fake.set("https://m/blatt", "v2!");
        let before = fake.calls();
        files.sync(7, fake.download()).await.unwrap();
        assert_eq!(fake.calls(), before, "keine automatische Aktualisierung");
        assert_eq!(std::fs::read_to_string(&blatt).unwrap(), "meine Notizen");

        let state = files.states(7).await.unwrap()[&(10, "/Blatt.pdf".to_string())];
        assert_eq!(state.status, FileStatus::UpdateAvailable);
        assert!(state.modified_locally);
        assert_eq!(course_stats(&db).unwrap()[&7].updates, 1);

        // Neue Version holen: alte bleibt umbenannt erhalten.
        files.update(state.id, fake.download()).await.unwrap();
        assert_eq!(std::fs::read_to_string(&blatt).unwrap(), "v2!");
        let versions = files.versions(state.id).unwrap();
        assert_eq!(versions.len(), 1);
        assert!(versions[0].to_string_lossy().contains("Blatt (lokal "));
        assert_eq!(
            std::fs::read_to_string(&versions[0]).unwrap(),
            "meine Notizen"
        );
        let state = files.states(7).await.unwrap()[&(10, "/Blatt.pdf".to_string())];
        assert_eq!(state.status, FileStatus::Current);
    }

    #[tokio::test]
    async fn missing_and_removed() {
        let (db, dir, fake) = setup(1);
        let files = Files {
            db: &db,
            root: dir.path(),
        };
        files.sync(7, fake.download()).await.unwrap();
        let a = dir
            .path()
            .join("Bio 10b/Fotosynthese_ Grundlagen/Material/Teil 1/a.txt");
        std::fs::remove_file(&a).unwrap();
        let key = (11, "/Teil 1/a.txt".to_string());
        assert_eq!(
            files.states(7).await.unwrap()[&key].status,
            FileStatus::Missing
        );
        files.sync(7, fake.download()).await.unwrap();
        assert!(a.exists(), "fehlende Datei neu geladen");

        let mut s = sections(1);
        s[0].modules.retain(|m| m.id != 11);
        record(&db, 7, &collect("Bio 10b", &s)).unwrap();
        assert_eq!(
            files.states(7).await.unwrap()[&key].status,
            FileStatus::Removed
        );
        assert!(a.exists(), "lokale Kopie bleibt");
    }

    #[tokio::test]
    async fn does_not_overwrite_foreign_files() {
        let (db, dir, fake) = setup(1);
        let blatt = dir
            .path()
            .join("Bio 10b/Fotosynthese_ Grundlagen/Blatt.pdf");
        std::fs::create_dir_all(blatt.parent().unwrap()).unwrap();
        std::fs::write(&blatt, "fremd").unwrap();
        let files = Files {
            db: &db,
            root: dir.path(),
        };
        files.sync(7, fake.download()).await.unwrap();
        assert_eq!(std::fs::read_to_string(&blatt).unwrap(), "fremd");
        assert_eq!(
            std::fs::read_to_string(blatt.with_file_name("Blatt (2).pdf")).unwrap(),
            "v1!"
        );
    }

    #[test]
    fn dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_792_609_267), "2026-10-21");
        assert_eq!(date(951_782_400), "2000-02-29");
    }
}
