use tauri::State;
use tsa_core::{Account, Result, SecretKind, Service, Settings};

use crate::AppState;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings> {
    Settings::load(&state.db, &state.default_sync_dir)
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> Result<()> {
    settings.save(&state.db)
}

#[tauri::command]
pub fn list_accounts(state: State<AppState>) -> Result<Vec<Account>> {
    Account::list(&state.db)
}

#[tauri::command]
pub fn remove_account(state: State<AppState>, service: Service) -> Result<()> {
    Account::remove(&state.db, state.secrets.as_ref(), service)
}

/// Prüft, ob der Schlüsselbund des Systems erreichbar ist.
#[tauri::command]
pub fn check_keyring(state: State<AppState>) -> Result<()> {
    state
        .secrets
        .get(Service::Moodle, SecretKind::Token)
        .map(drop)
}
