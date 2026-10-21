//! Kursinhalte aus `core_course_get_contents`, aufbereitet für die Anzeige.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tsa_core::Database;
use url::Url;

use crate::html::{MediaUrls, sanitize};
use crate::{Result, plain};

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteSection {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub section: i64,
    pub visible: Option<i64>,
    pub uservisible: Option<bool>,
    pub availabilityinfo: Option<String>,
    /// `mod_subsection` bei Unterabschnitten (Moodle 4.5+).
    pub component: Option<String>,
    #[serde(default)]
    pub modules: Vec<RemoteModule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteModule {
    pub id: i64,
    pub url: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub visible: Option<i64>,
    pub uservisible: Option<bool>,
    pub availabilityinfo: Option<String>,
    #[serde(default)]
    pub modicon: String,
    pub modname: String,
    #[serde(default)]
    pub modplural: String,
    #[serde(default)]
    pub indent: i64,
    pub customdata: Option<String>,
    #[serde(default)]
    pub dates: Vec<RemoteDate>,
    pub completiondata: Option<RemoteCompletion>,
    #[serde(default)]
    pub contents: Vec<RemoteContent>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteDate {
    pub label: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteCompletion {
    pub state: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemoteContent {
    #[serde(rename = "type")]
    pub kind: String,
    pub filename: String,
    pub filepath: Option<String>,
    #[serde(default)]
    pub filesize: i64,
    pub fileurl: String,
    pub timemodified: Option<i64>,
    pub mimetype: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseView {
    pub course_id: i64,
    /// Unix-Zeit des Abrufs, für „Stand: …“ im Offline-Fall.
    pub fetched_at: i64,
    pub sections: Vec<SectionView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionView {
    pub id: i64,
    pub number: i64,
    pub name: String,
    /// Bereinigtes HTML.
    pub summary: String,
    pub visible: bool,
    pub available: bool,
    pub availability: Option<String>,
    pub modules: Vec<ModuleView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleView {
    pub id: i64,
    pub name: String,
    /// Moodle-Modulname, z. B. `resource`, `assign`, `label`.
    pub kind: String,
    /// Bezeichnung des Typs in der Sprache der Seite, z. B. „Dateien“.
    pub kind_name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    /// Ansicht im Browser.
    pub url: Option<String>,
    /// Ziel eines Link-Moduls.
    pub link: Option<String>,
    pub visible: bool,
    pub available: bool,
    pub availability: Option<String>,
    pub indent: i64,
    pub dates: Vec<DateView>,
    /// `None`, wenn die Aktivität keinen Abschluss verfolgt.
    pub completed: Option<bool>,
    pub files: Vec<FileView>,
    /// Inhalt eines Unterabschnitts.
    pub subsection: Option<Box<SectionView>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DateView {
    pub label: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileView {
    pub name: String,
    /// Ordnerpfad innerhalb des Moduls, `/` für die oberste Ebene.
    pub path: String,
    pub size: i64,
    pub modified: Option<i64>,
    pub mime: Option<String>,
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Speichert die Rohdaten, damit der Kurs offline angezeigt werden kann und
/// spätere Verbesserungen der Aufbereitung auch für alte Daten gelten.
pub fn store(db: &Database, course_id: i64, raw: &serde_json::Value) -> Result<i64> {
    let fetched_at = now();
    db.conn().execute(
        "INSERT OR REPLACE INTO moodle_course_contents (course_id, json, fetched_at)
         VALUES (?1, ?2, ?3)",
        (course_id, raw.to_string(), fetched_at),
    )?;
    Ok(fetched_at)
}

pub fn load(db: &Database, course_id: i64) -> Result<Option<(Vec<RemoteSection>, i64)>> {
    let row: Option<(String, i64)> = db
        .conn()
        .query_row(
            "SELECT json, fetched_at FROM moodle_course_contents WHERE course_id = ?1",
            [course_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(json, at)| Ok((serde_json::from_str(&json)?, at)))
        .transpose()
}

pub fn clear(db: &Database) -> Result<()> {
    db.conn()
        .execute("DELETE FROM moodle_course_contents", [])?;
    Ok(())
}

struct Builder<'a> {
    base: &'a Url,
    media: &'a MediaUrls,
}

impl Builder<'_> {
    fn html(&self, html: &str) -> String {
        sanitize(html, self.base, self.media)
    }

    fn optional_html(&self, html: Option<&str>) -> Option<String> {
        html.map(|h| self.html(h)).filter(|h| !h.trim().is_empty())
    }

    fn section(
        &self,
        s: RemoteSection,
        subsections: &mut HashMap<i64, RemoteSection>,
    ) -> SectionView {
        SectionView {
            id: s.id,
            number: s.section,
            name: plain(&s.name),
            summary: self.html(&s.summary),
            visible: s.visible.unwrap_or(1) != 0,
            available: s.uservisible.unwrap_or(true),
            availability: self.optional_html(s.availabilityinfo.as_deref()),
            modules: s
                .modules
                .into_iter()
                .map(|m| self.module(m, subsections))
                .collect(),
        }
    }

    fn module(&self, m: RemoteModule, subsections: &mut HashMap<i64, RemoteSection>) -> ModuleView {
        let subsection = (m.modname == "subsection")
            .then(|| subsection_id(m.customdata.as_deref()))
            .flatten()
            .and_then(|id| subsections.remove(&id))
            .map(|s| Box::new(self.section(s, subsections)));
        let link = (m.modname == "url")
            .then(|| m.contents.iter().find(|c| c.kind == "url"))
            .flatten()
            .map(|c| c.fileurl.clone());
        let files = m
            .contents
            .iter()
            .filter(|c| c.kind == "file")
            .map(|c| FileView {
                name: c.filename.clone(),
                path: c.filepath.clone().unwrap_or_else(|| "/".into()),
                size: c.filesize,
                modified: c.timemodified,
                mime: c.mimetype.clone(),
            })
            .collect();
        let icon = Url::parse(&m.modicon).ok().map(|u| self.media.url(&u));
        ModuleView {
            id: m.id,
            name: plain(&m.name),
            kind: m.modname,
            kind_name: plain(&m.modplural),
            description: self.optional_html(m.description.as_deref()),
            icon,
            url: m.url,
            link,
            visible: m.visible.unwrap_or(1) != 0,
            available: m.uservisible.unwrap_or(true),
            availability: self.optional_html(m.availabilityinfo.as_deref()),
            indent: m.indent,
            dates: m
                .dates
                .into_iter()
                .map(|d| DateView {
                    label: plain(&d.label),
                    timestamp: d.timestamp,
                })
                .collect(),
            completed: m.completiondata.map(|c| c.state > 0),
            files,
            subsection,
        }
    }
}

/// `customdata` eines Unterabschnitts: `{"sectionid":"710"}`.
fn subsection_id(customdata: Option<&str>) -> Option<i64> {
    let value: serde_json::Value = serde_json::from_str(customdata?).ok()?;
    match &value["sectionid"] {
        serde_json::Value::String(s) => s.parse().ok(),
        v => v.as_i64(),
    }
}

/// Baut die Anzeige; Unterabschnitte erscheinen in ihrem Modul statt oben.
pub fn view(
    course_id: i64,
    sections: Vec<RemoteSection>,
    fetched_at: i64,
    base: &Url,
    media: &MediaUrls,
) -> CourseView {
    let builder = Builder { base, media };
    let (nested, top): (Vec<_>, Vec<_>) = sections
        .into_iter()
        .partition(|s| s.component.as_deref() == Some("mod_subsection"));
    let mut subsections: HashMap<i64, RemoteSection> =
        nested.into_iter().map(|s| (s.id, s)).collect();
    let mut views: Vec<SectionView> = top
        .into_iter()
        .map(|s| builder.section(s, &mut subsections))
        .collect();
    // Nicht zugeordnete Unterabschnitte nicht verschlucken.
    let mut rest: Vec<_> = subsections.into_values().collect();
    rest.sort_by_key(|s| s.section);
    for s in rest {
        views.push(builder.section(s, &mut HashMap::new()));
    }
    CourseView {
        course_id,
        fetched_at,
        sections: views,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn media() -> MediaUrls {
        MediaUrls {
            prefix: "media://localhost/".into(),
            scheme: "media",
        }
    }

    fn sample() -> Vec<RemoteSection> {
        serde_json::from_value(json!([
            {"id": 1, "name": "Allgemeines", "summary": "<p onclick='x'>Hallo</p>", "section": 0,
             "visible": 1, "uservisible": true, "component": null, "modules": [
                {"id": 10, "name": "Skript", "modname": "resource", "modplural": "Dateien",
                 "modicon": "https://m.example/theme/image.php/f/pdf", "url": "https://m.example/mod/resource/view.php?id=10",
                 "contents": [{"type": "file", "filename": "skript.pdf", "filepath": "/", "filesize": 100,
                               "fileurl": "https://m.example/webservice/pluginfile.php/1/skript.pdf", "timemodified": 5,
                               "mimetype": "application/pdf"}]},
                {"id": 11, "name": "Wikipedia", "modname": "url", "modplural": "Links",
                 "contents": [{"type": "url", "filename": "Wikipedia", "fileurl": "https://de.wikipedia.org"}]},
                {"id": 12, "name": "Teil A", "modname": "subsection", "customdata": "{\"sectionid\":\"3\"}"},
                {"id": 13, "name": "Abgabe", "modname": "assign", "uservisible": false,
                 "availabilityinfo": "<div>Ab Montag</div>",
                 "dates": [{"label": "F&auml;llig:", "timestamp": 99}], "completiondata": {"state": 1}}
             ]},
            {"id": 3, "name": "Teil A", "summary": "", "section": 2, "component": "mod_subsection",
             "modules": [{"id": 20, "name": "Notiz", "modname": "label", "description": "<b>Hi</b>"}]}
        ]))
        .unwrap()
    }

    #[test]
    fn builds_view() {
        let base = Url::parse("https://m.example/").unwrap();
        let v = view(7, sample(), 42, &base, &media());
        assert_eq!(v.sections.len(), 1, "Unterabschnitt nicht oben");
        let s = &v.sections[0];
        assert_eq!(s.summary, "<p>Hallo</p>");
        let [file, url, sub, assign] = &s.modules[..] else {
            panic!("vier Module erwartet")
        };
        assert_eq!(file.files[0].name, "skript.pdf");
        assert!(
            file.icon
                .as_deref()
                .unwrap()
                .starts_with("media://localhost/")
        );
        assert_eq!(url.link.as_deref(), Some("https://de.wikipedia.org"));
        let inner = sub.subsection.as_ref().expect("Unterabschnitt eingebettet");
        assert_eq!(inner.modules[0].description.as_deref(), Some("<b>Hi</b>"));
        assert!(!assign.available);
        assert_eq!(assign.availability.as_deref(), Some("<div>Ab Montag</div>"));
        assert_eq!(assign.dates[0].label, "Fällig:");
        assert_eq!(assign.completed, Some(true));
        assert_eq!(file.completed, None);
    }

    #[test]
    fn orphan_subsection_is_kept() {
        let mut sections = sample();
        sections[0].modules.retain(|m| m.modname != "subsection");
        let base = Url::parse("https://m.example/").unwrap();
        let v = view(7, sections, 42, &base, &media());
        assert_eq!(v.sections.len(), 2);
    }

    #[test]
    fn cache_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        crate::init(&db).unwrap();
        assert!(load(&db, 7).unwrap().is_none());
        let raw = serde_json::to_value(json!([{"id": 1, "section": 0}])).unwrap();
        store(&db, 7, &raw).unwrap();
        let (sections, _) = load(&db, 7).unwrap().unwrap();
        assert_eq!(sections[0].id, 1);
    }
}
