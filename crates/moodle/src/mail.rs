//! Moodle-Mail (Plugin `local_mail`, ab Version 2).
//!
//! Die Funktionen des Plugins gehören nicht zum Dienst der Moodle-App, das
//! Token darf sie meist nicht aufrufen. Dann nutzt die App wie die
//! Weboberfläche des Plugins eine Browser-Sitzung (`lib/ajax/service.php`),
//! angemeldet per Auto-Login-Schlüssel oder gespeichertem Passwort.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tsa_core::{SecretKind, Service};
use url::Url;

use crate::assign::{html_to_text, text_to_html};
use crate::client::{browser, text_only};
use crate::html::{MediaUrls, sanitize};
use crate::{Error, Result, Session, paths, plain};

/// So lange nach einer gescheiterten Anmeldung nicht neu versuchen: Moodle
/// gibt Auto-Login-Schlüssel nur alle paar Minuten aus.
const LOGIN_BACKOFF: Duration = Duration::from_secs(6 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Das Token darf die Funktionen aufrufen.
    Token,
    /// Über eine Browser-Sitzung.
    Web,
    Missing,
}

#[derive(Default)]
struct Inner {
    mode: Option<Mode>,
    web: Option<WebSession>,
    failed_login: Option<Instant>,
}

/// Zustand über einzelne Aufrufe hinweg (Art des Zugriffs, Sitzung). Lebt
/// nur im Speicher. Die Browser-Sitzung nutzen auch andere Plugins ohne
/// App-Schnittstelle (Board), denn Auto-Login-Schlüssel sind knapp.
#[derive(Default)]
pub struct MailState(Mutex<Inner>);

impl MailState {
    pub async fn reset(&self) {
        *self.0.lock().await = Inner::default();
    }
}

struct WebSession {
    http: reqwest::Client,
    base: Url,
    sesskey: String,
}

impl WebSession {
    async fn call<T: DeserializeOwned>(&self, function: &str, args: &Value) -> Result<T> {
        let mut url = self
            .base
            .join("lib/ajax/service.php")
            .expect("relativer Pfad");
        url.query_pairs_mut()
            .append_pair("sesskey", &self.sesskey)
            .append_pair("info", function);
        let body = json!([{ "index": 0, "methodname": function, "args": args }]);
        let value: Value = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(first) = value.get(0) {
            if first["error"] == false {
                return Ok(serde_json::from_value(first["data"].clone())?);
            }
            let e = &first["exception"];
            return Err(moodle_error(&e["errorcode"], &e["message"]));
        }
        Err(moodle_error(&value["errorcode"], &value["error"]))
    }
}

fn moodle_error(code: &Value, message: &Value) -> Error {
    Error::Moodle {
        code: code.as_str().unwrap_or_default().to_owned(),
        message: text_only(message.as_str().unwrap_or_default()),
    }
}

fn session_expired(e: &Error) -> bool {
    matches!(e, Error::Moodle { code, .. }
        if matches!(code.as_str(), "servicerequireslogin" | "invalidsesskey" | "requireloginerror"))
}

