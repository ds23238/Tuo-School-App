//! Befehle für Moodle-Aktivitäten, die die App selbst anzeigt.

use tauri::{AppHandle, Manager, State};
use tsa_moodle::Session;
use tsa_moodle::activities::Cached;
use tsa_moodle::data::Database;
use tsa_moodle::forum::{Forum, Thread};

use crate::error::Result;
use crate::{AppState, launch, media};

fn moodle<'a>(state: &'a State<AppState>) -> Session<'a> {
    Session::new(&state.db, state.secrets.as_ref())
}

#[tauri::command]
pub async fn moodle_forum(
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
) -> Result<Cached<Forum>> {
    Ok(moodle(&state).forum(course_id, module_id).await?)
}

#[tauri::command]
pub async fn moodle_forum_thread(
    state: State<'_, AppState>,
    discussion_id: i64,
) -> Result<Cached<Thread>> {
    Ok(moodle(&state)
        .forum_thread(discussion_id, &media::urls())
        .await?)
}

#[tauri::command]
pub async fn moodle_forum_reply(
    state: State<'_, AppState>,
    post_id: i64,
    subject: String,
    text: String,
) -> Result<()> {
    Ok(moodle(&state).forum_reply(post_id, &subject, &text).await?)
}

#[tauri::command]
pub async fn moodle_forum_add(
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
    subject: String,
    text: String,
) -> Result<i64> {
    Ok(moodle(&state)
        .forum_add_discussion(course_id, module_id, &subject, &text)
        .await?)
}

#[tauri::command]
pub async fn moodle_forum_attachment(
    app: AppHandle,
    state: State<'_, AppState>,
    discussion_id: i64,
    post_id: i64,
    name: String,
) -> Result<()> {
    let dir = app.path().app_cache_dir()?.join("forum");
    let path = moodle(&state)
        .forum_attachment(discussion_id, post_id, &name, &dir)
        .await?;
    launch::path(&app, &path).await
}

#[tauri::command]
pub async fn moodle_database(
    state: State<'_, AppState>,
    course_id: i64,
    module_id: i64,
) -> Result<Cached<Database>> {
    Ok(moodle(&state)
        .database(course_id, module_id, &media::urls())
        .await?)
}
