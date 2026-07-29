use rusqlite::Connection;

/// Returns the active lab profile from app_config, defaulting to 'plant_tissue_culture'
/// if the row is missing or the query fails.
pub fn active_profile(conn: &Connection) -> String {
    conn.query_row(
        "SELECT lab_profile FROM app_config WHERE id = 1",
        [],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| "plant_tissue_culture".to_string())
}

/// Returns the biological domain for the active lab profile from app_config.
///
/// `active_profile` above is the **only** correct way to read the lab profile.
/// Reaching for `queries::read_setting(conn, "lab_profile", …)` looks equivalent
/// and is not: that reads the `app_settings` key-value table, and nothing has
/// ever written `lab_profile` into it. Such a call silently returns the default
/// on every database — which is how the WP-74 compliance gating spent a release
/// running plant-tissue-culture rules in mycology and cell-culture labs while
/// its own comment claimed the opposite.
///
/// Original doc comment follows.
///
/// Returns the biological domain for the active lab profile from app_config,
/// defaulting to 'Plantae' if the `domain` column is absent or the query fails.
///
/// Known values: 'Plantae', 'Animalia', 'Fungi'. Future domains (e.g. 'Bacteria',
/// 'Archaea') can be stored in the column without a schema migration (no CHECK
/// constraint is enforced at the DB level).
pub fn active_domain(conn: &Connection) -> String {
    conn.query_row(
        "SELECT domain FROM app_config WHERE id = 1",
        [],
        |r| r.get(0),
    )
    .unwrap_or_else(|_| "Plantae".to_string())
}

/// SQL predicate restricting a `specimens` query to the currently active lab.
///
/// `alias` is the table alias used in the query (`"s"`, `"sp"`, or the bare
/// table name). The returned fragment is a bare condition — the caller supplies
/// its own `AND`/`WHERE`.
///
/// Written as a correlated lookup rather than a bind parameter deliberately.
/// The queries that need it carry wildly different numbers of positional
/// parameters, and threading a new `?N` through each is exactly the kind of
/// index-arithmetic edit that silently binds the wrong value — the failure
/// would be a lab quietly seeing another lab's data, which is the thing this is
/// supposed to prevent. The COALESCE mirrors [`active_profile`], so a missing
/// `app_config` row degrades to the default lab here too, rather than comparing
/// against NULL and matching nothing.
///
/// This is defence in depth, not the primary mechanism: the specimen commands
/// scope their own reads and guard by-ID access through
/// [`require_active_lab_profile`]. This exists for the aggregate/report queries
/// that read `specimens` directly without going through those paths.
pub fn active_lab_sql(alias: &str) -> String {
    format!(
        "{alias}.lab_profile = COALESCE(\
             (SELECT lab_profile FROM app_config WHERE id = 1), 'plant_tissue_culture')"
    )
}

/// Returns the lab profile a specimen belongs to, or `Err` if it does not exist.
///
/// Reads the stamped `specimens.lab_profile` column — never re-derives it from
/// the stage code, which is ambiguous across profiles (see migration 053).
pub fn specimen_lab_profile(conn: &Connection, specimen_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT lab_profile FROM specimens WHERE id = ?1",
        rusqlite::params![specimen_id],
        |r| r.get(0),
    )
    .map_err(|_| "Specimen not found".to_string())
}

/// Guard: refuse to read or mutate a specimen that belongs to a different lab
/// than the one currently active.
///
/// This is the enforcement point that keeps a mycology lab, a plant tissue
/// culture lab and a cell culture lab from operating on each other's cultures.
/// Filtering reads by profile is not sufficient on its own: an ID obtained
/// under one profile (from a QR scan, a bookmark, a stale UI, or a crafted IPC
/// call) would otherwise still resolve after switching profiles. Every
/// by-ID specimen command routes through here, so the block is default-deny.
pub fn require_active_lab_profile(conn: &Connection, specimen_id: &str) -> Result<(), String> {
    let active = active_profile(conn);
    let owner = specimen_lab_profile(conn, specimen_id)?;
    if owner == active {
        Ok(())
    } else {
        Err(format!(
            "This specimen belongs to the {} lab, but the {} lab is currently active. \
             Switch the active lab profile in Settings to work with it.",
            owner, active
        ))
    }
}