/// `sesskey` aus der Seitenkonfiguration (`M.cfg`).
fn sesskey(html: &str) -> Option<String> {
    let start = html.find(r#""sesskey":""#)? + 11;
    let key: String = html[start..]
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    (!key.is_empty()).then_some(key)
}

fn login_token(html: &str) -> Option<String> {
    let at = html.find(r#"name="logintoken""#)?;
    let rest = &html[at..];
    let start = rest.find(r#"value=""#)? + 7;
    let end = rest[start..].find('"')?;
    Some(rest[start..start + end].to_owned())
}

/// Argumente als flache Formularfelder für die REST-Schnittstelle, z. B.
/// `query[roles][0]=to`.
fn flatten(prefix: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}[{k}]")
                };
                flatten(&key, v, out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                flatten(&format!("{prefix}[{i}]"), v, out);
            }
        }
        Value::Bool(b) => out.push((prefix.to_owned(), u8::from(*b).to_string())),
        Value::Number(n) => out.push((prefix.to_owned(), n.to_string())),
        Value::String(s) => out.push((prefix.to_owned(), s.clone())),
        Value::Null => {}
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Folder {
    Inbox,
    Starred,
    Sent,
    Drafts,
    Trash,
}

impl Folder {
    fn query(self, course_id: Option<i64>, search: &str) -> Value {
        let mut query = json!({
            "courseid": course_id.unwrap_or(0),
            "deleted": matches!(self, Self::Trash),
            "content": search,
        });
        match self {
            Self::Inbox => query["roles"] = json!(["to", "cc", "bcc"]),
            Self::Sent => {
                query["roles"] = json!(["from"]);
                query["draft"] = json!(false);
            }
            Self::Drafts => query["draft"] = json!(true),
            Self::Starred => query["starred"] = json!(true),
            Self::Trash => {}
        }
        query
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailStatus {
    pub courses: Vec<MailCourse>,
    pub unread: i64,
    pub drafts: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailCourse {
    pub id: i64,
    #[serde(alias = "shortname")]
    pub short_name: String,
    #[serde(alias = "fullname")]
    pub full_name: String,
    #[serde(default)]
    pub unread: i64,
    #[serde(default)]
    pub drafts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipient {
    /// `to`, `cc` oder `bcc`.
    pub kind: String,
    #[serde(flatten)]
    pub person: Person,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSummary {
    pub id: i64,
    pub subject: String,
    pub time: i64,
    pub unread: bool,
    pub starred: bool,
    pub draft: bool,
    pub attachments: i64,
    pub course: String,
    pub sender: Person,
    pub recipients: Vec<Recipient>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailPage {
    pub messages: Vec<MailSummary>,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub name: String,
    pub size: i64,
    pub mime: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    pub id: i64,
    pub subject: String,
    pub html: String,
    pub time: i64,
    pub sender: Person,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    pub id: i64,
    pub subject: String,
    pub html: String,
    pub time: i64,
    pub unread: bool,
    pub starred: bool,
    pub deleted: bool,
    pub draft: bool,
    pub course_id: i64,
    pub course: String,
    pub sender: Person,
    pub recipients: Vec<Recipient>,
    pub attachments: Vec<Attachment>,
    pub references: Vec<Reference>,
}

/// Ein Entwurf zum Bearbeiten.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailDraft {
    pub id: i64,
    pub course_id: i64,
    pub subject: String,
    pub text: String,
    /// Weitergeleitete Nachricht unverändert von Moodle; wird beim
    /// Speichern wieder angehängt.
    pub quote: Option<String>,
    pub quote_html: Option<String>,
    pub to: Vec<Person>,
    pub cc: Vec<Person>,
    pub bcc: Vec<Person>,
    pub attachments: Vec<Attachment>,
    pub references: Vec<Reference>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftInput {
    pub course_id: i64,
    pub to: Vec<i64>,
    pub cc: Vec<i64>,
    pub bcc: Vec<i64>,
    pub subject: String,
    pub text: String,
    pub quote: Option<String>,
    /// Neue Anhänge von der Festplatte.
    pub add: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RemoteUser {
    id: i64,
    fullname: String,
    #[serde(default)]
    pictureurl: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RemoteRecipient {
    #[serde(rename = "type")]
    kind: String,
    #[serde(flatten)]
    user: RemoteUser,
}

#[derive(Debug, Deserialize)]
struct RemoteCourse {
    id: i64,
    shortname: String,
    #[serde(default)]
    fullname: String,
}

#[derive(Debug, Deserialize)]
struct RemoteSummary {
    id: i64,
    subject: String,
    #[serde(default)]
    numattachments: i64,
    draft: bool,
    time: i64,
    unread: bool,
    starred: bool,
    #[serde(default)]
    course: Option<RemoteCourse>,
    sender: RemoteUser,
    #[serde(default)]
    recipients: Vec<RemoteRecipient>,
}

#[derive(Debug, Deserialize)]
struct RemoteAttachment {
    filename: String,
    #[serde(default)]
    filesize: i64,
    #[serde(default)]
    mimetype: String,
    fileurl: String,
}

#[derive(Debug, Deserialize)]
struct RemoteReference {
    id: i64,
    subject: String,
    content: String,
    time: i64,
    sender: RemoteUser,
    #[serde(default)]
    attachments: Vec<RemoteAttachment>,
}

#[derive(Debug, Deserialize)]
struct RemoteMessage {
    id: i64,
    subject: String,
    content: String,
    draft: bool,
    time: i64,
    unread: bool,
    starred: bool,
    deleted: bool,
    course: RemoteCourse,
    sender: RemoteUser,
    #[serde(default)]
    recipients: Vec<RemoteRecipient>,
    #[serde(default)]
    attachments: Vec<RemoteAttachment>,
    #[serde(default)]
    references: Vec<RemoteReference>,
}

fn person(u: &RemoteUser, media: &MediaUrls) -> Person {
    Person {
        id: u.id,
        name: plain(&u.fullname),
        image: crate::messages::image(u.pictureurl.as_deref(), media),
    }
}

fn recipients(list: &[RemoteRecipient], media: &MediaUrls) -> Vec<Recipient> {
    list.iter()
        .map(|r| Recipient {
            kind: r.kind.clone(),
            person: person(&r.user, media),
        })
        .collect()
}

fn attachments(list: &[RemoteAttachment]) -> Vec<Attachment> {
    list.iter()
        .map(|a| Attachment {
            name: a.filename.clone(),
            size: a.filesize,
            mime: a.mimetype.clone(),
            url: a.fileurl.clone(),
        })
        .collect()
}

/// Ein frisch weitergeleiteter Entwurf beginnt mit einem leeren Absatz,
/// danach folgt die Originalnachricht.
fn split_forward(content: &str) -> Option<&str> {
    content
        .strip_prefix("<p><br></p>")
        .filter(|rest| !rest.trim().is_empty())
}

impl Session<'_> {
    async fn mail_call<T: DeserializeOwned>(
        &self,
        state: &MailState,
        function: &str,
        args: Value,
    ) -> Result<T> {
        let mut inner = state.0.lock().await;
        let mode = match inner.mode {
            Some(mode) => mode,
            None => {
                let mode = self.detect_mail().await?;
                inner.mode = Some(mode);
                mode
            }
        };
        match mode {
            Mode::Missing => Err(Error::MailUnavailable),
            Mode::Token => {
                let mut pairs = Vec::new();
                flatten("", &args, &mut pairs);
                let params: Vec<(&str, String)> =
                    pairs.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
                self.call(function, &params).await
            }
            Mode::Web => {
                if let Some(web) = &inner.web {
                    match web.call(function, &args).await {
                        Err(e) if session_expired(&e) => inner.web = None,
                        result => return self.check_missing(&mut inner, result),
                    }
                }
                if inner
                    .failed_login
                    .is_some_and(|t| t.elapsed() < LOGIN_BACKOFF)
                {
                    return Err(Error::MailLogin);
                }
                let Some(web) = self.web_login("local/mail/view.php").await? else {
                    inner.failed_login = Some(Instant::now());
                    return Err(Error::MailLogin);
                };
                inner.failed_login = None;
                let result = web.call(function, &args).await;
                inner.web = Some(web);
                self.check_missing(&mut inner, result)
            }
        }
    }

    /// Ältere Versionen des Plugins haben andere Funktionen.
    fn check_missing<T>(&self, inner: &mut Inner, result: Result<T>) -> Result<T> {
        match result {
            Err(Error::Moodle { code, .. }) if code == "invalidrecord" => {
                inner.mode = Some(Mode::Missing);
                Err(Error::MailUnavailable)
            }
            result => result,
        }
    }

    async fn detect_mail(&self) -> Result<Mode> {
        let info: crate::SiteInfo = self.call("core_webservice_get_site_info", &[]).await?;
        if info
            .functions
            .iter()
            .any(|f| f.name == "local_mail_search_messages")
        {
            return Ok(Mode::Token);
        }
        let page = self
            .base()?
            .join("local/mail/view.php")
            .expect("relativer Pfad");
        let status = reqwest::get(page).await?.status();
        Ok(if status == reqwest::StatusCode::NOT_FOUND {
            Mode::Missing
        } else {
            Mode::Web
        })
    }

    /// Ruft eine Funktion über die Browser-Sitzung auf (`lib/ajax/service.php`);
    /// meldet sich bei Bedarf über `page` an, eine Seite des Plugins.
    pub(crate) async fn web_call<T: DeserializeOwned>(
        &self,
        state: &MailState,
        page: &str,
        function: &str,
        args: Value,
    ) -> Result<T> {
        let mut inner = state.0.lock().await;
        if let Some(web) = &inner.web {
            match web.call(function, &args).await {
                Err(e) if session_expired(&e) => inner.web = None,
                result => return result,
            }
        }
        if inner
            .failed_login
            .is_some_and(|t| t.elapsed() < LOGIN_BACKOFF)
        {
            return Err(Error::MailLogin);
        }
        let Some(web) = self.web_login(page).await? else {
            inner.failed_login = Some(Instant::now());
            return Err(Error::MailLogin);
        };
        inner.failed_login = None;
        let result = web.call(function, &args).await;
        inner.web = Some(web);
        result
    }

    async fn web_login(&self, page: &str) -> Result<Option<WebSession>> {
        let base = self.base()?;
        let http = browser();
        let target = base
            .join(page)
            .map_err(|_| Error::InvalidUrl(page.to_owned()))?;
        let open = async |url: Url| -> Result<Option<String>> {
            let response = http.get(url).send().await?;
            if response.url().path() != target.path() {
                return Ok(None);
            }
            Ok(sesskey(&response.text().await?))
        };
        let mut key = None;
        if let Some(link) = self.autologin_link(target.as_str()).await? {
            key = open(link).await?;
        }
        let password = match key {
            None => self.secrets().get(Service::Moodle, SecretKind::Password)?,
            Some(_) => None,
        };
        if let Some(password) = password {
            let login = base.join("login/index.php").expect("relativer Pfad");
            let page = http.get(login.clone()).send().await?.text().await?;
            let account =
                tsa_core::Account::get(self.db(), Service::Moodle)?.ok_or(Error::NotSignedIn)?;
            let mut form = vec![
                ("username", account.username),
                ("password", password),
                ("anchor", String::new()),
            ];
            if let Some(token) = login_token(&page) {
                form.push(("logintoken", token));
            }
            http.post(login).form(&form).send().await?;
            key = open(target.clone()).await?;
        }
        Ok(key.map(|sesskey| WebSession {
            http: http.clone(),
            base,
            sesskey,
        }))
    }

    /// `None`, wenn die Seite kein Moodle-Mail hat.
    pub async fn mail_status(&self, state: &MailState) -> Result<Option<MailStatus>> {
        let courses: Vec<MailCourse> = match self
            .mail_call(state, "local_mail_get_courses", json!({}))
            .await
        {
            Err(Error::MailUnavailable) => return Ok(None),
            result => result?,
        };
        let courses: Vec<MailCourse> = courses
            .into_iter()
            .map(|c| MailCourse {
                short_name: plain(&c.short_name),
                full_name: plain(&c.full_name),
                ..c
            })
            .collect();
        Ok(Some(MailStatus {
            unread: courses.iter().map(|c| c.unread).sum(),
            drafts: courses.iter().map(|c| c.drafts).sum(),
            courses,
        }))
    }

    pub async fn mail_list(
        &self,
        state: &MailState,
        folder: Folder,
        course_id: Option<i64>,
        search: &str,
        offset: usize,
        media: &MediaUrls,
    ) -> Result<MailPage> {
        const PAGE: usize = 30;
        let query = folder.query(course_id, search.trim());
        let total: i64 = self
            .mail_call(
                state,
                "local_mail_count_messages",
                json!({ "query": query }),
            )
            .await?;
        let remote: Vec<RemoteSummary> = self
            .mail_call(
                state,
                "local_mail_search_messages",
                json!({ "query": query, "offset": offset, "limit": PAGE }),
            )
            .await?;
        let messages = remote
            .into_iter()
            .map(|m| MailSummary {
                id: m.id,
                subject: plain(&m.subject),
                time: m.time,
                unread: m.unread,
                starred: m.starred,
                draft: m.draft,
                attachments: m.numattachments,
                course: m.course.map(|c| plain(&c.shortname)).unwrap_or_default(),
                sender: person(&m.sender, media),
                recipients: recipients(&m.recipients, media),
            })
            .collect();
        Ok(MailPage { messages, total })
    }

    async fn remote_mail(&self, state: &MailState, id: i64) -> Result<RemoteMessage> {
        self.mail_call(state, "local_mail_get_message", json!({ "messageid": id }))
            .await
    }

    fn references(&self, list: &[RemoteReference], media: &MediaUrls) -> Result<Vec<Reference>> {
        let base = self.base()?;
        Ok(list
            .iter()
            .map(|r| Reference {
                id: r.id,
                subject: plain(&r.subject),
                html: sanitize(&r.content, &base, media),
                time: r.time,
                sender: person(&r.sender, media),
                attachments: attachments(&r.attachments),
            })
            .collect())
    }

    /// Öffnet eine Nachricht und markiert sie als gelesen.
    pub async fn mail_message(
        &self,
        state: &MailState,
        id: i64,
        media: &MediaUrls,
    ) -> Result<MailMessage> {
        let m = self.remote_mail(state, id).await?;
        if m.unread {
            let _: Value = self
                .mail_call(state, "local_mail_view_message", json!({ "messageid": id }))
                .await?;
        }
        let base = self.base()?;
        Ok(MailMessage {
            id: m.id,
            subject: plain(&m.subject),
            html: sanitize(&m.content, &base, media),
            time: m.time,
            unread: false,
            starred: m.starred,
            deleted: m.deleted,
            draft: m.draft,
            course_id: m.course.id,
            course: plain(&m.course.fullname),
            sender: person(&m.sender, media),
            recipients: recipients(&m.recipients, media),
            attachments: attachments(&m.attachments),
            references: self.references(&m.references, media)?,
        })
    }

    pub async fn mail_set_flag(
        &self,
        state: &MailState,
        id: i64,
        flag: &str,
        value: bool,
    ) -> Result<()> {
        let (function, key) = match flag {
            "starred" => ("local_mail_set_starred", "starred"),
            _ => ("local_mail_set_unread", "unread"),
        };
        let _: Value = self
            .mail_call(state, function, json!({ "messageid": id, key: value }))
            .await?;
        Ok(())
    }

    /// 0 = wiederherstellen, 1 = in den Papierkorb, 2 = endgültig löschen.
    pub async fn mail_set_deleted(&self, state: &MailState, id: i64, deleted: u8) -> Result<()> {
        let _: Value = self
            .mail_call(
                state,
                "local_mail_set_deleted",
                json!({ "messageid": id, "deleted": deleted.min(2) }),
            )
            .await?;
        Ok(())
    }

    pub async fn mail_search_users(
        &self,
        state: &MailState,
        course_id: i64,
        text: &str,
        media: &MediaUrls,
    ) -> Result<Vec<Person>> {
        let users: Vec<RemoteUser> = self
            .mail_call(
                state,
                "local_mail_search_users",
                json!({ "query": { "courseid": course_id, "fullname": text.trim() }, "limit": 30 }),
            )
            .await?;
        Ok(users.iter().map(|u| person(u, media)).collect())
    }

    /// Legt einen Entwurf an: neu (`course_id`), als Antwort oder als
    /// Weiterleitung von `id`.
    pub async fn mail_create(
        &self,
        state: &MailState,
        kind: &str,
        id: i64,
        media: &MediaUrls,
    ) -> Result<MailDraft> {
        let (function, args) = match kind {
            "reply" => (
                "local_mail_reply_message",
                json!({ "messageid": id, "all": false }),
            ),
            "replyAll" => (
                "local_mail_reply_message",
                json!({ "messageid": id, "all": true }),
            ),
            "forward" => ("local_mail_forward_message", json!({ "messageid": id })),
            _ => ("local_mail_create_message", json!({ "courseid": id })),
        };
        let draft: i64 = self.mail_call(state, function, args).await?;
        self.mail_draft(state, draft, kind == "forward", media)
            .await
    }

    pub async fn mail_draft(
        &self,
        state: &MailState,
        id: i64,
        fresh_forward: bool,
        media: &MediaUrls,
    ) -> Result<MailDraft> {
        let m = self.remote_mail(state, id).await?;
        let base = self.base()?;
        let quote = fresh_forward
            .then(|| split_forward(&m.content))
            .flatten()
            .map(str::to_owned);
        let text = if quote.is_some() {
            String::new()
        } else {
            html_to_text(&m.content)
        };
        let of = |kind: &str| {
            m.recipients
                .iter()
                .filter(|r| r.kind == kind)
                .map(|r| person(&r.user, media))
                .collect::<Vec<_>>()
        };
        Ok(MailDraft {
            id: m.id,
            course_id: m.course.id,
            subject: m.subject.clone(),
            text,
            quote_html: quote.as_deref().map(|q| sanitize(q, &base, media)),
            quote,
            to: of("to"),
            cc: of("cc"),
            bcc: of("bcc"),
            attachments: attachments(&m.attachments),
            references: self.references(&m.references, media)?,
        })
    }

    /// Speichert einen Entwurf und schickt ihn mit `send` ab.
    pub async fn mail_save(
        &self,
        state: &MailState,
        id: i64,
        input: DraftInput,
        send: bool,
    ) -> Result<()> {
        #[derive(Deserialize)]
        struct Form {
            draftitemid: i64,
        }
        let form: Form = self
            .mail_call(
                state,
                "local_mail_get_message_form",
                json!({ "messageid": id }),
            )
            .await?;
        let (client, token) = (crate::Client::new(self.base()?), self.token()?);
        for path in &input.add {
            let bytes = tokio::fs::read(path).await.map_err(Error::Io)?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Datei".into());
            client
                .upload(&token, form.draftitemid, &name, bytes)
                .await?;
        }
        let mut content = text_to_html(&input.text);
        if let Some(quote) = &input.quote {
            content.push_str("<p><br></p>");
            content.push_str(quote);
        }
        let data = json!({
            "courseid": input.course_id,
            "to": input.to,
            "cc": input.cc,
            "bcc": input.bcc,
            "subject": input.subject,
            "content": content,
            "format": 1,
            "draftitemid": form.draftitemid,
        });
        let _: Value = self
            .mail_call(
                state,
                "local_mail_update_message",
                json!({ "messageid": id, "data": data }),
            )
            .await?;
        if send {
            let _: Value = self
                .mail_call(state, "local_mail_send_message", json!({ "messageid": id }))
                .await?;
        }
        Ok(())
    }

    /// Lädt einen Anhang nach `dir` und gibt den Pfad zurück.
    pub async fn mail_attachment(
        &self,
        state: &MailState,
        message_id: i64,
        name: &str,
        dir: &Path,
    ) -> Result<PathBuf> {
        let m = self.remote_mail(state, message_id).await?;
        let file = m
            .attachments
            .iter()
            .chain(m.references.iter().flat_map(|r| &r.attachments))
            .find(|a| a.filename == name)
            .ok_or(Error::FileNotFound)?;
        let url = Url::parse(&file.fileurl).map_err(|_| Error::InvalidUrl(file.fileurl.clone()))?;
        let path = dir
            .join(message_id.to_string())
            .join(paths::component(&file.filename));
        let client = crate::Client::new(self.base()?);
        let by_token = client
            .download_to(client.authorize(&url, &self.token()?), &path)
            .await;
        if by_token.is_ok() {
            return Ok(path);
        }
        // Manche Seiten liefern Mail-Anhänge nur an eine Browser-Sitzung.
        let inner = state.0.lock().await;
        match &inner.web {
            Some(web) => {
                let bytes = web
                    .http
                    .get(url)
                    .send()
                    .await?
                    .error_for_status()?
                    .bytes()
                    .await?;
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(Error::Io)?;
                }
                tokio::fs::write(&path, bytes).await.map_err(Error::Io)?;
                Ok(path)
            }
            None => by_token.map(|()| path),
        }
    }

    /// Neueste ungelesene Mails im Posteingang (für Benachrichtigungen).
    pub(crate) async fn unread_mail(
        &self,
        state: &MailState,
    ) -> Result<Vec<(i64, String, String)>> {
        let mut query = Folder::Inbox.query(None, "");
        query["unread"] = json!(true);
        let remote: Vec<RemoteSummary> = self
            .mail_call(
                state,
                "local_mail_search_messages",
                json!({ "query": query, "limit": 5 }),
            )
            .await?;
        Ok(remote
            .into_iter()
            .map(|m| (m.id, plain(&m.sender.fullname), plain(&m.subject)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattens_nested_arguments() {
        let mut out = Vec::new();
        flatten(
            "",
            &json!({ "query": { "roles": ["to", "cc"], "deleted": false, "draft": null }, "limit": 5 }),
            &mut out,
        );
        out.sort();
        assert_eq!(
            out,
            [
                ("limit".to_owned(), "5".to_owned()),
                ("query[deleted]".to_owned(), "0".to_owned()),
                ("query[roles][0]".to_owned(), "to".to_owned()),
                ("query[roles][1]".to_owned(), "cc".to_owned()),
            ]
        );
    }

    #[test]
    fn finds_session_key_and_login_token() {
        let html = r#"<script>M.cfg = {"wwwroot":"x","sesskey":"Ab12Cd","sessiontimeout":"7200"};</script>
            <input type="hidden" name="logintoken" value="tok3n">"#;
        assert_eq!(sesskey(html).as_deref(), Some("Ab12Cd"));
        assert_eq!(login_token(html).as_deref(), Some("tok3n"));
        assert_eq!(sesskey("<html></html>"), None);
    }

    #[test]
    fn splits_fresh_forward() {
        assert_eq!(
            split_forward("<p><br></p><p>Original</p>"),
            Some("<p>Original</p>")
        );
        assert_eq!(split_forward("<p>Eigener Text</p>"), None);
    }
}
