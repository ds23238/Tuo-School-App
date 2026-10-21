mod autostart;
mod background;
mod commands;
mod error;
mod graphics;
mod inbox;
mod launch;
mod media;
mod texts;
mod tray;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{Manager, WindowEvent};
use tsa_core::{Database, KeyringStore, SecretStore};

pub struct AppState {
    pub db: Database,
    pub secrets: Box<dyn SecretStore>,
    pub default_sync_dir: PathBuf,
    pub background: background::Background,
    pub inbox: inbox::Inbox,
    pub mail: tsa_moodle::mail::MailState,
    /// Ohne Tray-Symbol beendet Schließen die App, statt sie zu verstecken.
    pub has_tray: AtomicBool,
}

/// Argument beim Autostart: im Tray starten, ohne Fenster.
const MINIMIZED: &str = "--minimized";

pub fn run() {
    #[cfg(target_os = "linux")]
    let graphics_mode = graphics::prepare("io.github.ds23238.tsasync");

    tauri::Builder::default()
        // Ein zweiter Start holt nur das vorhandene Fenster nach vorne.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            tray::show_window(app)
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![MINIMIZED]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol(media::SCHEME, |ctx, request, responder| {
            media::handle(ctx.app_handle(), request, responder)
        })
        .setup(move |app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db = Database::open(&data_dir.join("tsa-sync.db"))?;
            tsa_moodle::init(&db)?;
            app.manage(AppState {
                db,
                secrets: Box::new(KeyringStore),
                default_sync_dir: app.path().home_dir()?.join("TSA-Sync"),
                background: background::Background::default(),
                inbox: inbox::Inbox::default(),
                mail: tsa_moodle::mail::MailState::default(),
                has_tray: AtomicBool::new(false),
            });
            app.manage(tray::Tray::default());

            let handle = app.handle();
            let has_tray = match tray::create(handle) {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("Kein Tray-Symbol verfügbar: {e}");
                    false
                }
            };
            app.state::<AppState>()
                .has_tray
                .store(has_tray, Ordering::Relaxed);
            // Das Fenster startet sichtbar: unter Linux reagieren die Buttons der
            // Titelleiste sonst teils nicht mehr, wenn es erst später erscheint.
            #[cfg(target_os = "linux")]
            if let Some(window) = app.get_webview_window("main") {
                graphics::apply(&window, &graphics_mode);
            }
            let minimized = std::env::args().any(|a| a == MINIMIZED);
            if minimized && has_tray {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            background::init(handle);
            inbox::init(handle);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                if state.has_tray.load(Ordering::Relaxed) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::list_accounts,
            commands::remove_account,
            commands::check_keyring,
            commands::open_url,
            commands::autostart_enabled,
            commands::set_autostart,
            commands::graphics_configurable,
            commands::moodle_check_site,
            commands::moodle_login,
            commands::moodle_courses,
            commands::moodle_refresh_courses,
            commands::moodle_set_course_sync,
            commands::moodle_course,
            commands::moodle_refresh_course,
            commands::moodle_file_stats,
            commands::sync_status,
            commands::sync_now,
            commands::moodle_course_viewed,
            commands::moodle_sync_course,
            commands::moodle_open_file,
            commands::moodle_update_file,
            commands::moodle_open_course_folder,
            commands::moodle_open_in_browser,
            commands::moodle_set_completion,
            commands::moodle_page,
            commands::moodle_assignment,
            commands::moodle_save_submission,
            commands::moodle_submit,
            commands::moodle_open_assignment_file,
            commands::message_counts,
            commands::refresh_message_counts,
            commands::moodle_conversations,
            commands::moodle_chat,
            commands::moodle_mark_chat_read,
            commands::moodle_send_chat,
            commands::moodle_send_to_user,
            commands::moodle_search_people,
            commands::moodle_notifications,
            commands::moodle_mark_notification_read,
            commands::moodle_mark_all_notifications_read,
            commands::mail_status,
            commands::mail_list,
            commands::mail_message,
            commands::mail_set_flag,
            commands::mail_set_deleted,
            commands::mail_search_users,
            commands::mail_create,
            commands::mail_draft,
            commands::mail_save,
            commands::mail_open_attachment,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri-Anwendung konnte nicht gestartet werden");
}
