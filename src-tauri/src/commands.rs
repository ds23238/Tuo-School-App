use tauri::State;
use tsa_core::{Account, SecretKind, Service, Settings};
use tsa_moodle::session::SiteCheck;
use tsa_moodle::{Course, Session};

use crate::AppState;
use crate::error::Result;

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

#[tauri::command]
pub fn moodle_courses(state: State<AppState>) -> Result<Vec<Course>> {
    Ok(Course::list(&state.db)?)
}

#[tauri::command]
pub async fn moodle_refresh_courses(state: State<'_, AppState>) -> Result<Vec<Course>> {
    Ok(moodle(&state).refresh_courses().await?)
}

#[tauri::command]
pub fn moodle_set_course_sync(state: State<AppState>, id: i64, sync: bool) -> Result<()> {
    Ok(Course::set_sync(&state.db, id, sync)?)
}