/// Validates that `code` exists in the `stages` table for the given profile and is not
/// a terminal stage (is_terminal = 0). Returns false on any query error (table missing,
/// etc.) so unknown codes are always rejected.
pub fn stage_is_selectable(conn: &Connection, profile: &str, code: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM stages \
         WHERE profile = ?1 AND code = ?2 AND is_terminal = 0",
        rusqlite::params![profile, code],
        |r| r.get::<_, i64>(0),
    )
    .map(|c| c > 0)
    .unwrap_or(false)
}

/// Guard wrapper around [`stage_is_selectable`] returning a user-facing error string
/// when the stage is not valid/selectable for `profile`. Shared by `create_specimen`
/// and `bulk_update_stage` so both the New Specimen path and bulk updates enforce the
/// exact same profile-scoped rule (previously only bulk-update validated, letting a
/// stale cross-profile stage — e.g. an `explant` stage on a mycology specimen — be
/// written straight to the DB on create).
pub fn require_selectable_stage(conn: &Connection, profile: &str, code: &str) -> Result<(), String> {
    if stage_is_selectable(conn, profile, code) {
        Ok(())
    } else {
        Err(format!("'{}' is not a valid or selectable stage", code))
    }
}

#[cfg(test)]
mod profile_source_tests {
    use super::*;
    use rusqlite::Connection;

    fn db_with_both_tables() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_config (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 lab_profile TEXT NOT NULL DEFAULT 'plant_tissue_culture',
                 domain TEXT NOT NULL DEFAULT 'Plantae'
             );
             INSERT INTO app_config (id, lab_profile, domain) VALUES (1, 'mycology', 'Fungi');
             CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn active_profile_reads_the_table_the_profile_is_actually_stored_in() {
        let conn = db_with_both_tables();
        assert_eq!(active_profile(&conn), "mycology");
    }

