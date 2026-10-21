use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Datenbankfehler: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Ungültige Daten: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Schlüsselbund nicht verfügbar: {0}")]
    Secret(#[from] keyring::Error),
    #[error("Datenbank-Schema {module} {found} ist neuer als diese Version ({supported})")]
    SchemaTooNew {
        module: String,
        found: i64,
        supported: i64,
    },
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

// Fehler gehen als Text an die Oberfläche.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
