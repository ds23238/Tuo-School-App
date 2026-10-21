#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Ungültige Adresse: {0}")]
    InvalidUrl(String),
    #[error("Keine Verbindung: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Unter dieser Adresse wurde keine Moodle-Seite gefunden")]
    NotMoodle,
    #[error("Die Moodle-Seite erlaubt keinen Zugriff über die App-Schnittstelle")]
    WebServicesDisabled,
    #[error("Benutzername oder Passwort falsch")]
    InvalidLogin,
    #[error("Anmeldung abgelaufen")]
    InvalidToken,
    #[error("Nicht bei Moodle angemeldet")]
    NotSignedIn,
    #[error("{message}")]
    Moodle { code: String, message: String },
    #[error("Unerwartete Antwort von Moodle: {0}")]
    Response(#[from] serde_json::Error),
    #[error("Datei nicht gefunden")]
    FileNotFound,
    #[error("Dateifehler: {0}")]
    Io(std::io::Error),
    #[error(transparent)]
    Core(#[from] tsa_core::Error),
}

impl Error {
    /// Kurzer, stabiler Code für die Oberfläche.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidUrl(_) => "invalidUrl",
            Self::Network(_) => "network",
            Self::NotMoodle => "notMoodle",
            Self::WebServicesDisabled => "webServicesDisabled",
            Self::InvalidLogin => "invalidLogin",
            Self::InvalidToken => "invalidToken",
            Self::NotSignedIn => "notSignedIn",
            Self::Moodle { .. } => "moodle",
            Self::Response(_) => "response",
            Self::FileNotFound => "fileNotFound",
            Self::Io(_) => "io",
            Self::Core(_) => "internal",
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Core(e.into())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
