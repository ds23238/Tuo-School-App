//! Datenbank-Aktivität: Einträge lesen. Neue Einträge legt Moodle selbst an
//! (die Felder hängen stark von der Vorlage ab).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::activities::Cached;
use crate::html::{MediaUrls, sanitize};
use crate::{Error, Result, Session, plain};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Database {
    pub id: i64,
    pub fields: Vec<Field>,
    pub entries: Vec<Entry>,
    pub can_add: bool,
    /// Neuer Eintrag in Moodle.
    pub add_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub id: i64,
    pub name: String,
    /// Moodle-Feldtyp, z. B. `text`, `picture`, `date`.
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: i64,
    pub author: String,
    pub created: i64,
    pub modified: i64,
    pub approved: bool,
    pub values: Vec<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldValue {
    pub field_id: i64,
    /// Bereinigtes HTML; bei Datumsfeldern leer.
    pub html: String,
    /// Datumsfelder: Unix-Zeit, die Oberfläche formatiert.
    pub date: Option<i64>,
}

fn escape(text: &str) -> String {
    html_escape::encode_text(text).into_owned()
}

/// Dateien über die normale Adresse: Bilder lädt die App mit Token, Links
/// öffnen im Browser mit Anmeldung.
fn browser_url(url: &str) -> String {
    url.replacen("/webservice/pluginfile.php/", "/pluginfile.php/", 1)
}

/// Inhalt eines Feldes als HTML (noch unbereinigt).
fn render(kind: &str, c: &Value) -> (String, Option<i64>) {
    let content = c["content"].as_str().unwrap_or_default();
    let content1 = c["content1"].as_str().unwrap_or_default();
    let files: Vec<&Value> = c["files"]
        .as_array()
        .map(|f| f.iter().collect())
        .unwrap_or_default();
    let file_url = |name: &str| {
        files
            .iter()
            .find(|f| f["filename"].as_str() == Some(name))
            .and_then(|f| f["fileurl"].as_str())
            .map(browser_url)
    };
    let html = match kind {
        "textarea" => content.to_owned(),
        "picture" => match file_url(content) {
            Some(url) => format!(
                r#"<a href="{0}"><img src="{0}" alt="{1}"></a>"#,
                html_escape::encode_double_quoted_attribute(&url),
                html_escape::encode_double_quoted_attribute(content1)
            ),
            None => String::new(),
        },
        "file" => match file_url(content) {
            Some(url) => format!(
                r#"<a href="{}">{}</a>"#,
                html_escape::encode_double_quoted_attribute(&url),
                escape(if content1.is_empty() {
                    content
                } else {
                    content1
                })
            ),
            None => String::new(),
        },
        "url" if !content.is_empty() => format!(
            r#"<a href="{}">{}</a>"#,
            html_escape::encode_double_quoted_attribute(content),
            escape(if content1.is_empty() {
                content
            } else {
                content1
            })
        ),
        "checkbox" | "multimenu" => escape(
            &content
                .split("##")
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(", "),
        ),
        "latlong" if !content.is_empty() => format!(
            r#"<a href="https://www.openstreetmap.org/?mlat={0}&amp;mlon={1}#map=12/{0}/{1}">{0}, {1}</a>"#,
            escape(content),
            escape(content1)
        ),
        "date" => {
            return (
                String::new(),
                content.parse::<i64>().ok().filter(|t| *t != 0),
            );
        }
        _ => escape(content).replace('\n', "<br>"),
    };
    (html, None)
}

impl Session<'_> {
    pub async fn database(
        &self,
        course_id: i64,
        module_id: i64,
        media: &MediaUrls,
    ) -> Result<Cached<Database>> {
        let id = self
            .module(course_id, module_id)?
            .instance
            .ok_or(Error::FileNotFound)?;
        let base = self.base()?;
        let mut cached = self
            .cached(&format!("data:{module_id}"), || async {
                let fields: Value = self
                    .call("mod_data_get_fields", &[("databaseid", id.to_string())])
                    .await?;
                let fields: Vec<Field> = fields["fields"]
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .filter_map(|f| {
                                Some(Field {
                                    id: f["id"].as_i64()?,
                                    name: plain(f["name"].as_str().unwrap_or_default()),
                                    kind: f["type"].as_str().unwrap_or_default().to_owned(),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let mut entries = Vec::new();
                // Seitenweise, aber gedeckelt: Riesige Datenbanken öffnet Moodle.
                for page in 0..10 {
                    let v: Value = self
                        .call(
                            "mod_data_get_entries",
                            &[
                                ("databaseid", id.to_string()),
                                ("returncontents", "1".into()),
                                ("page", page.to_string()),
                                ("perpage", "100".into()),
                            ],
                        )
                        .await?;
                    let list = v["entries"].as_array().cloned().unwrap_or_default();
                    let done = list.len() < 100;
                    for e in list {
                        let Some(entry_id) = e["id"].as_i64() else {
                            continue;
                        };
                        let values = fields
                            .iter()
                            .filter_map(|f| {
                                let c = e["contents"]
                                    .as_array()?
                                    .iter()
                                    .find(|c| c["fieldid"].as_i64() == Some(f.id))?;
                                let (html, date) = render(&f.kind, c);
                                (!html.is_empty() || date.is_some()).then_some(FieldValue {
                                    field_id: f.id,
                                    html,
                                    date,
                                })
                            })
                            .collect();
                        entries.push(Entry {
                            id: entry_id,
                            author: e["fullname"].as_str().unwrap_or_default().to_owned(),
                            created: e["timecreated"].as_i64().unwrap_or(0),
                            modified: e["timemodified"].as_i64().unwrap_or(0),
                            approved: e["approved"].as_bool().unwrap_or(true),
                            values,
                        });
                    }
                    if done {
                        break;
                    }
                }
                let access: Value = self
                    .call(
                        "mod_data_get_data_access_information",
                        &[("databaseid", id.to_string())],
                    )
                    .await
                    .unwrap_or_default();
                Ok(Database {
                    id,
                    fields,
                    entries,
                    can_add: access["canaddentry"].as_bool().unwrap_or(false)
                        && access["timeavailable"].as_bool().unwrap_or(true),
                    add_url: base
                        .join(&format!("mod/data/edit.php?d={id}"))
                        .map(|u| u.to_string())
                        .unwrap_or_default(),
                })
            })
            .await?;
        for entry in &mut cached.data.entries {
            for value in &mut entry.values {
                value.html = sanitize(&value.html, &base, media);
            }
        }
        Ok(cached)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_field_types() {
        let c = serde_json::json!({"content": "Hund##Katze", "content1": null, "files": []});
        assert_eq!(render("checkbox", &c).0, "Hund, Katze");
        let c = serde_json::json!({"content": "a.jpg", "content1": "Alt", "files": [
            {"filename": "a.jpg", "fileurl": "https://m.example/webservice/pluginfile.php/1/mod_data/content/2/a.jpg"},
            {"filename": "thumb_a.jpg", "fileurl": "x"}]});
        assert_eq!(
            render("picture", &c).0,
            r#"<a href="https://m.example/pluginfile.php/1/mod_data/content/2/a.jpg"><img src="https://m.example/pluginfile.php/1/mod_data/content/2/a.jpg" alt="Alt"></a>"#
        );
        let c = serde_json::json!({"content": "1700000000"});
        assert_eq!(render("date", &c), (String::new(), Some(1_700_000_000)));
        let c = serde_json::json!({"content": "<b>x</b>\nneu"});
        assert_eq!(render("text", &c).0, "&lt;b&gt;x&lt;/b&gt;<br>neu");
    }
}
