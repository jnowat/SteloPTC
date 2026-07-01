//! WP-50 — Backend selection foundation.
//!
//! SQLite remains the only backend actually wired into `AppState`/the query
//! layer. This module tracks the lab's *intended* backend (persisted in
//! `app_settings.backend_type`) and validates whether a switch is currently
//! possible, without performing any live reconnection. A full dual-backend
//! query layer (rewriting `db::queries` and every command to target either
//! SQLite or PostgreSQL) is explicitly out of scope for this packet — see
//! ROADMAP.md WP-50 for the deferred-work list.

use rusqlite::Connection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Sqlite,
    Postgres,
}

impl BackendKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackendKind::Sqlite => "sqlite",
            BackendKind::Postgres => "postgres",
        }
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "sqlite" => Ok(BackendKind::Sqlite),
            "postgres" => Ok(BackendKind::Postgres),
            other => Err(format!(
                "Unknown backend type '{}'. Allowed values: sqlite, postgres",
                other
            )),
        }
    }
}

/// Reads the currently configured backend type from `app_settings`.
/// Falls back to `Sqlite` on any error (missing table, missing row, or an
/// unparseable value) so a corrupted setting can never brick the app.
pub fn current_backend_kind(conn: &Connection) -> BackendKind {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = 'backend_type'",
        [],
        |r| r.get::<_, String>(0),
    )
    .ok()
    .and_then(|v| BackendKind::parse(&v).ok())
    .unwrap_or(BackendKind::Sqlite)
}

/// Persists the intended backend type. This does not reconnect or change
/// which backend is actually serving live queries in this packet — see the
/// module doc comment.
pub fn set_backend_kind(conn: &Connection, kind: BackendKind) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE app_settings SET value = ?1, updated_at = datetime('now') WHERE key = 'backend_type'",
        rusqlite::params![kind.as_str()],
    )?;
    Ok(())
}

/// Validates a PostgreSQL connection string well-formedly enough to catch
/// obvious mistakes before attempting a network round-trip. Does not verify
/// reachability — that is `db::postgres::test_connection`'s job.
pub fn validate_connection_string(s: &str) -> Result<(), String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("A PostgreSQL connection string is required.".to_string());
    }
    if !(trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://")) {
        return Err(
            "Connection string must start with 'postgres://' or 'postgresql://'.".to_string(),
        );
    }
    Ok(())
}

/// Pure validation for a proposed backend switch. Takes `postgres_feature_enabled`
/// as an explicit parameter (rather than reading `cfg!` internally) so both
/// branches are exercised by unit tests regardless of how the test binary itself
/// was compiled.
pub fn validate_backend_switch(
    target: BackendKind,
    postgres_feature_enabled: bool,
    connection_string: Option<&str>,
) -> Result<(), String> {
    match target {
        BackendKind::Sqlite => Ok(()),
        BackendKind::Postgres => {
            if !postgres_feature_enabled {
                return Err(
                    "This build was not compiled with PostgreSQL support. Rebuild with \
                     `--features postgres` to enable the PostgreSQL backend."
                        .to_string(),
                );
            }
            match connection_string {
                Some(cs) => validate_connection_string(cs),
                None => Err(
                    "A PostgreSQL connection string is required to switch to the \
                     PostgreSQL backend."
                        .to_string(),
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn migrated_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
        conn
    }

    #[test]
    fn backend_kind_round_trips_through_str() {
        assert_eq!(BackendKind::parse("sqlite").unwrap(), BackendKind::Sqlite);
        assert_eq!(BackendKind::parse("postgres").unwrap(), BackendKind::Postgres);
        assert_eq!(BackendKind::Sqlite.as_str(), "sqlite");
        assert_eq!(BackendKind::Postgres.as_str(), "postgres");
    }

    #[test]
    fn backend_kind_parse_rejects_unknown() {
        assert!(BackendKind::parse("mysql").is_err());
        assert!(BackendKind::parse("").is_err());
    }

    #[test]
    fn current_backend_kind_defaults_to_sqlite_on_fresh_db() {
        let conn = migrated_db();
        assert_eq!(current_backend_kind(&conn), BackendKind::Sqlite);
    }

    #[test]
    fn set_backend_kind_persists_and_reads_back() {
        let conn = migrated_db();
        set_backend_kind(&conn, BackendKind::Postgres).unwrap();
        assert_eq!(current_backend_kind(&conn), BackendKind::Postgres);
        set_backend_kind(&conn, BackendKind::Sqlite).unwrap();
        assert_eq!(current_backend_kind(&conn), BackendKind::Sqlite);
    }

    #[test]
    fn current_backend_kind_falls_back_to_sqlite_when_settings_table_missing() {
        let conn = Connection::open_in_memory().unwrap();
        // No migrations run — app_settings does not exist yet.
        assert_eq!(current_backend_kind(&conn), BackendKind::Sqlite);
    }

    #[test]
    fn validate_connection_string_accepts_postgres_scheme() {
        assert!(validate_connection_string("postgres://user:pass@host:5432/db").is_ok());
    }

    #[test]
    fn validate_connection_string_accepts_postgresql_scheme() {
        assert!(validate_connection_string("postgresql://user:pass@host/db").is_ok());
    }

    #[test]
    fn validate_connection_string_rejects_empty() {
        assert!(validate_connection_string("").is_err());
        assert!(validate_connection_string("   ").is_err());
    }

    #[test]
    fn validate_connection_string_rejects_wrong_scheme() {
        assert!(validate_connection_string("mysql://host/db").is_err());
        assert!(validate_connection_string("not a connection string").is_err());
    }

    #[test]
    fn validate_backend_switch_to_sqlite_always_ok() {
        assert!(validate_backend_switch(BackendKind::Sqlite, false, None).is_ok());
        assert!(validate_backend_switch(BackendKind::Sqlite, true, Some("garbage")).is_ok());
    }

    #[test]
    fn validate_backend_switch_to_postgres_without_feature_rejected() {
        let result = validate_backend_switch(
            BackendKind::Postgres,
            false,
            Some("postgres://host/db"),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--features postgres"));
    }

    #[test]
    fn validate_backend_switch_to_postgres_without_connection_string_rejected() {
        let result = validate_backend_switch(BackendKind::Postgres, true, None);
        assert!(result.is_err());
    }

    #[test]
    fn validate_backend_switch_to_postgres_with_invalid_connection_string_rejected() {
        let result = validate_backend_switch(BackendKind::Postgres, true, Some("not-a-url"));
        assert!(result.is_err());
    }

    #[test]
    fn validate_backend_switch_to_postgres_with_valid_setup_ok() {
        let result = validate_backend_switch(
            BackendKind::Postgres,
            true,
            Some("postgres://user:pass@localhost:5432/steloptc"),
        );
        assert!(result.is_ok());
    }
}
