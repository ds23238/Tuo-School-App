//! Aufgaben (mod_assign): Stand ansehen, Dateien und Text abgeben.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::html::{MediaUrls, sanitize};
use crate::{Error, Result, Session, paths, plain};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assignment {
    pub id: i64,
    pub module_id: i64,
    pub name: String,
    pub intro: String,
    pub allow_from: Option<i64>,
    pub due_date: Option<i64>,
    pub cutoff_date: Option<i64>,
    pub file_submission: Option<FileRules>,
    pub text_submission: bool,
    pub word_limit: Option<i64>,
    /// Abgabe muss ausdrücklich eingereicht werden (sonst gilt Speichern).
    pub drafts: bool,
    pub requires_statement: bool,
    /// `new`, `draft`, `submitted`, `reopened`; `None` ohne Versuch.
    pub status: Option<String>,
    pub submitted_at: Option<i64>,
    pub files: Vec<AssignFile>,
    /// Abgegebener Text als bereinigtes HTML.
    pub text: Option<String>,
    /// Derselbe Text als Klartext zum Bearbeiten.
    pub text_plain: String,
    pub can_edit: bool,
    pub can_submit: bool,
    pub locked: bool,
    pub grading_status: Option<String>,
    pub grade: Option<String>,
    pub graded_at: Option<i64>,
    pub feedback: Option<String>,
    pub feedback_files: Vec<AssignFile>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRules {
    pub max_files: i64,
    /// 0 = Grenze der Seite.
    pub max_bytes: i64,
    /// Erlaubte Typen wie `.pdf`, leer = alle.
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignFile {
    pub name: String,
    pub size: i64,
    pub modified: Option<i64>,
    #[serde(skip_serializing)]
    pub url: String,
}

#[derive(Debug, Deserialize)]
struct Courses {
    courses: Vec<CourseAssignments>,
}

#[derive(Debug, Deserialize)]
struct CourseAssignments {
    assignments: Vec<RemoteAssignment>,
}

#[derive(Debug, Deserialize)]
struct RemoteAssignment {
    id: i64,
    cmid: i64,
    name: String,
    #[serde(default)]
    intro: String,
    #[serde(default)]
    allowsubmissionsfromdate: i64,
    #[serde(default)]
    duedate: i64,
    #[serde(default)]
    cutoffdate: i64,
    #[serde(default)]
    submissiondrafts: i64,
    #[serde(default)]
    requiresubmissionstatement: i64,
    #[serde(default)]
    configs: Vec<Config>,
}

#[derive(Debug, Deserialize)]
struct Config {
    plugin: String,
    subtype: String,
    name: String,
    value: String,
}

impl RemoteAssignment {
    fn config(&self, plugin: &str, name: &str) -> Option<&str> {
        self.configs
            .iter()
            .find(|c| c.subtype == "assignsubmission" && c.plugin == plugin && c.name == name)
            .map(|c| c.value.as_str())
    }
}

#[derive(Debug, Default, Deserialize)]
struct Status {
    lastattempt: Option<LastAttempt>,
    feedback: Option<Feedback>,
}

#[derive(Debug, Deserialize)]
struct LastAttempt {
    submission: Option<Submission>,
    #[serde(default)]
    canedit: bool,
    #[serde(default)]
    cansubmit: bool,
    #[serde(default)]
    locked: bool,
    gradingstatus: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Submission {
    status: String,
    timemodified: Option<i64>,
    #[serde(default)]
    plugins: Vec<Plugin>,
}

#[derive(Debug, Deserialize)]
struct Plugin {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    fileareas: Vec<FileArea>,
    #[serde(default)]
    editorfields: Vec<EditorField>,
}

#[derive(Debug, Deserialize)]
struct FileArea {
    #[serde(default)]
    files: Vec<RemoteFile>,
}

#[derive(Debug, Deserialize)]
struct RemoteFile {
    filename: String,
    #[serde(default)]
    filesize: i64,
    fileurl: String,
    timemodified: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EditorField {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct Feedback {
    gradefordisplay: Option<String>,
    gradeddate: Option<i64>,
    #[serde(default)]
    plugins: Vec<Plugin>,
}

fn files_of(plugins: &[Plugin], kind: &str) -> Vec<AssignFile> {
    plugins
        .iter()
        .filter(|p| p.kind == kind)
        .flat_map(|p| &p.fileareas)
        .flat_map(|a| &a.files)
        .map(|f| AssignFile {
            name: f.filename.clone(),
            size: f.filesize,
            modified: f.timemodified,
            url: f.fileurl.clone(),
        })
        .collect()
}

fn text_of(plugins: &[Plugin], kind: &str) -> Option<String> {
    plugins
        .iter()
        .filter(|p| p.kind == kind)
        .flat_map(|p| &p.editorfields)
        .map(|f| f.text.clone())
        .find(|t| !t.trim().is_empty())
}

fn date(value: i64) -> Option<i64> {
    (value > 0).then_some(value)
}

/// Klartext für das Bearbeiten: Absätze und Zeilenumbrüche bleiben erhalten.
pub fn html_to_text(html: &str) -> String {
    let marked = html
        .replace("</p>", "\n\n")
        .replace("<br>", "\n")
        .replace("<br />", "\n")
        .replace("<br/>", "\n");
    let text = ammonia::Builder::empty().clean(&marked).to_string();
    plain(&text)
}

/// Klartext als HTML für Moodle: Leerzeilen trennen Absätze.
pub fn text_to_html(text: &str) -> String {
    text.split("\n\n")
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let escaped = html_escape::encode_text(p).replace('\n', "<br>");
            format!("<p>{escaped}</p>")
        })
        .collect()
}

/// Was beim Speichern abgegeben werden soll.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    /// Namen bereits abgegebener Dateien, die bleiben sollen.
    pub keep: Vec<String>,
    /// Neue Dateien von der Festplatte.
    pub add: Vec<PathBuf>,
    /// Klartext, falls Textabgabe aktiv ist.
    pub text: Option<String>,
}

impl Session<'_> {
    pub async fn assignment(
        &self,
        course_id: i64,
        module_id: i64,
        media: &MediaUrls,
    ) -> Result<Assignment> {
        let base = self.base()?;
        let courses: Courses = self
            .call(
                "mod_assign_get_assignments",
                &[("courseids[0]", course_id.to_string())],
            )
            .await?;
        let a = courses
            .courses
            .into_iter()
            .flat_map(|c| c.assignments)
            .find(|a| a.cmid == module_id)
            .ok_or(Error::FileNotFound)?;
        let status: Status = self
            .call(
                "mod_assign_get_submission_status",
                &[("assignid", a.id.to_string())],
            )
            .await?;
        let attempt = status.lastattempt;
        let submission = attempt.as_ref().and_then(|l| l.submission.as_ref());
        let plugins = submission.map_or(&[][..], |s| &s.plugins[..]);
        let raw_text = text_of(plugins, "onlinetext");
        let feedback = status.feedback;
        let feedback_plugins = feedback.as_ref().map_or(&[][..], |f| &f.plugins[..]);
        let html = |h: &str| sanitize(h, &base, media);

        let file_enabled = a.config("file", "enabled") == Some("1");
        let number = |plugin, name| a.config(plugin, name).and_then(|v| v.parse::<i64>().ok());
        Ok(Assignment {
            id: a.id,
            module_id: a.cmid,
            name: plain(&a.name),
            intro: html(&a.intro),
            allow_from: date(a.allowsubmissionsfromdate),
            due_date: date(a.duedate),
            cutoff_date: date(a.cutoffdate),
            file_submission: file_enabled.then(|| FileRules {
                max_files: number("file", "maxfilesubmissions").unwrap_or(1),
                max_bytes: number("file", "maxsubmissionsizebytes").unwrap_or(0),
                types: a
                    .config("file", "filetypeslist")
                    .unwrap_or_default()
                    .split([',', ' ', ';'])
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned)
                    .collect(),
            }),
            text_submission: a.config("onlinetext", "enabled") == Some("1"),
            word_limit: (a.config("onlinetext", "wordlimitenabled") == Some("1"))
                .then(|| number("onlinetext", "wordlimit"))
                .flatten(),
            drafts: a.submissiondrafts != 0,
            requires_statement: a.requiresubmissionstatement != 0,
            status: submission.map(|s| s.status.clone()),
            submitted_at: submission.and_then(|s| s.timemodified),
            files: files_of(plugins, "file"),
            text_plain: raw_text.as_deref().map(html_to_text).unwrap_or_default(),
            text: raw_text.as_deref().map(html),
            can_edit: attempt.as_ref().is_some_and(|l| l.canedit),
            can_submit: attempt.as_ref().is_some_and(|l| l.cansubmit),
            locked: attempt.as_ref().is_some_and(|l| l.locked),
            grading_status: attempt.and_then(|l| l.gradingstatus),
            grade: feedback
                .as_ref()
                .and_then(|f| f.gradefordisplay.as_deref())
                .map(html),
            graded_at: feedback.as_ref().and_then(|f| f.gradeddate).and_then(date),
            feedback: text_of(feedback_plugins, "comments").as_deref().map(html),
            feedback_files: files_of(feedback_plugins, "file"),
        })
    }

    /// Speichert die Abgabe. Moodle ersetzt alle Dateien, daher werden die
    /// behaltenen Dateien erneut hochgeladen, wie es auch die Moodle-App tut.
    pub async fn save_submission(
        &self,
        course_id: i64,
        module_id: i64,
        draft: Draft,
        media: &MediaUrls,
    ) -> Result<()> {
        let current = self.assignment(course_id, module_id, media).await?;
        let (client, token) = (crate::Client::new(self.base()?), self.token()?);
        let mut item_id = 0;
        for file in current
            .files
            .iter()
            .filter(|f| draft.keep.contains(&f.name))
        {
            let url = Url::parse(&file.url).map_err(|_| Error::InvalidUrl(file.url.clone()))?;
            let (bytes, _) = self.fetch_media(&url).await?;
            item_id = client.upload(&token, item_id, &file.name, bytes).await?;
        }
        for path in &draft.add {
            let bytes = tokio::fs::read(path).await.map_err(Error::Io)?;
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Datei".into());
            item_id = client.upload(&token, item_id, &name, bytes).await?;
        }
        let mut params = vec![("assignmentid", current.id.to_string())];
        if current.file_submission.is_some() {
            params.push(("plugindata[files_filemanager]", item_id.to_string()));
        }
        if let (true, Some(text)) = (current.text_submission, &draft.text) {
            params.push(("plugindata[onlinetext_editor][text]", text_to_html(text)));
            params.push(("plugindata[onlinetext_editor][format]", "1".into()));
            params.push(("plugindata[onlinetext_editor][itemid]", item_id.to_string()));
        }
        let warnings: Vec<Warning> = self.call("mod_assign_save_submission", &params).await?;
        warnings_to_result(warnings)
    }

    /// Reicht eine gespeicherte Abgabe zur Bewertung ein.
    pub async fn submit_for_grading(&self, assign_id: i64, accept_statement: bool) -> Result<()> {
        let warnings: Vec<Warning> = self
            .call(
                "mod_assign_submit_for_grading",
                &[
                    ("assignmentid", assign_id.to_string()),
                    (
                        "acceptsubmissionstatement",
                        u8::from(accept_statement).to_string(),
                    ),
                ],
            )
            .await?;
        warnings_to_result(warnings)
    }

    /// Lädt eine abgegebene Datei oder eine Feedback-Datei zum Ansehen.
    pub async fn assignment_file(
        &self,
        course_id: i64,
        module_id: i64,
        feedback: bool,
        name: &str,
        dir: &Path,
        media: &MediaUrls,
    ) -> Result<PathBuf> {
        let a = self.assignment(course_id, module_id, media).await?;
        let files = if feedback { a.feedback_files } else { a.files };
        let file = files
            .into_iter()
            .find(|f| f.name == name)
            .ok_or(Error::FileNotFound)?;
        let url = Url::parse(&file.url).map_err(|_| Error::InvalidUrl(file.url.clone()))?;
        let path = dir
            .join(a.id.to_string())
            .join(if feedback { "feedback" } else { "abgabe" })
            .join(paths::component(&file.name));
        let client = crate::Client::new(self.base()?);
        client
            .download_to(client.authorize(&url, &self.token()?), &path)
            .await?;
        Ok(path)
    }
}

#[derive(Debug, Deserialize)]
struct Warning {
    #[serde(default)]
    message: String,
    #[serde(default)]
    warningcode: String,
}

fn warnings_to_result(warnings: Vec<Warning>) -> Result<()> {
    match warnings.into_iter().next() {
        None => Ok(()),
        Some(w) => Err(Error::Moodle {
            code: w.warningcode,
            message: w.message,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_roundtrip() {
        let html = text_to_html("Erster Absatz\nmit Umbruch\n\nZweiter <b>Absatz</b>");
        assert_eq!(
            html,
            "<p>Erster Absatz<br>mit Umbruch</p><p>Zweiter &lt;b&gt;Absatz&lt;/b&gt;</p>"
        );
        assert_eq!(
            html_to_text(&html),
            "Erster Absatz\nmit Umbruch\n\nZweiter <b>Absatz</b>"
        );
    }
}
