//! Board (Plugin `mod_board`): Spalten mit Notizen lesen.
//!
//! Das Plugin bietet seine Funktionen nur der Weboberfläche an, daher läuft
//! der Zugriff über die Browser-Sitzung wie bei Moodle-Mail.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::activities::Cached;
use crate::html::{MediaUrls, sanitize};
use crate::mail::MailState;
use crate::{Error, Result, Session};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub id: i64,
    pub name: String,
    pub notes: Vec<Note>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: i64,
    pub heading: String,
    /// Bereinigtes HTML aus Text und Anhang (Bild, Video-Link, Link).
    pub html: String,
    pub created: i64,
    pub rating: i64,
}

/// Das Plugin speichert Texte HTML-kodiert.
fn decoded(v: &Value) -> String {
    html_escape::decode_html_entities(v.as_str().unwrap_or_default()).into_owned()
}

fn attr(text: &str) -> String {
    html_escape::encode_double_quoted_attribute(text).into_owned()
}

/// Inhalt einer Notiz als HTML (noch unbereinigt).
fn note_html(n: &Value) -> String {
    let content = decoded(&n["content"]);
    let mut html = if content.contains('<') {
        content
    } else {
        crate::assign::text_to_html(&content)
    };
    let (info, url) = (decoded(&n["info"]), decoded(&n["url"]));
    if !url.is_empty() {
        let label = if info.is_empty() {
            url.clone()
        } else {
            info.clone()
        };
        // 1 Video (YouTube), 2 Bild, 3 Link.
        html.push_str(&match n["type"].as_i64().unwrap_or(0) {
            2 => format!(r#"<p><img src="{}" alt="{}"></p>"#, attr(&url), attr(&info)),
            1 => format!(
                r#"<p><iframe src="{}" title="{}"></iframe></p>"#,
                attr(&url),
                attr(&label)
            ),
            _ => format!(
                r#"<p><a href="{}">{}</a></p>"#,
                attr(&url),
                html_escape::encode_text(&label)
            ),
        });
    }
    html
}

fn parse(v: &Value) -> Board {
    // Moodle liefert Listen hier je nach Version als Feld oder als Objekt
    // mit Kennungen als Schlüsseln.
    fn items(v: &Value) -> Vec<&Value> {
        let mut list: Vec<&Value> = match v {
            Value::Array(a) => a.iter().collect(),
            Value::Object(o) => o.values().collect(),
            _ => Vec::new(),
        };
        list.sort_by_key(|x| (x["sortorder"].as_i64().unwrap_or(0), x["id"].as_i64()));
        list
    }
    Board {
        columns: items(v)
            .into_iter()
            .filter_map(|c| {
                Some(Column {
                    id: c["id"].as_i64()?,
                    name: decoded(&c["name"]),
                    notes: items(&c["notes"])
                        .into_iter()
                        .filter_map(|n| {
                            Some(Note {
                                id: n["id"].as_i64()?,
                                heading: decoded(&n["heading"]),
                                html: note_html(n),
                                created: n["timecreated"].as_i64().unwrap_or(0),
                                rating: n["rating"].as_i64().unwrap_or(0),
                            })
                        })
                        .collect(),
                })
            })
            .collect(),
    }
}

impl Session<'_> {
    pub async fn board(
        &self,
        state: &MailState,
        course_id: i64,
        module_id: i64,
        media: &MediaUrls,
    ) -> Result<Cached<Board>> {
        let id = self
            .module(course_id, module_id)?
            .instance
            .ok_or(Error::FileNotFound)?;
        let base: Url = self.base()?;
        let mut cached = self
            .cached(&format!("board:{module_id}"), || async {
                let v: Value = self
                    .web_call(
                        state,
                        &format!("mod/board/view.php?id={module_id}"),
                        "mod_board_get_board",
                        json!({ "id": id }),
                    )
                    .await?;
                Ok(parse(&v))
            })
            .await?;
        for note in cached.data.columns.iter_mut().flat_map(|c| &mut c.notes) {
            note.html = sanitize(&note.html, &base, media);
        }
        Ok(cached)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_columns_and_notes() {
        let v = json!({"5": {"id": 5, "name": "Ideen &amp; Fragen", "locked": false, "notes": {
            "9": {"id": 9, "userid": 2, "heading": "Bild", "content": "Hallo\nWelt", "type": 2,
                  "info": "Katze", "url": "https://m.example/pluginfile.php/1/mod_board/images/9/k.jpg",
                  "timecreated": 1, "rating": 3, "sortorder": 0},
            "10": {"id": 10, "heading": "", "content": "", "type": 3, "info": "", "url": "https://x.example"}
        }}});
        let board = parse(&v);
        assert_eq!(board.columns[0].name, "Ideen & Fragen");
        let notes = &board.columns[0].notes;
        assert_eq!(notes.len(), 2);
        assert!(
            notes[0]
                .html
                .starts_with("<p>Hallo<br>Welt</p><p><img src=")
        );
        assert_eq!(notes[0].rating, 3);
        assert_eq!(
            notes[1].html,
            r#"<p><a href="https://x.example">https://x.example</a></p>"#
        );
    }
}
