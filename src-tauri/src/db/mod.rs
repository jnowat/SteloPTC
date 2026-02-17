pub mod migrations;
pub mod queries;

use rusqlite::Connection;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Database not found at {0}")]
    NotFound(String),
    #[error("Migration failed: {0}")]
    Migration(String),
    #[error("Constraint violation: {0}")]
    Constraint(String),
}

pub type DbResult<T> = Result<T, DbError>;

pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn new() -> DbResult<Self> {
        let db_path = Self::default_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let conn = Connection::open(&db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;")?;
        Ok(Database { conn })
    }

    pub fn new_in_memory() -> DbResult<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        Ok(Database { conn })
    }

    fn default_path() -> PathBuf {
        let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
        path.push("stelo_ptc.db");
        path
    }

    pub fn run_migrations(&self) -> DbResult<()> {
        migrations::run_all(&self.conn)
    }

    pub fn seed_defaults(&self) -> DbResult<()> {
        migrations::seed_defaults(&self.conn)
    }
}

fn dirs_next() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .ok()
            .map(|p| PathBuf::from(p).join("SteloPTC"))
    } else {
        std::env::var("HOME")
            .ok()
            .map(|p| PathBuf::from(p).join(".steloptc"))
    }
}
