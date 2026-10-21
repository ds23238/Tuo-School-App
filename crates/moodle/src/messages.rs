//! Moodle-Mitteilungen: Unterhaltungen (Chats) und Benachrichtigungen.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::assign::text_to_html;
use crate::html::{MediaUrls, sanitize};
use crate::mail::MailState;
use crate::{Error, Result, Session, plain};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: i64,
    pub name: String,
    pub subtitle: Option<String>,
    pub image: Option<String>,
    /// `individual`, `group` oder `self` (Notizen an sich selbst).
    pub kind: &'static str,
    pub unread: i64,
    pub member_count: i64,
    pub favourite: bool,
    pub muted: bool,
    pub members: Vec<Member>,
    pub last: Option<LastMessage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastMessage {
    pub preview: String,
    pub time: i64,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: i64,
    pub name: String,
    pub image: Option<String>,
    pub can_message: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: i64,
    pub from: i64,
    pub from_name: Option<String>,
    pub html: String,
    pub time: i64,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: i64,
    /// Älteste zuerst.
    pub messages: Vec<ChatMessage>,
    /// Es gibt ältere Nachrichten als die geladenen.
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: i64,
    pub subject: String,
    pub html: String,
    pub time: i64,
    pub read: bool,
    pub url: Option<String>,
    pub url_name: Option<String>,
    pub component: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub chats: i64,
    pub notifications: i64,
    /// `None`, wenn die Seite kein Moodle-Mail hat.
    pub mail: Option<i64>,
}

/// Neu eingetroffen seit der letzten Prüfung, für Desktop-Benachrichtigungen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct News {
    /// `chat`, `notification` oder `mail`.
    pub kind: &'static str,
    pub title: String,
    pub body: String,
}

/// Neueste bereits gemeldete Kennungen.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Seen {
    chat: i64,
    notification: i64,
    mail: i64,
}

pub(crate) const SEEN: &str = "moodle.messages.seen";

#[derive(Debug, Deserialize)]
pub(crate) struct RemoteConversation {
    id: i64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    subname: Option<String>,
    #[serde(default)]
    imageurl: Option<String>,
    #[serde(rename = "type")]
    kind: i64,
    #[serde(default)]
    membercount: Option<i64>,
    #[serde(default)]
    isfavourite: bool,
    #[serde(default)]
    ismuted: bool,
    #[serde(default)]
    unreadcount: Option<i64>,
    #[serde(default)]
    members: Vec<RemoteMember>,
    #[serde(default)]
    pub(crate) messages: Vec<RemoteMessage>,
}

