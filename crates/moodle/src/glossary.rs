//! Glossare: Einträge lesen und neue vorschlagen.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::activities::{Cached, FileRef};
use crate::assign::text_to_html;
use crate::html::{MediaUrls, sanitize};
use crate::{Error, Result, Session, plain};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Glossary {
    pub id: i64,
    pub entries: Vec<GlossaryEntry>,
    pub can_add: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlossaryEntry {
    pub id: i64,
    pub concept: String,
    /// Bereinigtes HTML.
    pub html: String,
    pub author: String,
    pub created: i64,
    pub approved: bool,
    pub attachments: Vec<FileRef>,
}

impl Session<'_> {
    pub async fn glossary(
        &self,
        course_id: i64,
        module_id: i64,
        media: &MediaUrls,
    ) -> Result<Cached<Glossary>> {
        let id = self
            .module(course_id, module_id)?
            .instance
            .ok_or(Error::FileNotFound)?;
        let base = self.base()?;
        let mut cached = self
            .cached(&format!("glossary:{module_id}"), || async {
                let mut entries = Vec::new();
                // Seitenweise, gedeckelt bei 2000 Einträgen.
                for page in 0..20 {
                    let v: Value = self
                        .call(
                            "mod_glossary_get_entries_by_letter",
                            &[
                                ("id", id.to_string()),
                                ("letter", "ALL".into()),
                                ("from", (page * 100).to_string()),
                                ("limit", "100".into()),
                            ],
                        )
                        .await?;
                    let list = v["entries"].as_array().cloned().unwrap_or_default();
                    let done = list.len() < 100;
                    entries.extend(list.iter().filter_map(|e| {
                        Some(GlossaryEntry {
                            id: e["id"].as_i64()?,
                            concept: plain(e["concept"].as_str().unwrap_or_default()),
                            html: e["definition"].as_str().unwrap_or_default().to_owned(),
                            author: e["userfullname"].as_str().unwrap_or_default().to_owned(),
                            created: e["timecreated"].as_i64().unwrap_or(0),
                            approved: e["approved"].as_bool().unwrap_or(true),
                            attachments: FileRef::list(&e["attachments"]),
                        })
                    }));
                    if done {
                        break;
                    }
                }
                let course: Value = self
                    .call(
                        "mod_glossary_get_glossaries_by_courses",
                        &[("courseids[0]", course_id.to_string())],
                    )
                    .await
                    .unwrap_or_default();
                let can_add = course["glossaries"]
                    .as_array()
                    .and_then(|list| list.iter().find(|g| g["id"].as_i64() == Some(id)))
                    .is_some_and(|g| {
                        g["canaddentry"].as_i64() == Some(1) || g["canaddentry"] == true
                    });
                Ok(Glossary {
                    id,
                    entries,
                    can_add,
                })
            })
            .await?;
        for entry in &mut cached.data.entries {
            entry.html = sanitize(&entry.html, &base, media);
        }
        Ok(cached)
    }

    /// Neuer Eintrag; muss je nach Einstellung erst freigegeben werden.
    pub async fn glossary_add(&self, glossary_id: i64, concept: &str, text: &str) -> Result<()> {
        let _: Value = self
            .call(
                "mod_glossary_add_entry",
                &[
                    ("glossaryid", glossary_id.to_string()),
                    ("concept", concept.to_owned()),
                    ("definition", text_to_html(text)),
                    ("definitionformat", "1".into()),
                ],
            )
            .await?;
        Ok(())
    }

    /// Anhang eines Eintrags zum Öffnen.
    pub async fn glossary_attachment(
        &self,
        course_id: i64,
        module_id: i64,
        entry_id: i64,
        name: &str,
        dir: &std::path::Path,
    ) -> Result<std::path::PathBuf> {
        let glossary = self
            .glossary(course_id, module_id, &MediaUrls::none())
            .await?;
        let file = glossary
            .data
            .entries
            .into_iter()
            .find(|e| e.id == entry_id)
            .and_then(|e| e.attachments.into_iter().find(|a| a.name == name))
            .ok_or(Error::FileNotFound)?;
        self.download_file(&file, &dir.join(entry_id.to_string()))
            .await
    }
}
