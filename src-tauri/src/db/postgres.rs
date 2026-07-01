//! WP-50 — PostgreSQL backend foundation.
//!
//! This module is a standalone PostgreSQL connector: it can test connectivity
//! and bootstrap a schema mirroring SteloPTC's core logical structure. It is
//! **not** wired into `AppState` or the live query/command layer — every
//! existing `#[tauri::command]` continues to read and write through
//! `rusqlite::Connection` exactly as before. Rewriting the ~5,800-line query
//! layer and 24 command modules to target either backend is a deliberately
//! separate, much larger future effort (see ROADMAP.md WP-50 "Not yet
//! implemented").
//!
//! The public functions below (`test_connection`, `bootstrap_schema`) have two
//! implementations selected by the `postgres` Cargo feature: a real one built
//! on `sqlx` when the feature is enabled, and a stub that returns a clear
//! error when it is not. Callers (Tauri commands) use one call site regardless
//! of how the binary was compiled.

/// PostgreSQL-flavored DDL for the five core tables, mirroring SteloPTC's
/// current SQLite logical structure. This is intentionally **not** a 1:1 port
/// of all 34 SQLite migrations — it establishes a clean starting schema for
/// the tables most central to multi-user deployments (specimens, subcultures,
/// audit_log, taxa, strains). Vocabulary tables, compliance, inventory, and
/// the remaining domain tables are deferred to the future full-migration WP.
pub const BOOTSTRAP_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS specimens (
    id                  UUID PRIMARY KEY,
    accession_number    TEXT NOT NULL UNIQUE,
    species_id          UUID NOT NULL,
    strain_id           UUID,
    stage               TEXT NOT NULL,
    health_status       SMALLINT NOT NULL DEFAULT 3,
    location            TEXT,
    parent_specimen_id  UUID,
    is_archived         BOOLEAN NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS subcultures (
    id              UUID PRIMARY KEY,
    specimen_id     UUID NOT NULL REFERENCES specimens(id) ON DELETE CASCADE,
    passage_number  INTEGER NOT NULL,
    event_type      TEXT NOT NULL DEFAULT 'passage',
    performed_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    notes           TEXT
);
CREATE TABLE IF NOT EXISTS audit_log (
    id          UUID PRIMARY KEY,
    lineage_id  TEXT NOT NULL,
    chain_seq   BIGINT NOT NULL,
    prev_hash   TEXT NOT NULL,
    entry_hash  TEXT NOT NULL,
    user_id     UUID,
    action      TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id   TEXT,
    details     TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (lineage_id, chain_seq)
);
CREATE TABLE IF NOT EXISTS taxa (
    id          UUID PRIMARY KEY,
    rank        TEXT NOT NULL,
    name        TEXT NOT NULL,
    parent_id   UUID REFERENCES taxa(id),
    status      TEXT NOT NULL DEFAULT 'accepted',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS strains (
    id          UUID PRIMARY KEY,
    species_id  UUID NOT NULL,
    name        TEXT NOT NULL,
    code        TEXT NOT NULL,
    strain_type TEXT NOT NULL DEFAULT 'cultivar',
    status      TEXT NOT NULL DEFAULT 'unverified',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#;

/// Splits a DDL batch into individual, non-empty statements. Used instead of
/// executing the whole batch as one string because sqlx's extended query
/// protocol (used by `sqlx::query`) does not support multiple statements per
/// call against PostgreSQL — each `CREATE TABLE` must be sent separately.
pub fn split_sql_statements(sql: &str) -> Vec<&str> {
    sql.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(feature = "postgres")]
mod live {
    use super::{split_sql_statements, BOOTSTRAP_SCHEMA_SQL};
    use crate::db::backend::validate_connection_string;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    /// Opens a short-lived connection pool, verifies it with `SELECT 1`, and
    /// closes it. Used by the Settings UI's "Test Connection" action.
    pub async fn test_connection(connection_string: &str) -> Result<String, String> {
        validate_connection_string(connection_string)?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(connection_string)
            .await
            .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

        let result = sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .map_err(|e| format!("Connection succeeded but the health-check query failed: {}", e));

        pool.close().await;
        result?;
        Ok("Connected successfully. The PostgreSQL server is reachable and responsive.".to_string())
    }

    /// Connects and creates the foundation schema (see `BOOTSTRAP_SCHEMA_SQL`).
    /// Returns the names of the tables created/verified. Safe to call repeatedly
    /// — every statement is `CREATE TABLE IF NOT EXISTS`.
    pub async fn bootstrap_schema(connection_string: &str) -> Result<Vec<String>, String> {
        validate_connection_string(connection_string)?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect(connection_string)
            .await
            .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

        for statement in split_sql_statements(BOOTSTRAP_SCHEMA_SQL) {
            if let Err(e) = sqlx::query(statement).execute(&pool).await {
                pool.close().await;
                return Err(format!("Failed to run bootstrap statement: {}", e));
            }
        }
        pool.close().await;

        Ok(vec![
            "specimens".to_string(),
            "subcultures".to_string(),
            "audit_log".to_string(),
            "taxa".to_string(),
            "strains".to_string(),
        ])
    }
}

#[cfg(not(feature = "postgres"))]
mod live {
    const NOT_COMPILED_MSG: &str =
        "This build was not compiled with PostgreSQL support. Rebuild with \
         `--features postgres` to enable PostgreSQL connectivity.";

    pub async fn test_connection(_connection_string: &str) -> Result<String, String> {
        Err(NOT_COMPILED_MSG.to_string())
    }

    pub async fn bootstrap_schema(_connection_string: &str) -> Result<Vec<String>, String> {
        Err(NOT_COMPILED_MSG.to_string())
    }
}

pub use live::{bootstrap_schema, test_connection};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_schema_sql_contains_all_core_tables() {
        for table in ["specimens", "subcultures", "audit_log", "taxa", "strains"] {
            assert!(
                BOOTSTRAP_SCHEMA_SQL.contains(&format!("TABLE IF NOT EXISTS {}", table)),
                "expected bootstrap schema to define table '{}'",
                table
            );
        }
    }

    #[test]
    fn split_sql_statements_returns_five_statements() {
        let statements = split_sql_statements(BOOTSTRAP_SCHEMA_SQL);
        assert_eq!(statements.len(), 5);
        for s in &statements {
            assert!(s.to_uppercase().starts_with("CREATE TABLE"));
        }
    }

    #[test]
    fn split_sql_statements_ignores_blank_segments() {
        let sql = "CREATE TABLE a (id INT);; \n  ;CREATE TABLE b (id INT);";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn split_sql_statements_empty_input_yields_no_statements() {
        assert!(split_sql_statements("").is_empty());
        assert!(split_sql_statements("   ;  ; ").is_empty());
    }

    #[cfg(not(feature = "postgres"))]
    #[tokio::test]
    async fn test_connection_without_feature_returns_clear_error() {
        let result = test_connection("postgres://localhost/db").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--features postgres"));
    }

    #[cfg(not(feature = "postgres"))]
    #[tokio::test]
    async fn bootstrap_schema_without_feature_returns_clear_error() {
        let result = bootstrap_schema("postgres://localhost/db").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--features postgres"));
    }
}
