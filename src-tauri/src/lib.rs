mod commands;

use std::path::PathBuf;

use tauri::Manager;
use tsa_core::{Database, KeyringStore, SecretStore};

pub struct AppState {
    pub db: Database,
    pub secrets: Box<dyn SecretStore>,
    pub default_sync_dir: PathBuf,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            app.manage(AppState {
                db: Database::open(&data_dir.join("tsa-sync.db"))?,
                secrets: Box::new(KeyringStore),
                default_sync_dir: app.path().home_dir()?.join("TSA-Sync"),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_accounts,
            commands::remove_account,
            commands::check_keyring,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri-Anwendung konnte nicht gestartet werden");
}
