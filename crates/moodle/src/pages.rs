//! Textseiten und Bücher: Inhalt als bereinigtes HTML, offline verfügbar.

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::contents::{self, RemoteModule, now};
use crate::html::{MediaUrls, sanitize};
use crate::{Error, Result, Session, plain};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageContent {
    pub module_id: i64,
    pub name: String,
    pub chapters: Vec<Chapter>,
    pub fetched_at: i64,
    /// Neu laden fehlgeschlagen; gezeigt wird der gespeicherte Stand.
    pub offline: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chapter {
    pub title: String,
    /// 0 = Kapitel, 1 = Unterkapitel.
    pub level: i64,
    pub html: String,
}

/// Rohes Kapitel mit der Adresse, gegen die relative Bilder aufgelöst werden.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RawChapter {
    title: String,
    level: i64,
    html: String,
    base: String,
}

#[derive(Debug, Deserialize)]
struct BookItem {
    title: String,
    href: String,
    #[serde(default)]
    level: i64,
    #[serde(default)]
    subitems: Vec<BookItem>,
}

fn flatten(items: Vec<BookItem>, out: &mut Vec<BookItem>) {
    for mut item in items {
        let subitems = std::mem::take(&mut item.subitems);
        out.push(item);
        flatten(subitems, out);
    }
}

fn find_module(sections: Vec<contents::RemoteSection>, module_id: i64) -> Option<RemoteModule> {
    sections
        .into_iter()
        .flat_map(|s| s.modules)
        .find(|m| m.id == module_id)
}

impl Session<'_> {
    pub async fn page_content(
        &self,
        course_id: i64,
        module_id: i64,
        media: &MediaUrls,
    ) -> Result<PageContent> {
        let (sections, _) = contents::load(self.db(), course_id)?.ok_or(Error::FileNotFound)?;
        let module = find_module(sections, module_id).ok_or(Error::FileNotFound)?;
        let (raw, fetched_at, offline) = match self.fetch_chapters(&module).await {
            Ok(raw) => {
                let at = now();
                self.db().conn().execute(
                    "INSERT OR REPLACE INTO moodle_pages (module_id, json, fetched_at)
                     VALUES (?1, ?2, ?3)",
                    (module_id, serde_json::to_string(&raw)?, at),
                )?;
                (raw, at, false)
            }
            Err(e) => {
                let cached: Option<(String, i64)> = self
                    .db()
                    .conn()
                    .query_row(
                        "SELECT json, fetched_at FROM moodle_pages WHERE module_id = ?1",
                        [module_id],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )
                    .optional()?;
                let Some((json, at)) = cached else {
                    return Err(e);
                };
                (serde_json::from_str(&json)?, at, true)
            }
        };
        let chapters = raw
            .into_iter()
            .map(|c| {
                let base = Url::parse(&c.base).map_err(|_| Error::InvalidUrl(c.base.clone()))?;
                Ok(Chapter {
                    title: plain(&c.title),
                    level: c.level,
                    html: sanitize(&c.html, &base, media),
                })
            })
            .collect::<Result<_>>()?;
        Ok(PageContent {
            module_id,
            name: plain(&module.name),
            chapters,
            fetched_at,
            offline,
        })
    }

    async fn fetch_chapters(&self, module: &RemoteModule) -> Result<Vec<RawChapter>> {
        let file = |path: &str| {
            module
                .contents
                .iter()
                .find(|c| {
                    c.kind == "file"
                        && c.filename == "index.html"
                        && c.filepath.as_deref() == Some(path)
                })
                .and_then(|c| c.fileurl.clone())
        };
        // Bei Textseiten fehlt in der Adresse von index.html die Revision
        // (`…/content/index.html`), die Bilder liegen unter `…/content/6/`.
        // Relative Bilder werden daher gegen eine Nachbardatei aufgelöst.
        let sibling = |path: &str| {
            module
                .contents
                .iter()
                .find(|c| {
                    c.kind == "file"
                        && c.filename != "index.html"
                        && c.filepath.as_deref() == Some(path)
                })
                .and_then(|c| c.fileurl.clone())
        };
        let mut chapters = Vec::new();
        match module.modname.as_str() {
            "page" => {
                let url = file("/").ok_or(Error::FileNotFound)?;
                let mut chapter = self.chapter(&module.name, 0, &url).await?;
                if let Some(base) = sibling("/") {
                    chapter.base = base;
                }
                chapters.push(chapter);
            }
            "book" => {
                let structure = module
                    .contents
                    .iter()
                    .find(|c| c.kind == "content" && c.filename == "structure")
                    .and_then(|c| c.content.as_deref())
                    .ok_or(Error::FileNotFound)?;
                let mut items = Vec::new();
                flatten(serde_json::from_str(structure)?, &mut items);
                for item in items {
                    // href: "139/index.html" → Dateipfad "/139/".
                    let dir = item.href.trim_end_matches("index.html");
                    if let Some(url) = file(&format!("/{dir}")) {
                        chapters.push(self.chapter(&item.title, item.level, &url).await?);
                    }
                }
            }
            _ => return Err(Error::FileNotFound),
        }
        Ok(chapters)
    }

    async fn chapter(&self, title: &str, level: i64, url: &str) -> Result<RawChapter> {
        let parsed = Url::parse(url).map_err(|_| Error::InvalidUrl(url.to_owned()))?;
        Ok(RawChapter {
            title: title.to_owned(),
            level,
            html: self.fetch_text(&parsed).await?,
            base: url.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_book_structure() {
        let items: Vec<BookItem> = serde_json::from_str(
            r#"[{"title":"A","href":"1/index.html","level":0,"subitems":[{"title":"A.1","href":"2/index.html","level":1}]},{"title":"B","href":"3/index.html","level":0,"subitems":[]}]"#,
        )
        .unwrap();
        let mut out = Vec::new();
        flatten(items, &mut out);
        let titles: Vec<_> = out.iter().map(|i| (i.title.as_str(), i.level)).collect();
        assert_eq!(titles, [("A", 0), ("A.1", 1), ("B", 0)]);
    }
}
