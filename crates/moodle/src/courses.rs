use rusqlite::Row;
use serde::{Deserialize, Serialize};
use tsa_core::Database;

use crate::Result;

/// Kurs, wie die App ihn speichert und anzeigt.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Course {
    pub id: i64,
    pub full_name: String,
    pub short_name: String,
    pub visible: bool,
    /// `false`, wenn der Nutzer nicht mehr eingeschrieben ist, der Kurs aber
    /// noch synchronisiert wurde.
    pub enrolled: bool,
    pub sync: bool,
    pub start_date: Option<i64>,
    pub end_date: Option<i64>,
    pub last_access: Option<i64>,
    /// Fortschritt in Prozent, falls der Kurs Abschlüsse verfolgt.
    pub progress: Option<f64>,
    /// Kursbild als Medien-Adresse der App, siehe [`Course::with_media`].
    pub image: Option<String>,
    #[serde(skip)]
    pub image_url: Option<String>,
}

/// Kurs aus `core_enrol_get_users_courses`.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteCourse {
    pub id: i64,
    pub fullname: String,
    pub displayname: Option<String>,
    pub shortname: String,
    #[serde(default = "visible")]
    pub visible: i64,
    pub startdate: Option<i64>,
    pub enddate: Option<i64>,
    pub lastaccess: Option<i64>,
    pub progress: Option<f64>,
    #[serde(default)]
    pub overviewfiles: Vec<OverviewFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OverviewFile {
    pub fileurl: String,
    pub mimetype: Option<String>,
}

fn visible() -> i64 {
    1
}

/// Moodle nutzt 0 für „kein Datum“.
fn date(value: Option<i64>) -> Option<i64> {
    value.filter(|&v| v > 0)
}

impl Course {
    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            full_name: row.get("fullname")?,
            short_name: row.get("shortname")?,
            visible: row.get("visible")?,
            enrolled: row.get("enrolled")?,
            sync: row.get("sync")?,
            start_date: row.get("start_date")?,
            end_date: row.get("end_date")?,
            last_access: row.get("last_access")?,
            progress: row.get("progress")?,
            image: None,
            image_url: row.get("image_url")?,
        })
    }

    /// Setzt [`Course::image`] auf eine von der App auslieferbare Adresse.
    pub fn with_media(mut self, media: &crate::html::MediaUrls) -> Self {
        self.image = self
            .image_url
            .as_deref()
            .and_then(|u| url::Url::parse(u).ok())
            .map(|u| media.url(&u));
        self
    }

    pub fn list(db: &Database) -> Result<Vec<Self>> {
        let conn = db.conn();
        let mut stmt =
            conn.prepare("SELECT * FROM moodle_courses ORDER BY fullname COLLATE NOCASE, id")?;
        let courses = stmt
            .query_map([], Self::from_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(courses)
    }

    /// Übernimmt die aktuelle Kursliste. Die Sync-Auswahl bleibt erhalten;
    /// Kurse ohne Einschreibung bleiben nur, wenn sie synchronisiert wurden.
    pub fn store_enrolled(db: &Database, remote: &[RemoteCourse]) -> Result<()> {
        let mut conn = db.conn();
        let tx = conn.transaction()?;
        tx.execute("UPDATE moodle_courses SET enrolled = 0", [])?;
        for c in remote {
            tx.execute(
                "INSERT INTO moodle_courses
                     (id, fullname, shortname, visible, enrolled, start_date, end_date,
                      last_access, progress, image_url)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT (id) DO UPDATE SET
                     fullname = excluded.fullname,
                     shortname = excluded.shortname,
                     visible = excluded.visible,
                     enrolled = 1,
                     start_date = excluded.start_date,
                     end_date = excluded.end_date,
                     last_access = excluded.last_access,
                     progress = excluded.progress,
                     image_url = excluded.image_url",
                (
                    c.id,
                    crate::plain(c.displayname.as_ref().unwrap_or(&c.fullname)),
                    crate::plain(&c.shortname),
                    c.visible != 0,
                    date(c.startdate),
                    date(c.enddate),
                    date(c.lastaccess),
                    c.progress,
                    c.overviewfiles
                        .iter()
                        .find(|f| {
                            f.mimetype
                                .as_deref()
                                .is_some_and(|m| m.starts_with("image/"))
                        })
                        .map(|f| f.fileurl.clone()),
                ),
            )?;
        }
        tx.execute(
            "DELETE FROM moodle_courses WHERE enrolled = 0 AND sync = 0",
            [],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn set_sync(db: &Database, id: i64, sync: bool) -> Result<()> {
        db.conn().execute(
            "UPDATE moodle_courses SET sync = ?2 WHERE id = ?1",
            (id, sync),
        )?;
        Ok(())
    }

    pub fn clear(db: &Database) -> Result<()> {
        db.conn().execute("DELETE FROM moodle_courses", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Database {
        let db = Database::open_in_memory().unwrap();
        crate::init(&db).unwrap();
        db
    }

    fn remote(id: i64, name: &str) -> RemoteCourse {
        RemoteCourse {
            id,
            fullname: name.into(),
            displayname: None,
            shortname: name.into(),
            visible: 1,
            startdate: Some(0),
            enddate: Some(1_700_000_000),
            lastaccess: None,
            progress: Some(12.5),
            overviewfiles: vec![],
        }
    }

    #[test]
    fn store_and_list() {
        let db = db();
        Course::store_enrolled(&db, &[remote(2, "b"), remote(1, "A")]).unwrap();
        let list = Course::list(&db).unwrap();
        assert_eq!(list.iter().map(|c| c.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(list[0].start_date, None);
        assert_eq!(list[0].end_date, Some(1_700_000_000));
        assert!(list.iter().all(|c| c.enrolled && !c.sync));
    }

    #[test]
    fn keeps_selection_and_synced_courses() {
        let db = db();
        Course::store_enrolled(&db, &[remote(1, "a"), remote(2, "b"), remote(3, "c")]).unwrap();
        Course::set_sync(&db, 1, true).unwrap();
        Course::set_sync(&db, 2, true).unwrap();
        // Kurs 2 und 3 nicht mehr eingeschrieben, Kurs 1 umbenannt.
        Course::store_enrolled(&db, &[remote(1, "a2")]).unwrap();
        let list = Course::list(&db).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].id, list[0].full_name.as_str()), (1, "a2"));
        assert!(list[0].sync && list[0].enrolled);
        assert!(list[1].sync && !list[1].enrolled);
    }

    #[test]
    fn prefers_display_name() {
        let db = db();
        let mut c = remote(1, "raw");
        c.displayname = Some(" Kunst &amp; Musik ".into());
        Course::store_enrolled(&db, &[c]).unwrap();
        assert_eq!(Course::list(&db).unwrap()[0].full_name, "Kunst & Musik");
    }
}
