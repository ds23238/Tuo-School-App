use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, Transaction};

use crate::{Error, Result};

/// Migrationen in Reihenfolge; Index + 1 ist die Schema-Version.
/// Bestehende Einträge niemals ändern, nur neue anhängen.
const MIGRATIONS: &[&str] = &[
    "CREATE TABLE settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    CREATE TABLE accounts (
        service      TEXT PRIMARY KEY,
        base_url     TEXT NOT NULL,
        username     TEXT NOT NULL,
        display_name TEXT,
        created_at   INTEGER NOT NULL
    );",
    "CREATE TABLE schema_versions (
        module  TEXT PRIMARY KEY,
        version INTEGER NOT NULL
    );",
];

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
        let current = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        apply(&mut conn, "core", current, MIGRATIONS, |tx, v| {
            tx.pragma_update(None, "user_version", v)
        })?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> MutexGuard<'_, Connection> {
        // Ein Panic während einer Abfrage lässt die Verbindung selbst intakt.
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Bringt die Tabellen eines Moduls (z. B. `moodle`) auf den neuesten Stand.
    /// Für die Liste gelten dieselben Regeln wie für [`MIGRATIONS`].
    pub fn migrate_module(&self, module: &str, migrations: &[&str]) -> Result<()> {
        let mut conn = self.conn();
        let current = conn
            .query_row(
                "SELECT version FROM schema_versions WHERE module = ?1",
                [module],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        apply(&mut conn, module, current, migrations, |tx, v| {
            tx.execute(
                "INSERT OR REPLACE INTO schema_versions (module, version) VALUES (?1, ?2)",
                (module, v),
            )
            .map(drop)
        })
    }
}

fn apply(
    conn: &mut Connection,
    module: &str,
    current: i64,
    migrations: &[&str],
    set_version: impl Fn(&Transaction, i64) -> rusqlite::Result<()>,
) -> Result<()> {
    let supported = migrations.len() as i64;
    if current > supported {
        return Err(Error::SchemaTooNew {
            module: module.to_owned(),
            found: current,
            supported,
        });
    }
    for (i, sql) in migrations.iter().enumerate().skip(current as usize) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        set_version(&tx, i as i64 + 1)?;
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

    const MODULE: &[&str] = &["CREATE TABLE a (x)", "CREATE TABLE b (y)"];

    #[test]
    fn module_migrations() {
        let db = Database::open_in_memory().unwrap();
        db.migrate_module("test", &MODULE[..1]).unwrap();
        db.migrate_module("test", MODULE).unwrap();
        db.migrate_module("test", MODULE).unwrap();
        db.conn().execute("INSERT INTO b VALUES (1)", []).unwrap();
        assert!(matches!(
            db.migrate_module("test", &MODULE[..1]),
            Err(Error::SchemaTooNew {
                found: 2,
                supported: 1,
                ..
            })
        ));
    }
}
