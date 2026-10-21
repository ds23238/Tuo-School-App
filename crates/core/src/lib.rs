//! Gemeinsamer Kern: Datenbank, Einstellungen, Konten und Zugangsdaten.

pub mod accounts;
pub mod db;
pub mod error;
pub mod secrets;
pub mod settings;

pub use accounts::{Account, Service};
pub use db::Database;
pub use error::{Error, Result};
pub use secrets::{KeyringStore, MemoryStore, SecretKind, SecretStore};
pub use settings::Settings;
