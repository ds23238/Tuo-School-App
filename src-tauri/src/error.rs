use serde::Serialize;

/// Fehler, wie die Oberfläche sie bekommt: ein stabiler Code für die
/// Übersetzung und der Text als Rückfall.
#[derive(Debug, Serialize)]
pub struct CommandError {
    code: &'static str,
    message: String,
}

impl CommandError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<tsa_core::Error> for CommandError {
    fn from(e: tsa_core::Error) -> Self {
        Self {
            code: "internal",
            message: e.to_string(),
        }
    }
}

impl From<tsa_moodle::Error> for CommandError {
    fn from(e: tsa_moodle::Error) -> Self {
        Self {
            code: e.code(),
            message: e.to_string(),
        }
    }
}

impl From<tauri_plugin_opener::Error> for CommandError {
    fn from(e: tauri_plugin_opener::Error) -> Self {
        Self {
            code: "open",
            message: e.to_string(),
        }
    }
}

impl From<tauri::Error> for CommandError {
    fn from(e: tauri::Error) -> Self {
        Self {
            code: "internal",
            message: e.to_string(),
        }
    }
}

pub type Result<T, E = CommandError> = std::result::Result<T, E>;