#[derive(Debug, Deserialize)]
struct RemoteMember {
    id: i64,
    fullname: String,
    #[serde(default)]
    profileimageurl: Option<String>,
    #[serde(default)]
    canmessage: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RemoteMessage {
    pub(crate) id: i64,
    useridfrom: i64,
    text: String,
    timecreated: i64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Conversations {
    pub(crate) conversations: Vec<RemoteConversation>,
}

#[derive(Debug, Deserialize)]
struct ConversationMessages {
    #[serde(default)]
    members: Vec<RemoteMember>,
    messages: Vec<RemoteMessage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RemoteNotification {
    pub(crate) id: i64,
    pub(crate) subject: String,
    #[serde(default)]
    smallmessage: Option<String>,
    #[serde(default)]
    fullmessagehtml: Option<String>,
    #[serde(default)]
    fullmessage: Option<String>,
    #[serde(default)]
    contexturl: Option<String>,
    #[serde(default)]
    contexturlname: Option<String>,
    timecreated: i64,
    #[serde(default)]
    pub(crate) read: bool,
    #[serde(default)]
    component: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Notifications {
    pub(crate) notifications: Vec<RemoteNotification>,
}

/// Erste Zeile einer Nachricht als Vorschau.
fn preview(html: &str) -> String {
    let text = plain(&html.replace("<br", "\n<br").replace("</p>", "</p>\n"));
    let mut chars = text.chars();
    let short: String = chars.by_ref().take(120).collect();
    if chars.next().is_some() {
        format!("{short} …")
    } else {
        short
    }
}

/// Profil- oder Gruppenbild; Moodles Platzhalter (`theme/image.php/…/u/f1`)
/// fällt weg, die Oberfläche zeigt dann Initialen.
pub(crate) fn image(url: Option<&str>, media: &MediaUrls) -> Option<String> {
    let url = Url::parse(url?).ok()?;
    let placeholder = url.path().contains("/theme/image.php/")
        || url.path().contains("/pix/")
        || url.path().ends_with("/u/f1")
        || url.path().ends_with("/u/f2")
        || url.path().ends_with("/u/f3");
    (matches!(url.scheme(), "http" | "https") && !placeholder).then(|| media.url(&url))
}

fn member(m: &RemoteMember, media: &MediaUrls) -> Member {
    Member {
        id: m.id,
        name: plain(&m.fullname),
        image: image(m.profileimageurl.as_deref(), media),
        can_message: m.canmessage.unwrap_or(true),
    }
}

impl Session<'_> {
    pub async fn conversations(&self, media: &MediaUrls) -> Result<Vec<Conversation>> {
        let me = self.user_id().await?;
        let remote: Conversations = self
            .call(
                "core_message_get_conversations",
                &[
                    ("userid", me.to_string()),
                    ("limitnum", "100".into()),
                    ("mergeself", "1".into()),
                ],
            )
            .await?;
        Ok(remote
            .conversations
            .into_iter()
            .map(|c| conversation(c, me, media))
            .collect())
    }

    /// Nachrichten einer Unterhaltung; `before` lädt ältere nach.
    pub async fn chat(
        &self,
        conversation_id: i64,
        before: usize,
        media: &MediaUrls,
    ) -> Result<Chat> {
        const PAGE: usize = 50;
        let me = self.user_id().await?;
        let base = self.base()?;
        let remote: ConversationMessages = self
            .call(
                "core_message_get_conversation_messages",
                &[
                    ("currentuserid", me.to_string()),
                    ("convid", conversation_id.to_string()),
                    ("limitfrom", before.to_string()),
                    ("limitnum", (PAGE + 1).to_string()),
                    ("newest", "1".into()),
                ],
            )
            .await?;
        let has_more = remote.messages.len() > PAGE;
        let names: std::collections::HashMap<i64, String> = remote
            .members
            .iter()
            .map(|m| (m.id, plain(&m.fullname)))
            .collect();
        let mut messages: Vec<ChatMessage> = remote
            .messages
            .into_iter()
            .take(PAGE)
            .map(|m| ChatMessage {
                id: m.id,
                from: m.useridfrom,
                from_name: names.get(&m.useridfrom).cloned(),
                html: sanitize(&m.text, &base, media),
                time: m.timecreated,
                mine: m.useridfrom == me,
            })
            .collect();
        messages.reverse();
        Ok(Chat {
            id: conversation_id,
            messages,
            has_more,
        })
    }

    pub async fn mark_chat_read(&self, conversation_id: i64) -> Result<()> {
        let me = self.user_id().await?;
        let _: serde_json::Value = self
            .call(
                "core_message_mark_all_conversation_messages_as_read",
                &[
                    ("userid", me.to_string()),
                    ("conversationid", conversation_id.to_string()),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn send_chat(
        &self,
        conversation_id: i64,
        text: &str,
        media: &MediaUrls,
    ) -> Result<ChatMessage> {
        let me = self.user_id().await?;
        let base = self.base()?;
        let sent: Vec<RemoteMessage> = self
            .call(
                "core_message_send_messages_to_conversation",
                &[
                    ("conversationid", conversation_id.to_string()),
                    ("messages[0][text]", text_to_html(text)),
                    ("messages[0][textformat]", "1".into()),
                ],
            )
            .await?;
        let m = sent.into_iter().next().ok_or_else(|| Error::Moodle {
            code: "send".into(),
            message: String::new(),
        })?;
        Ok(ChatMessage {
            id: m.id,
            from: me,
            from_name: None,
            html: sanitize(&m.text, &base, media),
            time: m.timecreated,
            mine: true,
        })
    }

    /// Erste Nachricht an jemanden ohne bestehende Unterhaltung; liefert
    /// deren Kennung.
    pub async fn send_to_user(&self, user_id: i64, text: &str) -> Result<i64> {
        #[derive(Deserialize)]
        struct Sent {
            msgid: i64,
            #[serde(default)]
            conversationid: Option<i64>,
            #[serde(default)]
            errormessage: Option<String>,
        }
        let sent: Vec<Sent> = self
            .call(
                "core_message_send_instant_messages",
                &[
                    ("messages[0][touserid]", user_id.to_string()),
                    ("messages[0][text]", text_to_html(text)),
                    ("messages[0][textformat]", "1".into()),
                ],
            )
            .await?;
        match sent.into_iter().next() {
            Some(Sent {
                msgid,
                conversationid: Some(id),
                ..
            }) if msgid > 0 => Ok(id),
            other => Err(Error::Moodle {
                code: "send".into(),
                message: other
                    .and_then(|s| s.errormessage)
                    .map(|m| plain(&m))
                    .unwrap_or_default(),
            }),
        }
    }

    pub async fn search_people(&self, text: &str, media: &MediaUrls) -> Result<Vec<Member>> {
        #[derive(Deserialize)]
        struct Found {
            contacts: Vec<RemoteMember>,
            noncontacts: Vec<RemoteMember>,
        }
        let me = self.user_id().await?;
        let found: Found = self
            .call(
                "core_message_message_search_users",
                &[
                    ("userid", me.to_string()),
                    ("search", text.to_owned()),
                    ("limitnum", "30".into()),
                ],
            )
            .await?;
        Ok(found
            .contacts
            .iter()
            .chain(&found.noncontacts)
            .filter(|m| m.id != me)
            .map(|m| member(m, media))
            .collect())
    }

    pub async fn notifications(&self, media: &MediaUrls) -> Result<Vec<Notification>> {
        let base = self.base()?;
        let remote = self.raw_notifications(50).await?;
        Ok(remote
            .into_iter()
            .map(|n| {
                let html = match n.fullmessagehtml.as_deref().map(str::trim) {
                    Some(h) if !h.is_empty() => h.to_owned(),
                    _ => text_to_html(
                        n.smallmessage
                            .as_deref()
                            .or(n.fullmessage.as_deref())
                            .unwrap_or_default(),
                    ),
                };
                Notification {
                    id: n.id,
                    subject: plain(&n.subject),
                    html: sanitize(&html, &base, media),
                    time: n.timecreated,
                    read: n.read,
                    url: n.contexturl.filter(|u| !u.is_empty()),
                    url_name: n
                        .contexturlname
                        .map(|n| plain(&n))
                        .filter(|n| !n.is_empty()),
                    component: n.component,
                }
            })
            .collect())
    }

    pub(crate) async fn raw_notifications(&self, limit: usize) -> Result<Vec<RemoteNotification>> {
        let me = self.user_id().await?;
        let remote: Notifications = self
            .call(
                "message_popup_get_popup_notifications",
                &[
                    ("useridto", me.to_string()),
                    ("newestfirst", "1".into()),
                    ("limit", limit.to_string()),
                ],
            )
            .await?;
        Ok(remote.notifications)
    }

    pub async fn mark_notification_read(&self, id: i64) -> Result<()> {
        let _: serde_json::Value = self
            .call(
                "core_message_mark_notification_read",
                &[("notificationid", id.to_string())],
            )
            .await?;
        Ok(())
    }

    pub async fn mark_all_notifications_read(&self) -> Result<()> {
        let me = self.user_id().await?;
        let _: serde_json::Value = self
            .call(
                "core_message_mark_all_notifications_as_read",
                &[("useridto", me.to_string())],
            )
            .await?;
        Ok(())
    }

    /// Ungelesene Chats und Benachrichtigungen (ohne Mail).
    pub async fn message_counts(&self) -> Result<Counts> {
        let me = self.user_id().await?.to_string();
        let chats: i64 = self
            .call(
                "core_message_get_unread_conversations_count",
                &[("useridto", me.clone())],
            )
            .await?;
        let notifications: i64 = self
            .call(
                "message_popup_get_unread_popup_notification_count",
                &[("useridto", me)],
            )
            .await?;
        Ok(Counts {
            chats,
            notifications,
            mail: None,
        })
    }

    pub(crate) async fn raw_conversations(&self, limit: usize) -> Result<Vec<RemoteConversation>> {
        let me = self.user_id().await?;
        let remote: Conversations = self
            .call(
                "core_message_get_conversations",
                &[("userid", me.to_string()), ("limitnum", limit.to_string())],
            )
            .await?;
        Ok(remote.conversations)
    }
}

impl Session<'_> {
    /// Zählt Ungelesenes und liefert, was seit dem letzten Aufruf neu ist.
    /// Beim ersten Aufruf ist nichts neu.
    pub async fn check_messages(&self, mail: &MailState) -> Result<(Counts, Vec<News>)> {
        let mut counts = self.message_counts().await?;
        let me = self.user_id().await?;
        let stored = tsa_core::settings::get_value(self.db(), SEEN)?;
        let first = stored.is_none();
        let mut seen: Seen = stored
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let mut news = Vec::new();

        if counts.notifications > 0 || first {
            for n in self.raw_notifications(5).await? {
                if !n.read && n.id > seen.notification {
                    news.push(News {
                        kind: "notification",
                        title: plain(&n.subject),
                        body: String::new(),
                    });
                }
                seen.notification = seen.notification.max(n.id);
            }
        }
        if counts.chats > 0 || first {
            for c in self.raw_conversations(5).await? {
                let unread = c.unreadcount.unwrap_or(0) > 0;
                let Some(last) = c.messages.first() else {
                    continue;
                };
                if unread && last.id > seen.chat && last.useridfrom != me {
                    let (id, text) = (last.id, preview(&last.text));
                    let view = conversation(c, me, &MediaUrls::none());
                    news.push(News {
                        kind: "chat",
                        title: view.name,
                        body: text,
                    });
                    seen.chat = seen.chat.max(id);
                } else {
                    seen.chat = seen.chat.max(last.id);
                }
            }
        }
        // Ohne Mail-Plugin oder ohne Sitzung zählen nur die anderen.
        if let Ok(Some(status)) = self.mail_status(mail).await {
            counts.mail = Some(status.unread);
            if status.unread > 0 || first {
                for (id, sender, subject) in self.unread_mail(mail).await.unwrap_or_default() {
                    if id > seen.mail {
                        news.push(News {
                            kind: "mail",
                            title: sender,
                            body: subject,
                        });
                    }
                    seen.mail = seen.mail.max(id);
                }
            }
        }
        tsa_core::settings::set_value(self.db(), SEEN, &serde_json::to_string(&seen)?)?;
        if first {
            news.clear();
        }
        Ok((counts, news))
    }
}

fn conversation(c: RemoteConversation, me: i64, media: &MediaUrls) -> Conversation {
    let members: Vec<Member> = c.members.iter().map(|m| member(m, media)).collect();
    let kind = match c.kind {
        2 => "group",
        3 => "self",
        _ => "individual",
    };
    let other = members.iter().find(|m| m.id != me).or(members.first());
    let name = c
        .name
        .as_deref()
        .map(plain)
        .filter(|n| !n.is_empty())
        .or_else(|| other.map(|m| m.name.clone()))
        .unwrap_or_default();
    let image = image(c.imageurl.as_deref(), media).or_else(|| other.and_then(|m| m.image.clone()));
    let last = c.messages.first().map(|m| LastMessage {
        preview: preview(&m.text),
        time: m.timecreated,
        mine: m.useridfrom == me,
    });
    Conversation {
        id: c.id,
        name,
        subtitle: c.subname.as_deref().map(plain).filter(|s| !s.is_empty()),
        image,
        kind,
        unread: c.unreadcount.unwrap_or(0),
        member_count: c.membercount.unwrap_or(members.len() as i64),
        favourite: c.isfavourite,
        muted: c.ismuted,
        members,
        last,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_first_lines() {
        assert_eq!(preview("<p>Hallo</p><p>Welt</p>"), "Hallo Welt");
        let long = "x".repeat(200);
        assert_eq!(preview(&long).chars().count(), 122);
    }
}
