use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::{Error, Result};

/// Migrationen in Reihenfolge; Index + 1 ist die Schema-Version.
/// Bestehende Einträge niemals ändern, nur neue anhängen.
const MIGRATIONS: &[&str] = &["CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE accounts (
        service      TEXT PRIMARY KEY,
        base_url     TEXT NOT NULL,
        username     TEXT NOT NULL,
        display_name TEXT,
        created_at   INTEGER NOT NULL
    );"];

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        // Ein Panic während einer Abfrage lässt die Verbindung selbst intakt.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let supported = MIGRATIONS.len() as i64;
    let current: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if current > supported {
        return Err(Error::SchemaTooNew {
            found: current,
            supported,
        });
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i as i64 + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(db: &Database) -> i64 {
        db.conn()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn migrates_to_latest() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(version(&db), MIGRATIONS.len() as i64);
    }

    #[test]
    fn reopen_keeps_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");
        drop(Database::open(&path).unwrap());
        let db = Database::open(&path).unwrap();
        assert_eq!(version(&db), MIGRATIONS.len() as i64);
    }

    #[test]
    fn rejects_newer_schema() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");
        Connection::open(&path)
            .unwrap()
            .pragma_update(None, "user_version", 999)
            .unwrap();
        assert!(matches!(
            Database::open(&path),
            Err(Error::SchemaTooNew { found: 999, .. })
        ));
    }
}
