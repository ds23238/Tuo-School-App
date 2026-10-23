//! Befehle für WebUntis und die gemeinsame Heute-Ansicht.

use chrono::{Days, Local, NaiveDate};
use tauri::{AppHandle, Manager, State};
use tsa_moodle::deadlines::Deadline;
use tsa_moodle::sync::RecentFile;
use tsa_untis::homework::{Exam, Homework, Listed, NewExam};
use tsa_untis::messages::{Compose, DayNews, Message, MessageDetail, Outgoing};
use tsa_untis::timetable::{Lesson, Week};
use tsa_untis::{LoginResult, School, Session};

use crate::error::{CommandError, Result};
use crate::{AppState, inbox, launch, mapping};

fn untis<'a>(state: &'a State<AppState>) -> Session<'a> {
    Session::new(&state.db, state.secrets.as_ref(), &state.untis)
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn date(value: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| CommandError::new("internal", format!("Ungültiges Datum: {value}")))
}

#[tauri::command]
pub async fn untis_search(query: String) -> Result<Vec<School>> {
    Ok(tsa_untis::search_schools(&query).await?)
}

#[tauri::command]
pub async fn untis_login(
    app: AppHandle,
    state: State<'_, AppState>,
    school: School,
    username: String,
    password: String,
    remember: bool,
) -> Result<LoginResult> {
    let result = untis(&state)
        .login(school, &username, &password, remember)
        .await?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move { inbox::check_untis(&app).await });
    Ok(result)
}

#[tauri::command]
pub async fn untis_week(state: State<'_, AppState>, date: String) -> Result<Week> {
    Ok(untis(&state).week(self::date(&date)?).await?)
}

/// Heute und morgen (bzw. bis zum nächsten Schultag nach dem Wochenende).
#[tauri::command]
pub async fn untis_upcoming(state: State<'_, AppState>) -> Result<Vec<Lesson>> {
    let from = today();
    Ok(untis(&state).range(from, from + Days::new(3)).await?)
}

#[tauri::command]
pub async fn untis_homework(state: State<'_, AppState>) -> Result<Listed<Homework>> {
    Ok(untis(&state).homework(today()).await?)
}

#[tauri::command]
pub fn untis_set_homework_done(state: State<'_, AppState>, id: i64, done: bool) -> Result<()> {
    Ok(untis(&state).set_homework_done(id, done)?)
}

#[tauri::command]
pub async fn untis_exams(state: State<'_, AppState>) -> Result<Listed<Exam>> {
    Ok(untis(&state).exams(today()).await?)
}

#[tauri::command]
pub fn untis_add_exam(state: State<'_, AppState>, exam: NewExam) -> Result<()> {
    Ok(untis(&state).add_exam(exam)?)
}

#[tauri::command]
pub fn untis_delete_exam(state: State<'_, AppState>, id: i64) -> Result<()> {
    Ok(untis(&state).delete_exam(id)?)
}

#[tauri::command]
pub async fn untis_messages(state: State<'_, AppState>) -> Result<Vec<Message>> {
    Ok(untis(&state).messages().await?)
}

#[tauri::command]
pub async fn untis_sent_messages(state: State<'_, AppState>) -> Result<Vec<Message>> {
    Ok(untis(&state).sent_messages().await?)
}

#[tauri::command]
pub async fn untis_message(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<MessageDetail> {
    let detail = untis(&state).message(id).await?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move { inbox::check_untis(&app).await });
    Ok(detail)
}

/// Lädt einen Anhang in den Cache und öffnet ihn.
#[tauri::command]
pub async fn untis_open_attachment(
    app: AppHandle,
    state: State<'_, AppState>,
    message_id: i64,
    attachment_id: String,
) -> Result<()> {
    let dir = app.path().app_cache_dir()?.join("untis");
    let path = untis(&state)
        .attachment(message_id, &attachment_id, &dir)
        .await?;
    launch::path(&app, &path).await
}

#[tauri::command]
pub async fn untis_compose(state: State<'_, AppState>) -> Result<Compose> {
    Ok(untis(&state).compose().await?)
}

/// Neue Mitteilung; Anhänge als Dateipfade aus dem Auswahldialog.
#[tauri::command]
pub async fn untis_send_message(
    state: State<'_, AppState>,
    message: Outgoing,
    files: Vec<String>,
) -> Result<()> {
    let mut attachments = Vec::new();
    for path in files {
        let path = std::path::PathBuf::from(path);
        let bytes = std::fs::read(&path)
            .map_err(|e| CommandError::new("internal", format!("{}: {e}", path.display())))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Anhang".into());
        attachments.push((name, bytes));
    }
    Ok(untis(&state).send_message(message, &attachments).await?)
}

#[tauri::command]
pub async fn untis_day_news(state: State<'_, AppState>) -> Result<Vec<DayNews>> {
    Ok(untis(&state).day_news(today()).await?)
}

/// Mitteilungen in WebUntis im Browser öffnen (z. B. zum Antworten).
#[tauri::command]
pub async fn untis_open_web(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let url = untis(&state).web_url()?;
    launch::url(&app, &url).await
}

#[tauri::command]
pub async fn moodle_deadlines(state: State<'_, AppState>) -> Result<Vec<Deadline>> {
    Ok(tsa_moodle::Session::new(&state.db, state.secrets.as_ref())
        .deadlines()
        .await?)
}

/// Dateien der letzten drei Tage.
#[tauri::command]
pub fn moodle_recent_files(state: State<'_, AppState>) -> Result<Vec<RecentFile>> {
    let since = tsa_moodle::contents::now() - 3 * 24 * 3600;
    Ok(tsa_moodle::sync::recent(&state.db, since)?)
}

#[tauri::command]
pub fn mapping_list(state: State<'_, AppState>) -> Result<Vec<mapping::Entry>> {
    let subjects = untis(&state).subjects()?;
    Ok(mapping::list(&state.db, &subjects))
}

#[tauri::command]
pub fn mapping_set(
    state: State<'_, AppState>,
    subject: String,
    course_id: Option<i64>,
) -> Result<()> {
    Ok(mapping::set(&state.db, &subject, course_id)?)
}
