//! Foren: Themen und Beiträge lesen, antworten, neue Themen.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::activities::{Cached, FileRef};
use crate::html::{MediaUrls, sanitize};
use crate::{Error, Result, Session, plain};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Forum {
    pub id: i64,
    pub discussions: Vec<Discussion>,
    pub can_post: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discussion {
    /// Kennung der Diskussion (nicht des ersten Beitrags).
    pub id: i64,
    pub name: String,
    pub author: String,
    pub created: i64,
    /// Letzte Aktivität.
    pub modified: i64,
    pub replies: i64,
    pub unread: i64,
    pub pinned: bool,
    pub locked: bool,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
    pub id: i64,
    pub parent: Option<i64>,
    pub subject: String,
    pub author: String,
    pub created: i64,
    /// Bereinigtes HTML.
    pub html: String,
    pub attachments: Vec<FileRef>,
    pub unread: bool,
    pub can_reply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: i64,
    pub name: String,
    pub posts: Vec<Post>,
}

fn preview(html: &str) -> String {
    let text = plain(html);
    match text.char_indices().nth(160) {
        Some((i, _)) => format!("{}…", text[..i].trim_end()),
        None => text,
    }
}

/// Reiner Text als einfaches HTML für Beiträge.
fn text_html(text: &str) -> String {
    text.trim()
        .split("\n\n")
        .map(|p| {
            format!(
                "<p>{}</p>",
                html_escape::encode_text(p.trim()).replace('\n', "<br>")
            )
        })
        .collect()
}

impl Session<'_> {
    fn forum_id(&self, course_id: i64, module_id: i64) -> Result<i64> {
        self.module(course_id, module_id)?
            .instance
            .ok_or(Error::FileNotFound)
    }

    pub async fn forum(&self, course_id: i64, module_id: i64) -> Result<Cached<Forum>> {
        let id = self.forum_id(course_id, module_id)?;
        self.cached(&format!("forum:{module_id}"), || async {
            let v: Value = self
                .call(
                    "mod_forum_get_forum_discussions",
                    &[("forumid", id.to_string())],
                )
                .await?;
            let discussions = v["discussions"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .filter_map(|d| {
                            Some(Discussion {
                                id: d["discussion"].as_i64()?,
                                name: plain(d["name"].as_str().unwrap_or_default()),
                                author: d["userfullname"].as_str().unwrap_or_default().to_owned(),
                                created: d["created"].as_i64().unwrap_or(0),
                                modified: d["timemodified"]
                                    .as_i64()
                                    .or(d["modified"].as_i64())
                                    .unwrap_or(0),
                                replies: d["numreplies"].as_i64().unwrap_or(0),
                                unread: d["numunread"].as_i64().unwrap_or(0),
                                pinned: d["pinned"].as_bool().unwrap_or(false),
                                locked: d["locked"].as_bool().unwrap_or(false),
                                preview: preview(d["message"].as_str().unwrap_or_default()),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let can_post: Value = self
                .call(
                    "mod_forum_can_add_discussion",
                    &[("forumid", id.to_string())],
                )
                .await
                .unwrap_or_default();
            Ok(Forum {
                id,
                discussions,
                can_post: can_post["status"].as_bool().unwrap_or(false),
            })
        })
        .await
    }

    /// Alle Beiträge einer Diskussion in Moodles Reihenfolge (Baum über
    /// `parent`). Vermerkt den Aufruf in Moodle (Abschluss, Protokoll).
    pub async fn forum_thread(
        &self,
        discussion_id: i64,
        media: &MediaUrls,
    ) -> Result<Cached<Thread>> {
        let base = self.base()?;
        let cached = self
            .cached(&format!("thread:{discussion_id}"), || async {
                let v: Value = self
                    .call(
                        "mod_forum_get_discussion_posts",
                        &[
                            ("discussionid", discussion_id.to_string()),
                            ("sortby", "created".into()),
                            ("sortdirection", "ASC".into()),
                        ],
                    )
                    .await?;
                let posts: Vec<Post> = v["posts"]
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .filter_map(|p| {
                                Some(Post {
                                    id: p["id"].as_i64()?,
                                    parent: p["parentid"].as_i64(),
                                    subject: plain(p["subject"].as_str().unwrap_or_default()),
                                    author: p["author"]["fullname"]
                                        .as_str()
                                        .unwrap_or_default()
                                        .to_owned(),
                                    created: p["timecreated"].as_i64().unwrap_or(0),
                                    // Roh speichern, bereinigt wird beim Ausliefern.
                                    html: p["message"].as_str().unwrap_or_default().to_owned(),
                                    attachments: FileRef::list(&p["attachments"]),
                                    unread: p["unread"].as_bool().unwrap_or(false),
                                    can_reply: p["capabilities"]["reply"]
                                        .as_bool()
                                        .unwrap_or(false),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let name = posts
                    .iter()
                    .find(|p| p.parent.is_none())
                    .map(|p| p.subject.clone())
                    .unwrap_or_default();
                Ok(Thread {
                    id: discussion_id,
                    name,
                    posts,
                })
            })
            .await?;
        if !cached.offline {
            let _ = self
                .call::<Value>(
                    "mod_forum_view_forum_discussion",
                    &[("discussionid", discussion_id.to_string())],
                )
                .await;
        }
        let mut cached = cached;
        for post in &mut cached.data.posts {
            post.html = sanitize(&post.html, &base, media);
        }
        Ok(cached)
    }

    /// Antwort auf einen Beitrag; `text` ist reiner Text.
    pub async fn forum_reply(&self, post_id: i64, subject: &str, text: &str) -> Result<()> {
        let _: Value = self
            .call(
                "mod_forum_add_discussion_post",
                &[
                    ("postid", post_id.to_string()),
                    ("subject", subject.to_owned()),
                    ("message", text_html(text)),
                ],
            )
            .await?;
        Ok(())
    }

    /// Neues Thema; liefert die Kennung der Diskussion.
    pub async fn forum_add_discussion(
        &self,
        course_id: i64,
        module_id: i64,
        subject: &str,
        text: &str,
    ) -> Result<i64> {
        let id = self.forum_id(course_id, module_id)?;
        let v: Value = self
            .call(
                "mod_forum_add_discussion",
                &[
                    ("forumid", id.to_string()),
                    ("subject", subject.to_owned()),
                    ("message", text_html(text)),
                ],
            )
            .await?;
        v["discussionid"].as_i64().ok_or_else(|| Error::Moodle {
            code: "nodiscussionid".into(),
            message: "Moodle hat das Thema nicht angelegt".into(),
        })
    }

    /// Dateien eines Beitrags zum Öffnen.
    pub async fn forum_attachment(
        &self,
        discussion_id: i64,
        post_id: i64,
        name: &str,
        dir: &std::path::Path,
    ) -> Result<std::path::PathBuf> {
        let thread = self.forum_thread(discussion_id, &MediaUrls::none()).await?;
        let file = thread
            .data
            .posts
            .into_iter()
            .find(|p| p.id == post_id)
            .and_then(|p| p.attachments.into_iter().find(|a| a.name == name))
            .ok_or(Error::FileNotFound)?;
        self.download_file(&file, &dir.join(post_id.to_string()))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_becomes_paragraphs() {
        assert_eq!(
            text_html("Hallo <du>\nZeile 2\n\nAbsatz"),
            "<p>Hallo &lt;du&gt;<br>Zeile 2</p><p>Absatz</p>"
        );
    }

    #[test]
    fn previews_are_short() {
        let long = "a ".repeat(200);
        assert!(preview(&long).chars().count() <= 161);
        assert_eq!(preview("<p>Kurz</p>"), "Kurz");
    }
}
