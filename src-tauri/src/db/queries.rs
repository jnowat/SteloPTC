// Query helpers and shared database utilities
use rusqlite::{Connection, params};
use sha2::{Sha256, Digest};
use super::DbResult;

/// Zero-hash used as prev_hash when a lineage has no prior entry.
pub const ZERO_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Canonical serialization for an audit entry used in hash computation.
///
/// Format — pipe-separated UTF-8, no trailing newline, fixed field order:
///   lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
///
/// NULL optional fields serialize as empty string ("").
/// Never reorder fields; append new fields at the end only so that existing
/// stored hashes remain verifiable.
pub fn audit_canonical_bytes(
    lineage_id: &str,
    chain_seq: i64,
    timestamp: &str,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
    action: &str,
    details: &str,
) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        lineage_id, chain_seq, timestamp, user_id, entity_type, entity_id, action, details
    )
    .into_bytes()
}

/// SHA-256(canonical_bytes || prev_hash_utf8), returned as lowercase hex.
pub fn compute_entry_hash(canonical: &[u8], prev_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    hasher.update(prev_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a new accession number in format YYYY-MM-DD-SPECIESCODE-SEQ
pub fn generate_accession_number(conn: &Connection, species_code: &str, date: &str) -> DbResult<String> {
    let prefix = format!("{}-{}", date, species_code);
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM specimens WHERE accession_number LIKE ?1",
        params![format!("{}-%", prefix)],
        |r| r.get(0),
    )?;
    let seq = count + 1;
    Ok(format!("{}-{:03}", prefix, seq))
}

/// Log an audit entry that continues the entity's own lineage chain.
/// The lineage_id is entity_id (or "system" when entity_id is None).
/// All existing call sites use this function without change.
pub fn log_audit(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
) -> DbResult<()> {
    log_audit_impl(conn, user_id, action, entity_type, entity_id, old_value, new_value, details, None)
}

/// Log an audit entry for a new entity that was split or derived from parent_lineage_id.
///
/// The new entity starts its own lineage (chain_seq = 1) but inherits prev_hash
/// from the parent lineage's last entry, creating a cryptographically visible fork.
/// Both siblings of a split share the same prev_hash, which is the intended behaviour:
/// it records "these two chains both originate from the same parent state."
pub fn log_audit_for_child(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
    parent_lineage_id: &str,
) -> DbResult<()> {
    log_audit_impl(conn, user_id, action, entity_type, entity_id, old_value, new_value, details, Some(parent_lineage_id))
}

fn log_audit_impl(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
    parent_lineage_id: Option<&str>,
) -> DbResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let lineage_id = entity_id.unwrap_or("system").to_string();
    let timestamp = chrono::Utc::now()
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string();

    // Determine chain position within the lineage.
    //
    // Fork case (parent_lineage_id given): the new lineage starts at seq 1 and
    // inherits the parent's last entry_hash as its prev_hash. This makes the
    // split cryptographically visible — both children share the same prev_hash.
    //
    // Continuation case: look up the highest chain_seq in THIS lineage and
    // increment it; use that row's entry_hash as prev_hash.
    let (next_seq, prev_hash): (i64, String) = if let Some(plid) = parent_lineage_id {
        // Prefer matching by lineage_id (set by the new per-lineage code).
        // Fall back to entity_id for rows written before migration_009 where
        // lineage_id may be NULL even though the entity_id is correct.
        // The entry_hash IS NOT NULL guard means pre-WP-18 entries (no hash)
        // are always excluded — in that case we anchor with ZERO_HASH.
        let parent_hash: Option<String> = conn.query_row(
            "SELECT entry_hash FROM audit_log \
             WHERE (lineage_id = ?1 OR (lineage_id IS NULL AND entity_id = ?1)) \
               AND entry_hash IS NOT NULL \
             ORDER BY chain_seq DESC LIMIT 1",
            params![plid],
            |row| row.get(0),
        ).ok().flatten();
        (1, parent_hash.unwrap_or_else(|| ZERO_HASH.to_string()))
    } else {
        // Continue the lineage's own chain.
        // Same dual-lookup: prefer lineage_id, fall back to entity_id.
        let head: Option<(i64, String)> = conn.query_row(
            "SELECT chain_seq, entry_hash FROM audit_log \
             WHERE (lineage_id = ?1 OR (lineage_id IS NULL AND entity_id = ?1)) \
               AND entry_hash IS NOT NULL \
             ORDER BY chain_seq DESC LIMIT 1",
            params![lineage_id],
            |row| Ok((row.get::<_, i64>(0)? + 1, row.get::<_, String>(1)?)),
        ).ok();
        head.unwrap_or((1, ZERO_HASH.to_string()))
    };

    let canonical = audit_canonical_bytes(
        &lineage_id,
        next_seq,
        &timestamp,
        user_id.unwrap_or(""),
        entity_type,
        entity_id.unwrap_or(""),
        action,
        details.unwrap_or(""),
    );
    let entry_hash = compute_entry_hash(&canonical, &prev_hash);

    conn.execute(
        "INSERT INTO audit_log \
         (id, user_id, action, entity_type, entity_id, old_value, new_value, details, created_at, \
          lineage_id, chain_seq, prev_hash, entry_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id, user_id, action, entity_type, entity_id,
            old_value, new_value, details, timestamp,
            lineage_id, next_seq, prev_hash, entry_hash
        ],
    )?;
    Ok(())
}

