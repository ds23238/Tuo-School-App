use std::collections::HashMap;
use std::sync::Mutex;

use crate::{Result, Service};

/// Name, unter dem die Einträge im Schlüsselbund des Systems erscheinen.
const KEYRING_SERVICE: &str = "TSA-Sync";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecretKind {
    Token,
    /// Moodle: berechtigt zum Anfordern von Auto-Login-Schlüsseln.
    PrivateToken,
    Password,
}

impl SecretKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::PrivateToken => "private-token",
            Self::Password => "password",
        }
    }
}

fn key(service: Service, kind: SecretKind) -> String {
    format!("{}:{}", service.as_str(), kind.as_str())
}

/// Ablage für Zugangsdaten. Im Programm der Schlüsselbund des Systems,
/// in Tests ein Speicher im RAM.
pub trait SecretStore: Send + Sync {
    fn set(&self, service: Service, kind: SecretKind, secret: &str) -> Result<()>;
    fn get(&self, service: Service, kind: SecretKind) -> Result<Option<String>>;
    /// Entfernt den Eintrag; ein fehlender Eintrag ist kein Fehler.
    fn delete(&self, service: Service, kind: SecretKind) -> Result<()>;

    fn delete_all(&self, service: Service) -> Result<()> {
        self.delete(service, SecretKind::Token)?;
        self.delete(service, SecretKind::PrivateToken)?;
        self.delete(service, SecretKind::Password)
    }
}

/// Windows Credential Manager bzw. Secret Service (GNOME Keyring, KWallet).
#[derive(Default)]
pub struct KeyringStore;

impl KeyringStore {
    fn entry(service: Service, kind: SecretKind) -> Result<keyring::Entry> {
        Ok(keyring::Entry::new(KEYRING_SERVICE, &key(service, kind))?)
    }
}

impl SecretStore for KeyringStore {
    fn set(&self, service: Service, kind: SecretKind, secret: &str) -> Result<()> {
        Ok(Self::entry(service, kind)?.set_password(secret)?)
    }

    fn get(&self, service: Service, kind: SecretKind) -> Result<Option<String>> {
        match Self::entry(service, kind)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn delete(&self, service: Service, kind: SecretKind) -> Result<()> {
        match Self::entry(service, kind)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[derive(Default)]
pub struct MemoryStore(Mutex<HashMap<String, String>>);

impl MemoryStore {
    fn map(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl SecretStore for MemoryStore {
    fn set(&self, service: Service, kind: SecretKind, secret: &str) -> Result<()> {
        self.map().insert(key(service, kind), secret.to_owned());
        Ok(())
    }

    fn get(&self, service: Service, kind: SecretKind) -> Result<Option<String>> {
        Ok(self.map().get(&key(service, kind)).cloned())
    }

    fn delete(&self, service: Service, kind: SecretKind) -> Result<()> {
        self.map().remove(&key(service, kind));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store() {
        let store = MemoryStore::default();
        store.set(Service::Moodle, SecretKind::Token, "t").unwrap();
        store
            .set(Service::Moodle, SecretKind::Password, "p")
            .unwrap();
        store.set(Service::Untis, SecretKind::Token, "u").unwrap();
        assert_eq!(
            store.get(Service::Moodle, SecretKind::Token).unwrap(),
            Some("t".into())
        );
        store.delete_all(Service::Moodle).unwrap();
        assert_eq!(store.get(Service::Moodle, SecretKind::Token).unwrap(), None);
        assert_eq!(
            store.get(Service::Moodle, SecretKind::Password).unwrap(),
            None
        );
        assert_eq!(
            store.get(Service::Untis, SecretKind::Token).unwrap(),
            Some("u".into())
        );
    }
}

#[cfg(test)]
mod keyring_tests {
    use super::*;

    /// Braucht einen echten Schlüsselbund; manuell starten mit
    /// `cargo test -p tsa-core -- --ignored`.
    #[test]
    #[ignore]
    fn keyring_roundtrip() {
        let store = KeyringStore;
        store
            .set(Service::Untis, SecretKind::Token, "probe")
            .unwrap();
        assert_eq!(
            store.get(Service::Untis, SecretKind::Token).unwrap(),
            Some("probe".into())
        );
        store.delete_all(Service::Untis).unwrap();
        assert_eq!(store.get(Service::Untis, SecretKind::Token).unwrap(), None);
    }
}
