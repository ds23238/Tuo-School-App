//! Symbol im Infobereich: Fenster öffnen, synchronisieren, beenden.

use std::sync::Mutex;

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tsa_core::Settings;

use crate::{AppState, background, texts};

struct Items {
    open: MenuItem<Wry>,
    sync_now: MenuItem<Wry>,
    auto_sync: CheckMenuItem<Wry>,
    quit: MenuItem<Wry>,
}

#[derive(Default)]
pub struct Tray(Mutex<Option<Items>>);

/// Legt das Symbol an. Fehlt dem System die Unterstützung (unter Linux z. B.
/// ohne AppIndicator), kommt ein Fehler zurück und die App läuft ohne Tray.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "", true, None::<&str>)?;
    let sync_now = MenuItem::with_id(app, "sync", "", true, None::<&str>)?;
    let auto_sync = CheckMenuItem::with_id(app, "auto", "", true, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &sync_now, &auto_sync, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .menu(&menu)
        .tooltip("TSA-Sync")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_window(app),
            "sync" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = background::run(&app).await;
                });
            }
            "auto" => toggle_auto_sync(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    // Das Symbol geht als Datei an den Infobereich des Systems. Aus dem
    // Flatpak ist dafür nur dieser Ordner auch außerhalb sichtbar.
    if let (Some(id), Some(runtime)) = (
        std::env::var_os("FLATPAK_ID"),
        std::env::var_os("XDG_RUNTIME_DIR"),
    ) {
        let dir = std::path::Path::new(&runtime)
            .join("app")
            .join(id)
            .join("tray-icon");
        builder = builder.temp_dir_path(dir);
    }
    builder.build(app)?;

    *app.state::<Tray>().0.lock().unwrap() = Some(Items {
        open,
        sync_now,
        auto_sync,
        quit,
    });
    refresh(app);
    Ok(())
}

/// Texte (Sprache) und Häkchen an die aktuellen Einstellungen anpassen.
pub fn refresh(app: &AppHandle) {
    let (Some(tray), Some(state)) = (app.try_state::<Tray>(), app.try_state::<AppState>()) else {
        return;
    };
    let guard = tray.0.lock().unwrap_or_else(|e| e.into_inner());
    let Some(items) = guard.as_ref() else {
        return;
    };
    let Ok(settings) = Settings::load(&state.db, &state.default_sync_dir) else {
        return;
    };
    let t = texts::for_settings(&settings);
    let running = state.background.status().running;
    let _ = items.open.set_text(t.open);
    let _ = items.sync_now.set_text(t.sync_now);
    let _ = items.sync_now.set_enabled(!running);
    let _ = items.auto_sync.set_text(t.auto_sync);
    let _ = items.auto_sync.set_checked(settings.sync_enabled);
    let _ = items.quit.set_text(t.quit);
}

fn toggle_auto_sync(app: &AppHandle) {
    let state = app.state::<AppState>();
    if let Ok(mut settings) = Settings::load(&state.db, &state.default_sync_dir) {
        settings.sync_enabled = !settings.sync_enabled;
        let _ = settings.save(&state.db);
        state.background.wake();
    }
    refresh(app);
}

pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
