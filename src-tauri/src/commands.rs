use std::collections::HashMap;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tsa_core::{Account, SecretKind, Service, Settings};
use tsa_moodle::assign::{Assignment, Draft};
use tsa_moodle::contents::CourseView;
use tsa_moodle::pages::PageContent;
use tsa_moodle::session::{CourseSync, SiteCheck};
use tsa_moodle::sync::{CourseFiles, course_dir};
use tsa_moodle::{Course, Session};

use crate::error::Result;
use crate::{AppState, launch, media};

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    Ok(Settings::load(&state.db, &state.default_sync_dir)?)
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<()> {
    Ok(settings.save(&state.db)?)
}

#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<Account>> {
    Ok(Account::list(&state.db)?)
}

#[tauri::command]
pub fn remove_account(state: State<AppState>, service: Service) -> Result<()> {
    match service {
        Service::Moodle => Ok(moodle(&state).sign_out()?),
        Service::Untis => Ok(Account::remove(&state.db, state.secrets.as_ref(), service)?),
    }
}

/// Prüft, ob der Schlüsselbund des Systems erreichbar ist.
#[tauri::command]
pub fn check_keyring(state: State<AppState>) -> Result<()> {
    Ok(state
        .secrets
        .get(Service::Moodle, SecretKind::Token)
        .map(drop)?)
}

fn moodle<'a>(state: &'a State<AppState>) -> Session<'a> {
    Session::new(&state.db, state.secrets.as_ref())
}

#[tauri::command]
pub async fn moodle_check_site(url: String) -> Result<SiteCheck> {
    Ok(Session::check_site(&url).await?)
}

#[tauri::command]
pub async fn moodle_login(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
    remember: bool,
) -> Result<Account> {
    Ok(moodle(&state)
        .login(&url, &username, &password, remember)
        .await?)
}

fn with_media(courses: Vec<Course>) -> Vec<Course> {
    let media = media::urls();
    courses.into_iter().map(|c| c.with_media(&media)).collect()
}

#[tauri::command]
pub fn moodle_courses(state: State<AppState>) -> Result<Vec<Course>> {
    Ok(with_media(Course::list(&state.db)?))
}

#[tauri::command]
pub async fn moodle_refresh_courses(state: State<'_, AppState>) -> Result<Vec<Course>> {
    Ok(with_media(moodle(&state).refresh_courses().await?))
}

#[tauri::command]
pub fn moodle_set_course_sync(state: State<AppState>, id: i64, sync: bool) -> Result<()> {
    Ok(Course::set_sync(&state.db, id, sync)?)
}

/// Zuletzt geladener Stand, sofort und offline verfügbar.
fn sync_dir(state: &AppState) -> Result<PathBuf> {
    Ok(Settings::load(&state.db, &state.default_sync_dir)?.sync_dir)
}

#[tauri::command]
pub async fn moodle_course(state: State<'_, AppState>, id: i64) -> Result<Option<CourseView>> {
    let root = sync_dir(&state)?;
    Ok(moodle(&state)
        .cached_course(id, &media::urls(), &root)
        .await?)
}

#[tauri::command]
pub async fn moodle_refresh_course(state: State<'_, AppState>, id: i64) -> Result<CourseView> {
    let root = sync_dir(&state)?;
    Ok(moodle(&state)
        .refresh_course(id, &media::urls(), &root)
        .await?)
}

#[tauri::command]
pub fn moodle_file_stats(state: State<AppState>) -> Result<HashMap<i64, CourseFiles>> {
    Ok(moodle(&state).file_stats()?)
}

/// Synchronisiert alle ausgewählten Kurse.
#[tauri::command]
pub async fn moodle_sync(state: State<'_, AppState>) -> Result<Vec<CourseSync>> {
    let root = sync_dir(&state)?;
    Ok(moodle(&state).sync_all(&root).await?)
}

#[tauri::command]
pub async fn moodle_sync_course(state: State<'_, AppState>, id: i64) -> Result<usize> {
    let root = sync_dir(&state)?;
    Ok(moodle(&state).sync_course(id, &root).await?)
}

/// Öffnet die lokale Kopie; lädt sie vorher, falls nötig.
#[tauri::command]
pub async fn moodle_open_file(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<()> {
    let root = sync_dir(&state)?;
    let path = moodle(&state).local_file(id, &root).await?;
    launch::path(&app, &path).await
}

/// Holt die neue Version; die bisherige Datei bleibt umbenannt erhalten.
#[tauri::command]
pub async fn moodle_update_file(state: State<'_, AppState>, id: i64) -> Result<()> {
    let root = sync_dir(&state)?;
    moodle(&state).update_file(id, &root).await?;
    Ok(())
}

#[tauri::command]
pub async fn moodle_open_course_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<()> {
    let course = Course::list(&state.db)?
        .into_iter()
        .find(|c| c.id == id)
        .ok_or(tsa_moodle::Error::FileNotFound)?;
    let dir = sync_dir(&state)?.join(course_dir(&course.full_name));
    std::fs::create_dir_all(&dir).map_err(tsa_moodle::Error::Io)?;
    launch::path(&app, &dir).await
}

/// Öffnet eine Moodle-Seite im Browser, wenn möglich bereits angemeldet.
#[tauri::command]
pub async fn moodle_open_in_browser(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
) -> Result<()> {
    let url = moodle(&state).autologin_url(&url).await?;
    launch::url(&app, &url).await
}

#[tauri::command]
pub async fn moodle_set_completion(
    state: State<'_, AppState>,
    module_id: i64,
    completed: bool,
) -> Result<()> {
    Ok(moodle(&state).set_completion(module_id, completed).await?)
}

#[tauri::command]
pub async fn moodle_page(
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
) -> Result<PageContent> {
    Ok(moodle(&state)
        .page_content(course_id, module_id, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn moodle_assignment(
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
) -> Result<Assignment> {
    Ok(moodle(&state)
        .assignment(course_id, module_id, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn moodle_save_submission(
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
    draft: Draft,
) -> Result<()> {
    Ok(moodle(&state)
        .save_submission(course_id, module_id, draft, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn moodle_submit(
    state: State<'_, AppState>,
    assign_id: i64,
    accept_statement: bool,
) -> Result<()> {
    Ok(moodle(&state)
        .submit_for_grading(assign_id, accept_statement)
        .await?)
}

/// Öffnet eine abgegebene Datei oder eine Feedback-Datei.
#[tauri::command]
pub async fn moodle_open_assignment_file(
    app: AppHandle,
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
    feedback: bool,
    name: String,
) -> Result<()> {
    let dir = app.path().app_cache_dir()?.join("assign");
    let path = moodle(&state)
        .assignment_file(course_id, module_id, feedback, &name, &dir, &media::urls())
        .await?;
    launch::path(&app, &path).await
}

/// Öffnet einen Web- oder Mail-Link im Standardprogramm.
#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> Result<()> {
    launch::url(&app, &url).await
}
