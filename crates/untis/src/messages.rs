//! Untis-Mitteilungen (Posteingang) und Nachrichten des Tages.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, Result, Session, cache, untis_date};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: i64,
    pub subject: String,
    pub preview: String,
    pub sender: String,
    /// ISO-Zeitpunkt wie geliefert (Ortszeit der Schule).
    pub sent: String,
    pub read: bool,
    pub attachments: bool,
    /// Bei gesendeten Mitteilungen: an wen.
    pub recipients: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetail {
    pub id: i64,
    pub subject: String,
    /// Bereinigtes HTML.
    pub html: String,
    pub sender: String,
    /// Für Antworten; nicht jede Version liefert sie.
    pub sender_id: Option<i64>,
    pub sent: String,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub name: String,
    /// Direkter Link, falls WebUntis einen mitliefert.
    pub url: Option<String>,
}

/// Mögliche Empfängerin oder möglicher Empfänger einer neuen Mitteilung.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipient {
    pub id: i64,
    pub name: String,
    /// Gruppe wie geliefert, z. B. `TEACHER`.
    pub group: String,
    pub tags: Vec<String>,
}

/// Was beim Schreiben erlaubt ist.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Compose {
    pub allowed: bool,
    pub recipients: Vec<Recipient>,
    /// Bytes je Datei.
    pub max_file_size: Option<i64>,
    pub max_file_count: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outgoing {
    pub subject: String,
    pub content: String,
    pub recipients: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayNews {
    pub subject: String,
    /// Bereinigtes HTML.
    pub html: String,
}

/// Neue, ungelesene Mitteilung für Benachrichtigungen.
#[derive(Debug, Clone, Serialize)]
pub struct NewMessage {
    pub sender: String,
    pub subject: String,
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_owned()
}

fn clean(html: &str) -> String {
    // Reiner Text bekommt Zeilenumbrüche, HTML bleibt (bereinigt) erhalten.
    let tagged = html
        .as_bytes()
        .windows(2)
        .any(|w| w[0] == b'<' && (w[1].is_ascii_alphabetic() || w[1] == b'/'));
    let html = if tagged {
        html.to_owned()
    } else {
        html_escape::encode_text(html).replace('\n', "<br>")
    };
    ammonia::Builder::default()
        .link_rel(Some("noopener noreferrer"))
        .clean(&html)
        .to_string()
}

fn message(v: &Value) -> Option<Message> {
    Some(Message {
        id: v["id"].as_i64()?,
        subject: str_of(&v["subject"]),
        preview: crate::plain(v["contentPreview"].as_str().unwrap_or_default()),
        sender: str_of(&v["sender"]["displayName"]),
        sent: str_of(&v["sentDateTime"]),
        read: v["isMessageRead"]
            .as_bool()
            .or(v["read"].as_bool())
            .unwrap_or(true),
        attachments: v["hasAttachments"].as_bool().unwrap_or(false),
        recipients: recipients(v),
    })
}

/// Empfänger gesendeter Mitteilungen; je nach Version anders benannt.
fn recipients(v: &Value) -> String {
    let names: Vec<String> = ["recipients", "recipientPersons", "recipientGroups"]
        .iter()
        .filter_map(|k| v[*k].as_array())
        .flatten()
        .filter_map(|r| {
            r["displayName"]
                .as_str()
                .or(r["name"].as_str())
                .map(str::to_owned)
        })
        .collect();
    if !names.is_empty() {
        return names.join(", ");
    }
    v["recipientCount"]
        .as_i64()
        .map(|n| n.to_string())
        .unwrap_or_default()
}

fn text_of(v: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| match &v[*k] {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_owned()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

/// Anhänge einer Mitteilung; Feldnamen unterscheiden sich je nach Version.
fn attachments(v: &Value) -> Vec<Attachment> {
    let list = [
        "attachments",
        "storageAttachments",
        "fileAttachments",
        "attachmentList",
        "files",
    ]
    .iter()
    .filter_map(|k| v[*k].as_array())
    .flatten();
    let mut out: Vec<Attachment> = Vec::new();
    for a in list {
        let id =
            text_of(a, &["fileId", "attachmentId", "fileAttachmentId", "id"]).unwrap_or_default();
        let name = text_of(
            a,
            &[
                "fileRegularName",
                "fileName",
                "regularName",
                "name",
                "displayName",
            ],
        )
        .unwrap_or_default();
        if (id.is_empty() && name.is_empty()) || out.iter().any(|o| o.id == id && o.name == name) {
            continue;
        }
        out.push(Attachment {
            name: if name.is_empty() {
                format!("Anhang {id}")
            } else {
                name
            },
            url: text_of(a, &["downloadUrl", "storageUrl", "url", "src", "link"]),
            id,
        });
    }
    out
}

/// Der gelieferte Eintrag eines Anhangs, für Fehlermeldungen; Links mit
/// Signatur werden gekürzt.
fn raw_attachment(v: &Value, id: &str) -> String {
    let entry = v
        .as_object()
        .into_iter()
        .flat_map(|o| o.values())
        .filter_map(Value::as_array)
        .flatten()
        .find(|a| {
            text_of(a, &["fileId", "attachmentId", "fileAttachmentId", "id"]).as_deref() == Some(id)
        });
    shorten(&entry.map(Value::to_string).unwrap_or_default())
}

fn shorten(text: &str) -> String {
    let text = text.split('?').next().unwrap_or_default();
    if text.chars().count() > 400 {
        format!("{}…", text.chars().take(400).collect::<String>())
    } else {
        text.to_owned()
    }
}

/// Dateiname ohne Pfadteile und unter Windows verbotene Zeichen.
fn safe_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if "<>:\"/\\|?*".contains(c) || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    if cleaned.is_empty() {
        "Anhang".into()
    } else {
        cleaned
    }
}

/// Empfängerlisten: je nach Version Gruppen mit Personen oder flach.
fn recipient_list(v: &Value) -> Vec<Recipient> {
    let groups = v
        .as_array()
        .or(v["groups"].as_array())
        .or(v["persons"].as_array())
        .cloned()
        .unwrap_or_default();
    let mut out: Vec<Recipient> = Vec::new();
    let mut add = |p: &Value, group: &str| {
        let Some(id) = p["userId"].as_i64().or(p["id"].as_i64()) else {
            return;
        };
        if out.iter().any(|r| r.id == id) {
            return;
        }
        out.push(Recipient {
            id,
            name: p["displayName"]
                .as_str()
                .or(p["name"].as_str())
                .unwrap_or_default()
                .to_owned(),
            group: group.to_owned(),
            tags: p["tags"]
                .as_array()
                .map(|t| {
                    t.iter()
                        .filter_map(|t| t.as_str().or(t["name"].as_str()).map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        });
    };
    for g in &groups {
        match g["persons"].as_array() {
            Some(persons) => {
                let group = g["type"].as_str().unwrap_or_default();
                persons.iter().for_each(|p| add(p, group));
            }
            None => add(g, g["type"].as_str().unwrap_or_default()),
        }
    }
    out.sort_by_key(|r| r.name.to_lowercase());
    out
}

/// Fehlermeldung aus einer abgelehnten Anfrage.
fn send_error(status: u16, body: &str) -> Error {
    let v: Value = serde_json::from_str(body).unwrap_or_default();
    let mut parts: Vec<String> = v["validationErrors"]
        .as_array()
        .map(|e| {
            e.iter()
                .filter_map(|e| e["errorMessage"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    if parts.is_empty() {
        if let Some(m) = v["errorMessage"].as_str().or(v["message"].as_str()) {
            parts.push(m.to_owned());
        }
    }
    if parts.is_empty() {
        return Error::Response(format!("HTTP {status}"));
    }
    Error::Untis {
        code: status.into(),
        message: parts.join("\n"),
    }
}

const SEEN: &str = "messages.seen";

impl Session<'_> {
    pub async fn messages(&self) -> Result<Vec<Message>> {
        let inbox: Value = self
            .rest(true, "WebUntis/api/rest/view/v1/messages", &[])
            .await?;
        Ok(inbox["incomingMessages"]
            .as_array()
            .map(|list| list.iter().filter_map(message).collect())
            .unwrap_or_default())
    }

    /// Gesendete Mitteilungen.
    pub async fn sent_messages(&self) -> Result<Vec<Message>> {
        let sent: Value = self
            .rest(true, "WebUntis/api/rest/view/v1/messages/sent", &[])
            .await?;
        let list = ["sentMessages", "messages"]
            .iter()
            .find_map(|k| sent[*k].as_array())
            .or(sent.as_array());
        Ok(list
            .map(|list| list.iter().filter_map(message).collect())
            .unwrap_or_default())
    }

    /// Öffnet eine Mitteilung; WebUntis vermerkt sie dabei als gelesen.
    pub async fn message(&self, id: i64) -> Result<MessageDetail> {
        let v: Value = self
            .rest(
                true,
                &format!("WebUntis/api/rest/view/v1/messages/{id}"),
                &[],
            )
            .await?;
        let content = ["content", "contentText", "contentPreview"]
            .iter()
            .find_map(|k| v[*k].as_str().filter(|c| !c.is_empty()))
            .unwrap_or_default();
        Ok(MessageDetail {
            id,
            subject: str_of(&v["subject"]),
            html: clean(content),
            sender: str_of(&v["sender"]["displayName"]),
            sender_id: v["sender"]["userId"]
                .as_i64()
                .or(v["sender"]["id"].as_i64()),
            sent: str_of(&v["sentDateTime"]),
            attachments: attachments(&v),
        })
    }

    /// Berechtigungen und Empfänger zum Schreiben.
    pub async fn compose(&self) -> Result<Compose> {
        let permissions: Value = self
            .rest(true, "WebUntis/api/rest/view/v1/messages/permissions", &[])
            .await?;
        let options = permissions["recipientOptions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let mut recipients = Vec::new();
        if !options.is_empty() {
            for version in ["v2", "v1"] {
                let path =
                    format!("WebUntis/api/rest/view/{version}/messages/recipients/static/persons");
                match self.rest::<Value>(true, &path, &[]).await {
                    Ok(v) => {
                        recipients = recipient_list(&v);
                        break;
                    }
                    Err(Error::Network(e)) if e.status().is_none() => {
                        return Err(Error::Network(e));
                    }
                    Err(_) => {}
                }
            }
        }
        Ok(Compose {
            allowed: !options.is_empty() && !recipients.is_empty(),
            recipients,
            max_file_size: permissions["maxFileSize"].as_i64(),
            max_file_count: permissions["maxFileCount"].as_i64(),
        })
    }

    /// Sendet eine Mitteilung wie die Weboberfläche (mit Anhängen).
    pub async fn send_message(&self, message: Outgoing, files: &[(String, Vec<u8>)]) -> Result<()> {
        let permissions: Value = self
            .rest(true, "WebUntis/api/rest/view/v1/messages/permissions", &[])
            .await?;
        let options: Vec<&str> = permissions["recipientOptions"]
            .as_array()
            .map(|o| o.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let option = options
            .iter()
            .find(|o| **o == "TEACHER")
            .or(options.first())
            .ok_or_else(|| Error::Untis {
                code: 403,
                message: "Die Schule erlaubt keine Mitteilungen".into(),
            })?;
        let request = serde_json::json!({
            "subject": message.subject,
            "content": message.content,
            "recipientOption": option,
            "recipientPersons": message.recipients.iter().map(|id| serde_json::json!({ "id": id })).collect::<Vec<_>>(),
            "recipientGroups": [],
        })
        .to_string();
        let mut extra = Vec::new();
        if let Ok(year) = self
            .rpc::<Value>("getCurrentSchoolyear", Value::Object(Default::default()))
            .await
        {
            if let Some(id) = year["id"].as_i64() {
                extra.push(("X-Webuntis-Api-School-Year-Id", id.to_string()));
            }
        }
        let mut last = (0, String::new());
        for version in ["v2", "v1"] {
            let path = format!("WebUntis/api/rest/view/{version}/messages");
            last = self.post_form(&path, &extra, &request, files).await?;
            if (200..300).contains(&last.0) {
                return Ok(());
            }
            if !matches!(last.0, 404 | 405 | 500) {
                break;
            }
        }
        Err(send_error(last.0, &last.1))
    }

    /// Lädt einen Anhang nach `dir` und liefert den Pfad. Ohne mitgelieferten
    /// Link werden mögliche Pfade der Weboberfläche probiert; schlägt alles
    /// fehl, nennt der Fehler jeden Versuch und die gelieferten Daten.
    pub async fn attachment(
        &self,
        message_id: i64,
        attachment_id: &str,
        dir: &std::path::Path,
    ) -> Result<std::path::PathBuf> {
        let v: Value = self
            .rest(
                true,
                &format!("WebUntis/api/rest/view/v1/messages/{message_id}"),
                &[],
            )
            .await?;
        let found = attachments(&v)
            .into_iter()
            .find(|a| a.id == attachment_id)
            .ok_or_else(|| Error::Response("Anhang nicht gefunden".into()))?;
        let raw = raw_attachment(&v, attachment_id);

        // Die Weboberfläche lädt direkt aus dem Dateispeicher:
        // storage.webuntis.com/untis-sts-prod/<Mandant>/<Anhang>.
        let mut candidates: Vec<String> = found.url.iter().cloned().collect();
        match self.tenant().await? {
            Some(tenant) => candidates.push(format!(
                "{}/untis-sts-prod/{tenant}/{attachment_id}",
                crate::client::storage_base()
            )),
            None => {
                return Err(Error::Untis {
                    code: 404,
                    message: format!(
                        "„{}“: Im Token fehlt die Schulnummer (tenant_id).\nDaten: {raw}",
                        found.name
                    ),
                });
            }
        }
        let mut tried = Vec::new();
        let mut bytes = None;
        'outer: for url in &candidates {
            let (status, kind, body) = match self.get_bytes(url).await {
                Ok(r) => r,
                Err(Error::Network(e)) if e.status().is_none() && !e.is_connect() => {
                    tried.push(format!("{url} → {e}"));
                    continue;
                }
                Err(e) => return Err(e),
            };
            let kind_text = kind.clone().unwrap_or_else(|| "?".into());
            tried.push(format!("{url} → {status} {kind_text}"));
            if !(200..300).contains(&status) || kind_text.starts_with("text/html") {
                continue;
            }
            // Manche Pfade liefern nur einen Link zum Dateispeicher.
            if kind_text.contains("json") || kind_text.starts_with("text/plain") {
                let text = String::from_utf8_lossy(&body).trim().to_owned();
                let v: Value = serde_json::from_str(&text).unwrap_or_default();
                let link = text_of(&v, &["downloadUrl", "storageUrl", "url", "src"])
                    .or(v.as_str().map(str::to_owned))
                    .or(
                        (text.starts_with("http") && !text.contains(char::is_whitespace))
                            .then_some(text.clone()),
                    );
                let Some(link) = link else {
                    tried.push(format!("  Antwort: {}", shorten(&text)));
                    continue;
                };
                let (status, kind, body) = self.get_bytes(&link).await?;
                tried.push(format!(
                    "  → {} → {status} {}",
                    shorten(&link),
                    kind.unwrap_or_default()
                ));
                if (200..300).contains(&status) {
                    bytes = Some(body);
                    break 'outer;
                }
                continue;
            }
            bytes = Some(body);
            break;
        }
        let Some(bytes) = bytes else {
            return Err(Error::Untis {
                code: 404,
                message: format!(
                    "„{}“ ließ sich nicht laden.\nDaten: {}\nVersucht:\n{}",
                    found.name,
                    raw,
                    tried.join("\n")
                ),
            });
        };
        let dir = dir.join(message_id.to_string());
        std::fs::create_dir_all(&dir).map_err(|e| Error::Response(e.to_string()))?;
        let path = dir.join(safe_name(&found.name));
        std::fs::write(&path, bytes).map_err(|e| Error::Response(e.to_string()))?;
        Ok(path)
    }

    /// Nachrichten des Tages (Startseite von WebUntis).
    pub async fn day_news(&self, date: NaiveDate) -> Result<Vec<DayNews>> {
        let v: Value = self
            .rest(
                false,
                "WebUntis/api/public/news/newsWidgetData",
                &[("date", untis_date(date))],
            )
            .await?;
        Ok(v["data"]["messagesOfDay"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|m| DayNews {
                        subject: str_of(&m["subject"]),
                        html: clean(m["text"].as_str().unwrap_or_default()),
                    })
                    .filter(|n| !n.subject.is_empty() || !n.html.is_empty())
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Ungelesene Mitteilungen, die seit dem letzten Aufruf neu sind; beim
    /// ersten Aufruf keine. Liefert dazu die Zahl der ungelesenen.
    pub async fn check_messages(&self) -> Result<(i64, Vec<NewMessage>)> {
        let list = self.messages().await?;
        let seen: Option<(i64, i64)> = cache::load(self.db(), SEEN)?;
        let newest = list.iter().map(|m| m.id).max().unwrap_or(0);
        cache::store(self.db(), SEEN, &newest.max(seen.map_or(0, |s| s.0)))?;
        let unread = list.iter().filter(|m| !m.read).count() as i64;
        let Some((seen, _)) = seen else {
            return Ok((unread, Vec::new()));
        };
        Ok((
            unread,
            list.iter()
                .filter(|m| !m.read && m.id > seen)
                .map(|m| NewMessage {
                    sender: m.sender.clone(),
                    subject: m.subject.clone(),
                })
                .collect(),
        ))
    }

    /// Mitteilungen in WebUntis im Browser (z. B. zum Antworten).
    pub fn web_url(&self) -> Result<String> {
        let school = self.school()?;
        Ok(format!(
            "https://{}/WebUntis/?school={}#/basic/messages",
            school.server,
            url::form_urlencoded::byte_serialize(school.login_name.as_bytes()).collect::<String>()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipients_grouped_or_flat() {
        let grouped = serde_json::json!([
            {"type": "TEACHER", "persons": [
                {"userId": 2, "displayName": "Müller", "tags": ["MÜ"]},
                {"userId": 1, "displayName": "Adler", "tags": []}
            ]},
            {"type": "STAFF", "persons": [{"userId": 2, "displayName": "Müller"}]}
        ]);
        let list = recipient_list(&grouped);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Adler");
        assert_eq!(list[1].tags, vec!["MÜ"]);
        let flat = serde_json::json!({"persons": [{"id": 5, "name": "X"}]});
        assert_eq!(recipient_list(&flat)[0].id, 5);
    }

    #[test]
    fn send_errors_readable() {
        let e = send_error(
            400,
            r#"{"validationErrors":[{"errorMessage":"Betreff fehlt"}]}"#,
        );
        assert_eq!(e.to_string(), "Betreff fehlt");
        assert_eq!(
            send_error(500, "").to_string(),
            "Unerwartete Antwort von WebUntis: HTTP 500"
        );
    }

    #[test]
    fn attachments_in_all_shapes() {
        let v = serde_json::json!({
            "attachments": [{"id": 1, "name": "a.pdf"}],
            "storageAttachments": [{"fileId": "x9", "fileRegularName": "b.docx", "downloadUrl": "https://s3/b"}]
        });
        let list = attachments(&v);
        assert_eq!(list.len(), 2);
        assert_eq!((list[0].id.as_str(), list[0].name.as_str()), ("1", "a.pdf"));
        assert_eq!(list[1].url.as_deref(), Some("https://s3/b"));
        assert_eq!(safe_name("../x:y?.pdf"), "_x_y_.pdf");
    }

    #[test]
    fn plain_text_keeps_lines() {
        assert_eq!(clean("Hallo\nWelt <3"), "Hallo<br>Welt &lt;3");
        assert_eq!(
            clean("<p onclick=\"x\">Hi</p><script>x</script>"),
            "<p>Hi</p>"
        );
    }
}
