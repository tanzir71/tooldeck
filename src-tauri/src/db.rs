use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

const MIGRATION_001: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002: &str = include_str!("../migrations/002_notes_time.sql");
const MIGRATION_003: &str = include_str!("../migrations/003_safety_workflows.sql");
const MIGRATIONS: &[(i64, &str)] = &[(1, MIGRATION_001), (2, MIGRATION_002), (3, MIGRATION_003)];

pub struct Database {
    path: PathBuf,
    connection: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStatus {
    pub path: String,
    pub schema_version: i64,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, rusqlite::Error> {
        let path = path.as_ref().to_path_buf();
        let connection = Connection::open(&path)?;
        Self::configure(&connection)?;
        Self::migrate(&connection)?;
        Ok(Self {
            path,
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub fn open_memory() -> Result<Self, rusqlite::Error> {
        let connection = Connection::open_in_memory()?;
        Self::configure(&connection)?;
        Self::migrate(&connection)?;
        Ok(Self {
            path: PathBuf::from(":memory:"),
            connection: Mutex::new(connection),
        })
    }

    fn configure(connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA busy_timeout = 5000;
            ",
        )
    }

    fn migrate(connection: &Connection) -> Result<(), rusqlite::Error> {
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        for (version, sql) in MIGRATIONS {
            let applied = connection
                .query_row(
                    "SELECT version FROM schema_migrations WHERE version = ?1",
                    [version],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some();
            if !applied {
                let transaction = connection.unchecked_transaction()?;
                transaction.execute_batch(sql)?;
                transaction.execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    [version],
                )?;
                transaction.commit()?;
            }
        }
        Ok(())
    }

    pub fn with_connection<T>(
        &self,
        action: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "database lock was poisoned".to_string())?;
        action(&connection)
    }

    pub fn status(&self) -> Result<DbStatus, String> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| "database lock was poisoned".to_string())?;
        let schema_version = connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        Ok(DbStatus {
            path: self.path.to_string_lossy().to_string(),
            schema_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_migration_is_idempotent() {
        let database = Database::open_memory().expect("in-memory database opens");
        assert_eq!(database.status().expect("status").schema_version, 3);
        let connection = database.connection.lock().expect("database lock");
        Database::migrate(&connection).expect("migration reapplies");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count");
        assert_eq!(count, 3);
    }

    #[test]
    fn initial_tables_exist() {
        let database = Database::open_memory().expect("in-memory database opens");
        let connection = database.connection.lock().expect("database lock");
        let settings_table: String = connection
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'settings'",
                [],
                |row| row.get(0),
            )
            .expect("settings table exists");
        assert_eq!(settings_table, "settings");
    }
}