/// Paginated query helper
pub struct PaginationParams {
    pub page: u32,
    pub per_page: u32,
}

impl PaginationParams {
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.per_page
    }

    pub fn limit(&self) -> u32 {
        self.per_page
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self { page: 1, per_page: 50 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn mem_conn_with_specimens() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL UNIQUE,
                is_archived INTEGER NOT NULL DEFAULT 0
            );",
        )
        .expect("create specimens table");
        conn
    }

    fn mem_conn_with_audit() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE audit_log (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                action TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT,
                old_value TEXT,
                new_value TEXT,
                details TEXT,
                created_at TEXT NOT NULL,
                lineage_id TEXT,
                chain_seq INTEGER,
                prev_hash TEXT,
                entry_hash TEXT
            );
            CREATE INDEX idx_audit_lineage ON audit_log(lineage_id, chain_seq);",
        )
        .expect("create audit_log table");
        conn
    }

    #[test]
    fn accession_first_specimen_gets_seq_001() {
        let conn = mem_conn_with_specimens();
        let acc = generate_accession_number(&conn, "CIT-01", "2026-06-13").unwrap();
        assert_eq!(acc, "2026-06-13-CIT-01-001");
    }

    #[test]
    fn accession_second_specimen_gets_seq_002() {
        let conn = mem_conn_with_specimens();
        conn.execute(
            "INSERT INTO specimens (id, accession_number) VALUES ('a', '2026-06-13-CIT-01-001')",
            [],
        )
        .unwrap();
        let acc = generate_accession_number(&conn, "CIT-01", "2026-06-13").unwrap();
        assert_eq!(acc, "2026-06-13-CIT-01-002");
    }

    #[test]
    fn accession_different_species_resets_seq() {
        let conn = mem_conn_with_specimens();
        conn.execute(
            "INSERT INTO specimens (id, accession_number) VALUES ('a', '2026-06-13-CIT-01-001')",
            [],
        )
        .unwrap();
        let acc = generate_accession_number(&conn, "VAC-02", "2026-06-13").unwrap();
        assert_eq!(acc, "2026-06-13-VAC-02-001");
    }

    #[test]
    fn accession_different_date_resets_seq() {
        let conn = mem_conn_with_specimens();
        conn.execute(
            "INSERT INTO specimens (id, accession_number) VALUES ('a', '2026-06-13-CIT-01-001')",
            [],
        )
        .unwrap();
        let acc = generate_accession_number(&conn, "CIT-01", "2026-06-14").unwrap();
        assert_eq!(acc, "2026-06-14-CIT-01-001");
    }

    #[test]
    fn accession_format_has_three_digit_seq() {
        let conn = mem_conn_with_specimens();
        for i in 1..=9 {
            conn.execute(
                &format!(
                    "INSERT INTO specimens (id, accession_number) VALUES ('id{i}', '2026-01-01-SP-00{i}')"
                ),
                [],
            )
            .unwrap();
        }
        let acc = generate_accession_number(&conn, "SP", "2026-01-01").unwrap();
        assert_eq!(acc, "2026-01-01-SP-010");
    }

    #[test]
    fn pagination_offset_first_page() {
        let pg = PaginationParams { page: 1, per_page: 50 };
        assert_eq!(pg.offset(), 0);
        assert_eq!(pg.limit(), 50);
    }

    #[test]
    fn pagination_offset_second_page() {
        let pg = PaginationParams { page: 2, per_page: 25 };
        assert_eq!(pg.offset(), 25);
    }

    #[test]
    fn pagination_offset_does_not_underflow() {
        let pg = PaginationParams { page: 0, per_page: 10 };
        assert_eq!(pg.offset(), 0);
    }

    #[test]
    fn audit_chain_seq_increments_per_lineage() {
        let conn = mem_conn_with_audit();
        log_audit(&conn, Some("u1"), "create", "specimen", Some("sp-A"), None, None, Some("first")).unwrap();
        log_audit(&conn, Some("u1"), "update", "specimen", Some("sp-A"), None, None, Some("second")).unwrap();

        let seqs: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT chain_seq FROM audit_log WHERE lineage_id = 'sp-A' ORDER BY chain_seq"
            ).unwrap();
            stmt.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        assert_eq!(seqs, vec![1, 2]);
    }

    #[test]
    fn audit_child_lineage_starts_at_1_with_parent_prev_hash() {
        let conn = mem_conn_with_audit();
        log_audit(&conn, Some("u1"), "create", "specimen", Some("sp-A"), None, None, None).unwrap();

        let parent_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'sp-A' ORDER BY chain_seq DESC LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        log_audit_for_child(&conn, Some("u1"), "create", "specimen", Some("sp-B"), None, None, None, "sp-A").unwrap();

        let (child_seq, child_prev): (i64, String) = conn.query_row(
            "SELECT chain_seq, prev_hash FROM audit_log WHERE lineage_id = 'sp-B' LIMIT 1",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();

        assert_eq!(child_seq, 1);
        assert_eq!(child_prev, parent_hash, "child's prev_hash must equal parent's last entry_hash");
    }

    #[test]
    fn audit_split_siblings_share_same_prev_hash() {
        let conn = mem_conn_with_audit();
        log_audit(&conn, None, "create", "specimen", Some("sp-A"), None, None, None).unwrap();

        log_audit_for_child(&conn, None, "create", "specimen", Some("sp-B"), None, None, None, "sp-A").unwrap();
        log_audit_for_child(&conn, None, "create", "specimen", Some("sp-C"), None, None, None, "sp-A").unwrap();

        let b_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'sp-B'", [], |r| r.get(0),
        ).unwrap();
        let c_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'sp-C'", [], |r| r.get(0),
        ).unwrap();

        assert_eq!(b_prev, c_prev, "both split children must share the same prev_hash");
    }

    #[test]
    fn audit_entry_hash_is_deterministic() {
        let canonical = audit_canonical_bytes("sp-A", 1, "2026-01-01T00:00:00.000Z", "u1", "specimen", "sp-A", "create", "");
        let h1 = compute_entry_hash(&canonical, ZERO_HASH);
        let h2 = compute_entry_hash(&canonical, ZERO_HASH);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    /// Simulate the verify_audit_lineage loop for a fork lineage and confirm it
    /// passes when initialized from the first row's prev_hash (not ZERO_HASH).
    /// This is a regression test for the bug where fork lineages always reported
    /// "Chain broken at seq 1" because the loop was anchored to ZERO_HASH.
    #[test]
    fn verify_logic_passes_for_fork_lineage() {
        let conn = mem_conn_with_audit();
        // Create parent
        log_audit(&conn, None, "create", "specimen", Some("sp-A"), None, None, None).unwrap();
        let parent_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'sp-A'", [], |r| r.get(0),
        ).unwrap();

        // Create child forked from parent
        log_audit_for_child(&conn, None, "create", "specimen", Some("sp-B"), None, None, None, "sp-A").unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-B"), None, None, None).unwrap();

        // Fetch child's rows as verify_audit_lineage would
        struct Row { chain_seq: i64, prev_hash: String, entry_hash: String, lineage_id: String,
                     created_at: String, action: String, entity_id: Option<String>,
                     entity_type: String, user_id: Option<String>, details: Option<String> }
        let mut stmt = conn.prepare(
            "SELECT chain_seq, prev_hash, entry_hash, lineage_id, created_at, action, entity_id, entity_type, user_id, details \
             FROM audit_log WHERE lineage_id = 'sp-B' AND entry_hash IS NOT NULL ORDER BY chain_seq ASC"
        ).unwrap();
        let rows: Vec<Row> = stmt.query_map([], |r| Ok(Row {
            chain_seq: r.get(0)?, prev_hash: r.get(1)?, entry_hash: r.get(2)?,
            lineage_id: r.get(3)?, created_at: r.get(4)?, action: r.get(5)?,
            entity_id: r.get(6)?, entity_type: r.get(7)?, user_id: r.get(8)?,
            details: r.get(9)?,
        })).unwrap().filter_map(|r| r.ok()).collect();

        assert_eq!(rows.len(), 2, "expected 2 rows for sp-B");

        // seq=1 should have prev_hash == parent's entry_hash
        assert_eq!(rows[0].chain_seq, 1);
        assert_eq!(rows[0].prev_hash, parent_hash, "fork child's seq=1 must point to parent");

        // Simulate the fixed verify loop (anchor = rows[0].prev_hash)
        let mut prev_hash = rows[0].prev_hash.clone();
        let mut broken_at: Option<i64> = None;
        for row in &rows {
            if row.prev_hash != prev_hash { broken_at = Some(row.chain_seq); break; }
            let canonical = audit_canonical_bytes(&row.lineage_id, row.chain_seq, &row.created_at,
                row.user_id.as_deref().unwrap_or(""), &row.entity_type,
                row.entity_id.as_deref().unwrap_or(""), &row.action, row.details.as_deref().unwrap_or(""));
            let computed = compute_entry_hash(&canonical, &row.prev_hash);
            if computed != row.entry_hash { broken_at = Some(row.chain_seq); break; }
            prev_hash = row.entry_hash.clone();
        }
        assert!(broken_at.is_none(), "fork lineage must verify cleanly; broke at seq {:?}", broken_at);
    }
}
