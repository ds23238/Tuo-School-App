use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::{Database, Result, SecretStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Service {
    Moodle,
    Untis,
}

impl Service {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Moodle => "moodle",
            Self::Untis => "untis",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "moodle" => Some(Self::Moodle),
            "untis" => Some(Self::Untis),
            _ => None,
        }
    }
}

/// Höchstens ein Konto pro Dienst. Zugangsdaten liegen nicht hier,
/// sondern im [`SecretStore`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub service: Service,
    pub base_url: String,
    pub username: String,
    pub display_name: Option<String>,
    /// Unix-Zeit in Sekunden.
    pub created_at: i64,
}

impl Account {
    pub fn new(service: Service, base_url: String, username: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        Self {
            service,
            base_url,
            username,
            display_name: None,
            created_at,
        }
    }

    fn from_row(row: &Row) -> rusqlite::Result<Option<Self>> {
        let service: String = row.get("service")?;
        // Unbekannte Dienste (z. B. aus einer neueren Version) überspringen.
        let Some(service) = Service::parse(&service) else {
            return Ok(None);
        };
        Ok(Some(Self {
            service,
            base_url: row.get("base_url")?,
            username: row.get("username")?,
            display_name: row.get("display_name")?,
            created_at: row.get("created_at")?,
        }))
    }

    pub fn get(db: &Database, service: Service) -> Result<Option<Self>> {
        let account = db
            .conn()
            .query_row(
                "SELECT * FROM accounts WHERE service = ?1",
                [service.as_str()],
                Self::from_row,
            )
            .optional()?;
        Ok(account.flatten())
    }

    pub fn list(db: &Database) -> Result<Vec<Self>> {
        let conn = db.conn();
        let mut stmt = conn.prepare("SELECT * FROM accounts ORDER BY service")?;
        let rows = stmt.query_map([], Self::from_row)?;
        let mut accounts = Vec::new();
        for row in rows {
            accounts.extend(row?);
        }
        Ok(accounts)
    }

    /// Legt das Konto an oder ersetzt das bestehende Konto desselben Dienstes.
    pub fn save(&self, db: &Database) -> Result<()> {
        db.conn().execute(
            "INSERT OR REPLACE INTO accounts
                 (service, base_url, username, display_name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            (
                self.service.as_str(),
                &self.base_url,
                &self.username,
                &self.display_name,
                self.created_at,
            ),
        )?;
        Ok(())
    }

    /// Entfernt Konto und Zugangsdaten. Die Zugangsdaten zuerst, damit bei
    /// einem Fehler im Schlüsselbund kein verwaistes Token zurückbleibt.
    pub fn remove(db: &Database, secrets: &dyn SecretStore, service: Service) -> Result<()> {
        secrets.delete_all(service)?;
        db.conn().execute(
            "DELETE FROM accounts WHERE service = ?1",
            [service.as_str()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MemoryStore, SecretKind};

    fn moodle() -> Account {
        Account::new(
            Service::Moodle,
            "https://moodle.example".into(),
            "anna".into(),
        )
    }

    #[test]
    fn save_get_list() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(Account::get(&db, Service::Moodle).unwrap(), None);
        let a = moodle();
        a.save(&db).unwrap();
        assert_eq!(Account::get(&db, Service::Moodle).unwrap(), Some(a.clone()));
        assert_eq!(Account::list(&db).unwrap(), vec![a]);
    }

    #[test]
    fn one_account_per_service() {
        let db = Database::open_in_memory().unwrap();
        moodle().save(&db).unwrap();
        let mut other = moodle();
        other.username = "ben".into();
        other.save(&db).unwrap();
        let list = Account::list(&db).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].username, "ben");
    }

    #[test]
    fn remove_deletes_secrets() {
        let db = Database::open_in_memory().unwrap();
        let secrets = MemoryStore::default();
        moodle().save(&db).unwrap();
        secrets
            .set(Service::Moodle, SecretKind::Token, "t")
            .unwrap();
        Account::remove(&db, &secrets, Service::Moodle).unwrap();
        assert_eq!(Account::get(&db, Service::Moodle).unwrap(), None);
        assert_eq!(
            secrets.get(Service::Moodle, SecretKind::Token).unwrap(),
            None
        );
    }

    #[test]
    fn skips_unknown_service() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO accounts VALUES ('future', 'x', 'y', NULL, 0)",
                [],
            )
            .unwrap();
        assert!(Account::list(&db).unwrap().is_empty());
    }
}
