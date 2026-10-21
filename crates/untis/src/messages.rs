//! Untis-Mitteilungen (Posteingang) und Nachrichten des Tages.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Result, Session, cache, untis_date};

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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDetail {
    pub id: i64,
    pub subject: String,
    /// Bereinigtes HTML.
    pub html: String,
    pub sender: String,
    pub sent: String,
    pub attachments: Vec<String>,
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
        read: v["isMessageRead"].as_bool().unwrap_or(true),
        attachments: v["hasAttachments"].as_bool().unwrap_or(false),
    })
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
            sent: str_of(&v["sentDateTime"]),
            attachments: v["attachments"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|a| a["name"].as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
        })
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
    fn plain_text_keeps_lines() {
        assert_eq!(clean("Hallo\nWelt <3"), "Hallo<br>Welt &lt;3");
        assert_eq!(
            clean("<p onclick=\"x\">Hi</p><script>x</script>"),
            "<p>Hi</p>"
        );
    }
}
