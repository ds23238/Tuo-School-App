#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Keine Verbindung: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Zu viele Schulen gefunden, bitte genauer suchen")]
    TooManySchools,
    #[error("Benutzername oder Passwort falsch")]
    InvalidLogin,
    #[error("Anmeldung abgelaufen")]
    SessionExpired,
    #[error("Nicht bei WebUntis angemeldet")]
    NotSignedIn,
    #[error("{message}")]
    Untis { code: i64, message: String },
    #[error("Unerwartete Antwort von WebUntis: {0}")]
    Response(String),
    #[error(transparent)]
    Core(#[from] tsa_core::Error),
}

impl Error {
    /// Kurzer, stabiler Code für die Oberfläche.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Network(_) => "network",
            Self::TooManySchools => "tooManySchools",
            Self::InvalidLogin => "invalidLogin",
            Self::SessionExpired => "untisExpired",
            Self::NotSignedIn => "notSignedIn",
            Self::Untis { .. } => "untis",
            Self::Response(_) => "untisResponse",
            Self::Core(_) => "internal",
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Response(e.to_string())
    }
}

impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Core(e.into())
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
