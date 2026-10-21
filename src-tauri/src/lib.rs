pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("Tauri-Anwendung konnte nicht gestartet werden");
}
