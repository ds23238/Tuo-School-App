use std::collections::HashMap;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tsa_core::{Account, SecretKind, Service, Settings};
use tsa_moodle::assign::{Assignment, Draft};
use tsa_moodle::contents::CourseView;
use tsa_moodle::mail::{DraftInput, Folder, MailDraft, MailMessage, MailPage, MailStatus, Person};
use tsa_moodle::messages::{Chat, ChatMessage, Conversation, Member, Notification};
use tsa_moodle::pages::PageContent;
use tsa_moodle::session::{CourseSync, SiteCheck};
use tsa_moodle::sync::{CourseFiles, course_dir};
use tsa_moodle::{Course, Session};

use crate::error::Result;
use crate::{AppState, autostart, background, inbox, launch, media, tray};

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    Ok(Settings::load(&state.db, &state.default_sync_dir)?)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> Result<()> {
    settings.save(&state.db)?;
    state.background.wake();
    tray::refresh(&app);
    Ok(())
}

#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<Account>> {
    Ok(Account::list(&state.db)?)
}

#[tauri::command]
pub async fn remove_account(
    app: AppHandle,
    state: State<'_, AppState>,
    service: Service,
) -> Result<()> {
    crate::hybrid::reset(
        &app,
        match service {
            Service::Moodle => crate::hybrid::Site::Moodle,
            Service::Untis => crate::hybrid::Site::Untis,
        },
    )
    .await?;
    match service {
        Service::Moodle => {
            state.mail.reset().await;
            Ok(moodle(&state).sign_out()?)
        }
        Service::Untis => {
            Ok(
                tsa_untis::Session::new(&state.db, state.secrets.as_ref(), &state.untis)
                    .sign_out()
                    .await?,
            )
        }
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
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
    remember: bool,
) -> Result<Account> {
    crate::hybrid::reset(&app, crate::hybrid::Site::Moodle).await?;
    state.mail.reset().await;
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

#[tauri::command]
pub fn sync_status(state: State<AppState>) -> background::Status {
    state.background.status()
}

/// Synchronisiert alle ausgewählten Kurse; Fortschritt kommt als Event.
#[tauri::command]
pub async fn sync_now(app: AppHandle) -> Result<Vec<CourseSync>> {
    background::run(&app)
        .await
        .map_err(|e| crate::error::CommandError::new("sync", e))
}

/// Merkt sich den Besuch (für „zuletzt geöffnet“) und meldet ihn Moodle.
#[tauri::command]
pub async fn moodle_course_viewed(state: State<'_, AppState>, id: i64) -> Result<()> {
    Ok(moodle(&state).course_viewed(id).await?)
}

#[tauri::command]
pub async fn moodle_sync_course(app: AppHandle, id: i64) -> Result<usize> {
    background::run_course(&app, id)
        .await
        .map_err(|e| crate::error::CommandError::new("sync", e))
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

#[tauri::command]
pub fn autostart_enabled(app: AppHandle, state: State<'_, AppState>) -> Result<bool> {
    autostart::enabled(&app, &state)
}

#[tauri::command]
pub async fn set_autostart(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<()> {
    autostart::set(&app, &state, enabled).await
}

/// Die Grafik-Einstellung gibt es nur unter Linux (WebKitGTK).
#[tauri::command]
pub fn graphics_configurable() -> bool {
    cfg!(target_os = "linux")
}

#[tauri::command]
pub fn message_counts(state: State<'_, AppState>) -> inbox::Totals {
    state.inbox.counts()
}

#[tauri::command]
pub async fn refresh_message_counts(app: AppHandle) -> inbox::Totals {
    inbox::check(&app).await
}

#[tauri::command]
pub async fn moodle_conversations(state: State<'_, AppState>) -> Result<Vec<Conversation>> {
    Ok(moodle(&state).conversations(&media::urls()).await?)
}

#[tauri::command]
pub async fn moodle_chat(
    state: State<'_, AppState>,
    conversation_id: i64,
    before: usize,
) -> Result<Chat> {
    Ok(moodle(&state)
        .chat(conversation_id, before, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn moodle_mark_chat_read(
    app: AppHandle,
    state: State<'_, AppState>,
    conversation_id: i64,
) -> Result<()> {
    moodle(&state).mark_chat_read(conversation_id).await?;
    inbox::check(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn moodle_send_chat(
    state: State<'_, AppState>,
    conversation_id: i64,
    text: String,
) -> Result<ChatMessage> {
    Ok(moodle(&state)
        .send_chat(conversation_id, &text, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn moodle_send_to_user(
    state: State<'_, AppState>,
    user_id: i64,
    text: String,
) -> Result<i64> {
    Ok(moodle(&state).send_to_user(user_id, &text).await?)
}

#[tauri::command]
pub async fn moodle_search_people(state: State<'_, AppState>, text: String) -> Result<Vec<Member>> {
    Ok(moodle(&state).search_people(&text, &media::urls()).await?)
}

#[tauri::command]
pub async fn moodle_notifications(state: State<'_, AppState>) -> Result<Vec<Notification>> {
    Ok(moodle(&state).notifications(&media::urls()).await?)
}

#[tauri::command]
pub async fn moodle_mark_notification_read(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<()> {
    moodle(&state).mark_notification_read(id).await?;
    inbox::check(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn moodle_mark_all_notifications_read(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<()> {
    moodle(&state).mark_all_notifications_read().await?;
    inbox::check(&app).await;
    Ok(())
}

#[tauri::command]
pub async fn mail_status(state: State<'_, AppState>) -> Result<Option<MailStatus>> {
    Ok(moodle(&state).mail_status(&state.mail).await?)
}

#[tauri::command]
pub async fn mail_list(
    state: State<'_, AppState>,
    folder: Folder,
    course_id: Option<i64>,
    search: String,
    offset: usize,
) -> Result<MailPage> {
    Ok(moodle(&state)
        .mail_list(
            &state.mail,
            folder,
            course_id,
            &search,
            offset,
            &media::urls(),
        )
        .await?)
}

#[tauri::command]
pub async fn mail_message(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> Result<MailMessage> {
    let message = moodle(&state)
        .mail_message(&state.mail, id, &media::urls())
        .await?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move { inbox::check(&app).await });
    Ok(message)
}

#[tauri::command]
pub async fn mail_set_flag(
    state: State<'_, AppState>,
    id: i64,
    flag: String,
    value: bool,
) -> Result<()> {
    Ok(moodle(&state)
        .mail_set_flag(&state.mail, id, &flag, value)
        .await?)
}

#[tauri::command]
pub async fn mail_set_deleted(state: State<'_, AppState>, id: i64, deleted: u8) -> Result<()> {
    Ok(moodle(&state)
        .mail_set_deleted(&state.mail, id, deleted)
        .await?)
}

#[tauri::command]
pub async fn mail_search_users(
    state: State<'_, AppState>,
    course_id: i64,
    text: String,
) -> Result<Vec<Person>> {
    Ok(moodle(&state)
        .mail_search_users(&state.mail, course_id, &text, &media::urls())
        .await?)
}

/// `kind`: `new` (mit Kurs-ID), `reply`, `replyAll` oder `forward` (mit
/// Nachrichten-ID).
#[tauri::command]
pub async fn mail_create(state: State<'_, AppState>, kind: String, id: i64) -> Result<MailDraft> {
    Ok(moodle(&state)
        .mail_create(&state.mail, &kind, id, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn mail_draft(state: State<'_, AppState>, id: i64) -> Result<MailDraft> {
    Ok(moodle(&state)
        .mail_draft(&state.mail, id, false, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn mail_save(
    state: State<'_, AppState>,
    id: i64,
    input: DraftInput,
    send: bool,
) -> Result<()> {
    Ok(moodle(&state)
        .mail_save(&state.mail, id, input, send)
        .await?)
}

#[tauri::command]
pub async fn mail_open_attachment(
    app: AppHandle,
    state: State<'_, AppState>,
    message_id: i64,
    name: String,
) -> Result<()> {
    let dir = app.path().app_cache_dir()?.join("mail");
    let path = moodle(&state)
        .mail_attachment(&state.mail, message_id, &name, &dir)
        .await?;
    launch::path(&app, &path).await
}