    #[test]
    fn app_settings_never_carries_the_lab_profile() {
        // The regression this pins: `read_setting(conn, "lab_profile", …)` looks
        // like a profile lookup and is not — nothing writes that key, so it
        // returns the default on every database. Three callers did exactly that,
        // which ran plant-tissue-culture compliance rules in mycology and
        // cell-culture labs for a whole release. If a future change starts
        // writing the key, this test failing is the signal to revisit the
        // comment on `active_profile` rather than to delete the assertion.
        let conn = db_with_both_tables();
        let via_settings =
            crate::db::queries::read_setting(&conn, "lab_profile", "plant_tissue_culture");
        assert_eq!(
            via_settings, "plant_tissue_culture",
            "app_settings has no lab_profile key, so this can only ever return the default"
        );
        assert_ne!(
            via_settings,
            active_profile(&conn),
            "and that default silently disagrees with the real profile"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn db_with_stages() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_config (
                 id INTEGER PRIMARY KEY CHECK (id = 1),
                 lab_profile TEXT NOT NULL DEFAULT 'plant_tissue_culture',
                 domain TEXT NOT NULL DEFAULT 'Plantae'
             );
             INSERT INTO app_config (id, lab_profile, domain) VALUES (1, 'plant_tissue_culture', 'Plantae');

             CREATE TABLE stages (
                 id         INTEGER PRIMARY KEY AUTOINCREMENT,
                 profile    TEXT    NOT NULL,
                 code       TEXT    NOT NULL,
                 label      TEXT    NOT NULL,
                 sort_order INTEGER NOT NULL DEFAULT 0,
                 is_terminal INTEGER NOT NULL DEFAULT 0,
                 UNIQUE(profile, code)
             );
             INSERT INTO stages (profile, code, label, sort_order, is_terminal) VALUES
                 ('plant_tissue_culture', 'explant',  'Explant',  1, 0),
                 ('plant_tissue_culture', 'callus',   'Callus',   2, 0),
                 ('plant_tissue_culture', 'archived', 'Archived', 14, 1),
                 ('cell_culture',         'adherent', 'Adherent', 1, 0);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn active_profile_reads_from_app_config() {
        let conn = db_with_stages();
        assert_eq!(active_profile(&conn), "plant_tissue_culture");
    }

    #[test]
    fn active_profile_falls_back_when_table_missing() {
        let conn = Connection::open_in_memory().unwrap();
        assert_eq!(active_profile(&conn), "plant_tissue_culture");
    }

    #[test]
    fn active_domain_reads_plantae_for_ptc() {
        let conn = db_with_stages();
        assert_eq!(active_domain(&conn), "Plantae");
    }

    #[test]
    fn active_domain_reads_animalia_when_set() {
        let conn = db_with_stages();
        conn.execute("UPDATE app_config SET domain = 'Animalia' WHERE id = 1", [])
            .unwrap();
        assert_eq!(active_domain(&conn), "Animalia");
    }

    #[test]
    fn active_domain_reads_fungi_when_set() {
        let conn = db_with_stages();
        conn.execute("UPDATE app_config SET domain = 'Fungi' WHERE id = 1", [])
            .unwrap();
        assert_eq!(active_domain(&conn), "Fungi");
    }

    #[test]
    fn active_domain_falls_back_to_plantae_when_column_missing() {
        let conn = Connection::open_in_memory().unwrap();
        // No app_config table at all — must not panic.
        assert_eq!(active_domain(&conn), "Plantae");
    }

    #[test]
    fn stage_is_selectable_accepts_valid_non_terminal() {
        let conn = db_with_stages();
        assert!(stage_is_selectable(&conn, "plant_tissue_culture", "explant"));
        assert!(stage_is_selectable(&conn, "plant_tissue_culture", "callus"));
    }

    #[test]
    fn stage_is_selectable_rejects_terminal_stage() {
        let conn = db_with_stages();
        assert!(!stage_is_selectable(&conn, "plant_tissue_culture", "archived"),
            "'archived' is is_terminal=1 and must not be selectable");
    }

    #[test]
    fn stage_is_selectable_rejects_unknown_code() {
        let conn = db_with_stages();
        assert!(!stage_is_selectable(&conn, "plant_tissue_culture", "totally_fake"));
    }

    #[test]
    fn stage_is_selectable_rejects_cross_profile_code() {
        let conn = db_with_stages();
        // 'adherent' is valid in cell_culture but must not be selectable under plant_tissue_culture.
        assert!(!stage_is_selectable(&conn, "plant_tissue_culture", "adherent"));
    }

    #[test]
    fn stage_is_selectable_returns_false_when_stages_table_missing() {
        let conn = Connection::open_in_memory().unwrap();
        // No stages table — should not panic, just return false.
        assert!(!stage_is_selectable(&conn, "plant_tissue_culture", "explant"));
    }

    #[test]
    fn require_selectable_stage_ok_for_valid_non_terminal() {
        let conn = db_with_stages();
        assert!(require_selectable_stage(&conn, "plant_tissue_culture", "explant").is_ok());
    }

    #[test]
    fn require_selectable_stage_errors_for_cross_profile_code() {
        let conn = db_with_stages();
        // 'adherent' belongs to cell_culture; rejecting it under plant_tissue_culture is
        // exactly the create-path hole this guard closes.
        let err = require_selectable_stage(&conn, "plant_tissue_culture", "adherent").unwrap_err();
        assert!(err.contains("adherent"), "error should name the rejected code: {err}");
    }

    #[test]
    fn require_selectable_stage_errors_for_terminal_stage() {
        let conn = db_with_stages();
        assert!(require_selectable_stage(&conn, "plant_tissue_culture", "archived").is_err());
    }

    #[test]
    fn unique_constraint_rejects_duplicate_code_same_profile() {
        let conn = db_with_stages();
        let result = conn.execute(
            "INSERT INTO stages (profile, code, label, sort_order) \
             VALUES ('plant_tissue_culture', 'explant', 'Duplicate', 99)",
            [],
        );
        assert!(result.is_err(), "UNIQUE(profile, code) must reject a duplicate insert");
    }

    #[test]
    fn unique_constraint_allows_same_code_in_different_profile() {
        let conn = db_with_stages();
        // 'explant' exists in plant_tissue_culture; adding it to cell_culture should succeed.
        let result = conn.execute(
            "INSERT INTO stages (profile, code, label, sort_order) \
             VALUES ('cell_culture', 'explant', 'Explant', 1)",
            [],
        );
        assert!(result.is_ok());
    }
}
