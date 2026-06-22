// Query helpers and shared database utilities
use rusqlite::{Connection, params};
use sha2::{Sha256, Digest};
use super::{DbError, DbResult};

/// Zero-hash used as prev_hash when a lineage has no prior entry.
pub const ZERO_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Read a key from the `app_settings` table, returning `default` when the key
/// is absent or the table doesn't exist yet (e.g. before migration 014).
pub fn read_setting(conn: &Connection, key: &str, default: &str) -> String {
    conn.query_row(
        "SELECT value FROM app_settings WHERE key = ?1",
        params![key],
        |r| r.get::<_, String>(0),
    )
    .unwrap_or_else(|_| default.to_string())
}

/// Audit fields shared across all `log_audit*` public wrappers.
/// Passed as a single argument to `log_audit_impl` to stay within Clippy's
/// `too_many_arguments` limit without changing the public-facing API.
struct AuditEntry<'a> {
    user_id: Option<&'a str>,
    action: &'a str,
    entity_type: &'a str,
    entity_id: Option<&'a str>,
    old_value: Option<&'a str>,
    new_value: Option<&'a str>,
    details: Option<&'a str>,
}

/// Canonical serialization for an audit entry used in hash computation.
///
/// Format — pipe-separated UTF-8, no trailing newline, fixed field order:
///   lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
///
/// NULL optional fields serialize as empty string ("").
/// Never reorder fields; append new fields at the end only so that existing
/// stored hashes remain verifiable.
#[allow(clippy::too_many_arguments)]
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

/// Build a binary Merkle tree over SHA-256 hex `leaves` and return the root.
///
/// **Locked construction rule (must never change — alters every prior proof):**
/// For odd node counts at any level, duplicate the last node before pairing.
/// This matches Bitcoin's Merkle construction and must be reproduced identically
/// by any external verifier. See docs/merkle-checkpoints.md for the full spec.
///
/// Edge cases:
///   - Empty slice  → ZERO_HASH (64 zeroes)
///   - Single leaf  → that leaf itself (returned as-is, no extra hash round)
///   - Two or more  → pairs hashed as SHA-256(left_hex_bytes || right_hex_bytes)
pub fn build_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return ZERO_HASH.to_string();
    }
    let mut level: Vec<String> = leaves.to_vec();
    while level.len() > 1 {
        if !level.len().is_multiple_of(2) {
            let last = level.last().unwrap().clone();
            level.push(last);
        }
        let mut next = Vec::with_capacity(level.len() / 2);
        let mut i = 0;
        while i < level.len() {
            let mut hasher = Sha256::new();
            hasher.update(level[i].as_bytes());
            hasher.update(level[i + 1].as_bytes());
            next.push(format!("{:x}", hasher.finalize()));
            i += 2;
        }
        level = next;
    }
    level.remove(0)
}

/// A node in a Merkle proof path (internal type; not serialised here).
///
/// `position` describes where the sibling sits relative to the current node:
///   "right" → SHA256(current || sibling)
///   "left"  → SHA256(sibling || current)
pub struct PathNode {
    pub sibling_hash: String,
    pub position: String,
}

/// Compute the Merkle inclusion path for the leaf at `leaf_index`.
///
/// Uses the identical "duplicate-last" padding rule as `build_merkle_root`
/// so paths verify correctly against roots produced by that function.
/// Returns an empty Vec for trees with 0 or 1 leaf (root == leaf, no path needed).
///
/// To verify: start with `current = leaves[leaf_index]`, walk `path`, applying
///   SHA256(current || sibling)  when position == "right"
///   SHA256(sibling || current)  when position == "left"
/// and compare the final value to the stored `merkle_root`.
pub fn build_merkle_path(leaves: &[String], leaf_index: usize) -> Vec<PathNode> {
    if leaves.len() <= 1 || leaf_index >= leaves.len() {
        return vec![];
    }

    let mut path = Vec::new();
    let mut level: Vec<String> = leaves.to_vec();
    let mut idx = leaf_index;

    while level.len() > 1 {
        let padded: Vec<String> = if !level.len().is_multiple_of(2) {
            let mut v = level.clone();
            v.push(v.last().unwrap().clone());
            v
        } else {
            level.clone()
        };

        let sibling_idx = if idx.is_multiple_of(2) { idx + 1 } else { idx - 1 };
        path.push(PathNode {
            sibling_hash: padded[sibling_idx].clone(),
            position: if idx.is_multiple_of(2) { "right".to_string() } else { "left".to_string() },
        });

        let mut next = Vec::with_capacity(padded.len() / 2);
        let mut i = 0;
        while i < padded.len() {
            let mut hasher = Sha256::new();
            hasher.update(padded[i].as_bytes());
            hasher.update(padded[i + 1].as_bytes());
            next.push(format!("{:x}", hasher.finalize()));
            i += 2;
        }

        idx /= 2;
        level = next;
    }

    path
}

/// Verify a Merkle inclusion path, returning true when the recomputed root matches.
///
/// For a single-leaf tree (empty path), `leaf_hash` must equal `expected_root`.
pub fn verify_merkle_path(leaf_hash: &str, path: &[PathNode], expected_root: &str) -> bool {
    if path.is_empty() {
        return leaf_hash == expected_root;
    }
    let mut current = leaf_hash.to_string();
    for node in path {
        let mut hasher = Sha256::new();
        if node.position == "right" {
            hasher.update(current.as_bytes());
            hasher.update(node.sibling_hash.as_bytes());
        } else {
            hasher.update(node.sibling_hash.as_bytes());
            hasher.update(current.as_bytes());
        }
        current = format!("{:x}", hasher.finalize());
    }
    current == expected_root
}

/// Create auto-checkpoints for lineages that have enough uncovered entries.
///
/// A lineage is eligible when the number of entries beyond its latest checkpoint
/// is >= `min_uncovered` (or any count when `min_uncovered == 0`).
///
/// Returns the list of newly created checkpoint IDs so callers can log or surface them.
pub fn auto_checkpoint_lineages(
    conn: &Connection,
    user_id: &str,
    auto_source: &str,
    min_uncovered: i64,
) -> DbResult<Vec<String>> {
    let mut lineage_stmt = conn.prepare(
        "SELECT DISTINCT lineage_id FROM audit_log \
         WHERE entry_hash IS NOT NULL AND lineage_id IS NOT NULL",
    )?;
    let lineages: Vec<String> = lineage_stmt
        .query_map([], |r| r.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let mut created_ids = Vec::new();

    for lineage_id in &lineages {
        // Use -1 as the sentinel "no prior checkpoint" value so that seq=0
        // entries (written by log_audit_at_seq_zero for species births) are
        // included in the first auto-checkpoint for those lineages.
        let last_end_seq: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(end_seq), -1) FROM audit_checkpoints WHERE lineage_id = ?1",
                params![lineage_id],
                |r| r.get(0),
            )
            .unwrap_or(-1);

        let max_seq: Option<i64> = conn
            .query_row(
                "SELECT MAX(chain_seq) FROM audit_log \
                 WHERE lineage_id = ?1 AND entry_hash IS NOT NULL",
                params![lineage_id],
                |r| r.get(0),
            )
            .unwrap_or(None);

        let max_seq = match max_seq {
            Some(s) if s > last_end_seq => s,
            _ => continue,
        };

        let uncovered: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM audit_log \
                 WHERE lineage_id = ?1 AND chain_seq > ?2 AND entry_hash IS NOT NULL",
                params![lineage_id, last_end_seq],
                |r| r.get(0),
            )
            .unwrap_or(0);

        if min_uncovered > 0 && uncovered < min_uncovered {
            continue;
        }

        // Use the actual minimum uncovered chain_seq as start_seq so the
        // checkpoint accurately reflects its true coverage, including seq=0
        // for species lineages that use log_audit_at_seq_zero.
        let start_seq: i64 = conn
            .query_row(
                "SELECT MIN(chain_seq) FROM audit_log \
                 WHERE lineage_id = ?1 AND chain_seq > ?2 AND entry_hash IS NOT NULL",
                params![lineage_id, last_end_seq],
                |r| r.get::<_, Option<i64>>(0),
            )
            .unwrap_or(None)
            .unwrap_or(last_end_seq + 1);

        let mut stmt = conn.prepare(
            "SELECT entry_hash FROM audit_log \
             WHERE lineage_id = ?1 AND chain_seq >= ?2 AND chain_seq <= ?3 \
             AND entry_hash IS NOT NULL ORDER BY chain_seq ASC",
        )?;
        let hashes: Vec<String> = stmt
            .query_map(params![lineage_id, start_seq, max_seq], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        if hashes.is_empty() {
            continue;
        }

        let entry_count = hashes.len() as i64;
        let merkle_root = build_merkle_root(&hashes);
        let checkpoint_id = uuid::Uuid::new_v4().to_string();
        let created_at = chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string();

        conn.execute(
            "INSERT INTO audit_checkpoints \
             (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, \
              created_at, created_by, is_auto, auto_source) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9)",
            params![
                &checkpoint_id, lineage_id, start_seq, max_seq,
                entry_count, &merkle_root, &created_at, user_id, auto_source
            ],
        )?;

        created_ids.push(checkpoint_id);
    }

    Ok(created_ids)
}

/// SHA-256(canonical_bytes || prev_hash_utf8), returned as lowercase hex.
pub fn compute_entry_hash(canonical: &[u8], prev_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical);
    hasher.update(prev_hash.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate N letter-suffixed accession numbers for a split operation.
///
/// Each child gets the parent's accession number with an appended letter (A–Z).
/// Any candidate that already exists in the database is skipped.
///
/// Example:
///   parent="2026-06-13-CIT-SIN-001", count=3 → ["…-001A", "…-001B", "…-001C"]
///   parent="2026-06-13-CIT-SIN-001B", count=2 → ["…-001BA", "…-001BB"]
///
/// Returns `DbError::Constraint` if all 26 letter slots are exhausted.
pub fn generate_split_accession_numbers(
    conn: &Connection,
    parent_accession: &str,
    count: usize,
) -> DbResult<Vec<String>> {
    let mut results = Vec::with_capacity(count);
    let mut letter_idx: u8 = 0;

    while results.len() < count {
        if letter_idx > 25 {
            return Err(DbError::Constraint(format!(
                "Cannot generate {} split accession numbers from '{}': all 26 letter suffixes (A–Z) are already taken",
                count, parent_accession
            )));
        }
        let letter = char::from(b'A' + letter_idx);
        let candidate = format!("{}{}", parent_accession, letter);
        letter_idx += 1;

        let taken: bool = conn.query_row(
            "SELECT COUNT(*) FROM specimens WHERE accession_number = ?1",
            params![&candidate],
            |r| r.get::<_, i64>(0),
        ).map(|c| c > 0)?;

        if !taken {
            results.push(candidate);
        }
    }

    Ok(results)
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
#[allow(clippy::too_many_arguments)]
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
    log_audit_impl(conn, AuditEntry { user_id, action, entity_type, entity_id, old_value, new_value, details }, None)
}

/// Log an audit entry for a new entity that was split or derived from parent_lineage_id.
///
/// The new entity starts its own lineage (chain_seq = 1) but inherits prev_hash
/// from the parent lineage's last entry, creating a cryptographically visible fork.
/// Both siblings of a split share the same prev_hash, which is the intended behaviour:
/// it records "these two chains both originate from the same parent state."
#[allow(clippy::too_many_arguments)]
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
    log_audit_impl(conn, AuditEntry { user_id, action, entity_type, entity_id, old_value, new_value, details }, Some(parent_lineage_id))
}

/// Log an audit entry at chain_seq = 0 with prev_hash = ZERO_HASH.
///
/// Used exclusively for species creation. seq=0 marks the "birth" of the
/// species lineage and its entry_hash serves as the cryptographic seed that
/// new specimens inherit via log_audit_seeded_by_species.
#[allow(clippy::too_many_arguments)]
pub fn log_audit_at_seq_zero(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
) -> DbResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let lineage_id = entity_id.unwrap_or("system").to_string();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    let canonical = audit_canonical_bytes(
        &lineage_id, 0, &timestamp,
        user_id.unwrap_or(""), entity_type, entity_id.unwrap_or(""),
        action, details.unwrap_or(""),
    );
    let entry_hash = compute_entry_hash(&canonical, ZERO_HASH);
    conn.execute(
        "INSERT INTO audit_log \
         (id, user_id, action, entity_type, entity_id, old_value, new_value, details, created_at, \
          lineage_id, chain_seq, prev_hash, entry_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12)",
        params![
            id, user_id, action, entity_type, entity_id,
            old_value, new_value, details, timestamp,
            lineage_id, ZERO_HASH, entry_hash
        ],
    )?;
    Ok(())
}

/// Log an audit entry for a new root specimen, seeding its chain from the
/// species lineage. The specimen starts its own chain at seq=1 with prev_hash
/// set to the species' last entry_hash, cryptographically binding it to the
/// species definition it was created from.
///
/// Falls back to ZERO_HASH if the species has no audit entries (e.g. seeded
/// species written before the hash chain was introduced in v1.5.0).
#[allow(clippy::too_many_arguments)]
pub fn log_audit_seeded_by_species(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
    species_id: &str,
) -> DbResult<()> {
    log_audit_impl(conn, AuditEntry { user_id, action, entity_type, entity_id, old_value, new_value, details }, Some(species_id))
}

fn log_audit_impl(
    conn: &Connection,
    entry: AuditEntry<'_>,
    parent_lineage_id: Option<&str>,
) -> DbResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let lineage_id = entry.entity_id.unwrap_or("system").to_string();
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
        entry.user_id.unwrap_or(""),
        entry.entity_type,
        entry.entity_id.unwrap_or(""),
        entry.action,
        entry.details.unwrap_or(""),
    );
    let entry_hash = compute_entry_hash(&canonical, &prev_hash);

    conn.execute(
        "INSERT INTO audit_log \
         (id, user_id, action, entity_type, entity_id, old_value, new_value, details, created_at, \
          lineage_id, chain_seq, prev_hash, entry_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            id, entry.user_id, entry.action, entry.entity_type, entry.entity_id,
            entry.old_value, entry.new_value, entry.details, timestamp,
            lineage_id, next_seq, prev_hash, entry_hash
        ],
    )?;
    Ok(())
}

/// Returns `Ok(())` if a lab profile change is permitted given the current specimen count.
/// When `specimen_count > 0`, requires `confirmation` to equal `"CHANGE PROFILE"` (trimmed).
/// When `specimen_count == 0`, the confirmation argument is ignored.
pub fn check_profile_change_allowed(
    specimen_count: i64,
    confirmation: Option<&str>,
) -> Result<(), String> {
    if specimen_count == 0 {
        return Ok(());
    }
    match confirmation.map(str::trim) {
        Some("CHANGE PROFILE") => Ok(()),
        _ => Err(format!(
            "This lab has {} specimen{}. \
             Changing the active lab profile may affect vocabulary lookups and stage \
             validation for existing data. To confirm the change, type exactly: CHANGE PROFILE",
            specimen_count,
            if specimen_count == 1 { "" } else { "s" }
        )),
    }
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

    // --- Merkle tree tests ---

    #[test]
    fn merkle_empty_returns_zero_hash() {
        assert_eq!(build_merkle_root(&[]), ZERO_HASH);
    }

    #[test]
    fn merkle_single_leaf_returns_itself() {
        let leaf = "abc123".repeat(10);
        assert_eq!(build_merkle_root(&[leaf.clone()]), leaf);
    }

    #[test]
    fn merkle_two_leaves_hashes_them_together() {
        let a = "a".repeat(64);
        let b = "b".repeat(64);
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, a.as_bytes());
        sha2::Digest::update(&mut hasher, b.as_bytes());
        let expected = format!("{:x}", hasher.finalize());
        assert_eq!(build_merkle_root(&[a, b]), expected);
    }

    #[test]
    fn merkle_three_leaves_duplicates_last() {
        // 3 leaves [a, b, c] → pad to [a, b, c, c] → [hash(a,b), hash(c,c)] → hash(hash(a,b), hash(c,c))
        let a = "aa".repeat(32);
        let b = "bb".repeat(32);
        let c = "cc".repeat(32);

        let hash_ab = {
            let mut h = sha2::Sha256::new();
            sha2::Digest::update(&mut h, a.as_bytes());
            sha2::Digest::update(&mut h, b.as_bytes());
            format!("{:x}", h.finalize())
        };
        let hash_cc = {
            let mut h = sha2::Sha256::new();
            sha2::Digest::update(&mut h, c.as_bytes());
            sha2::Digest::update(&mut h, c.as_bytes());
            format!("{:x}", h.finalize())
        };
        let root = {
            let mut h = sha2::Sha256::new();
            sha2::Digest::update(&mut h, hash_ab.as_bytes());
            sha2::Digest::update(&mut h, hash_cc.as_bytes());
            format!("{:x}", h.finalize())
        };

        assert_eq!(build_merkle_root(&[a, b, c]), root);
    }

    #[test]
    fn merkle_root_is_deterministic() {
        let leaves: Vec<String> = (0..5).map(|i: u8| format!("{:064x}", i)).collect();
        assert_eq!(build_merkle_root(&leaves), build_merkle_root(&leaves));
    }

    #[test]
    fn merkle_root_changes_on_leaf_modification() {
        let leaves: Vec<String> = (0..4).map(|i: u8| format!("{:064x}", i)).collect();
        let root1 = build_merkle_root(&leaves);
        let mut leaves2 = leaves.clone();
        leaves2[2] = format!("{:064x}", 99u8);
        let root2 = build_merkle_root(&leaves2);
        assert_ne!(root1, root2);
    }

    fn mem_conn_with_checkpoints() -> Connection {
        let conn = mem_conn_with_audit();
        conn.execute_batch(
            "CREATE TABLE audit_checkpoints (
                id TEXT PRIMARY KEY,
                lineage_id TEXT NOT NULL,
                start_seq INTEGER NOT NULL,
                end_seq INTEGER NOT NULL,
                entry_count INTEGER NOT NULL,
                merkle_root TEXT NOT NULL,
                created_at TEXT NOT NULL,
                created_by TEXT,
                anchored_txid TEXT
            );",
        ).expect("create audit_checkpoints");
        conn
    }

    #[test]
    fn checkpoint_creation_stores_correct_merkle_root() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, Some("u1"), "create", "specimen", Some("sp-A"), None, None, Some("first")).unwrap();
        log_audit(&conn, Some("u1"), "update", "specimen", Some("sp-A"), None, None, Some("second")).unwrap();

        let hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log WHERE lineage_id='sp-A' ORDER BY chain_seq"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        assert_eq!(hashes.len(), 2);
        let expected_root = build_merkle_root(&hashes);

        let cp_id = uuid::Uuid::new_v4().to_string();
        let created_at = "2026-01-01T00:00:00.000Z";
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at) \
             VALUES (?1, 'sp-A', 1, 2, 2, ?2, ?3)",
            rusqlite::params![cp_id, expected_root, created_at],
        ).unwrap();

        let stored_root: String = conn.query_row(
            "SELECT merkle_root FROM audit_checkpoints WHERE id = ?1", rusqlite::params![cp_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(stored_root, expected_root);
    }

    #[test]
    fn checkpoint_verification_passes_on_intact_chain() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-V"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-V"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-V"), None, None, None).unwrap();

        // Snapshot entry_hash values and build root
        let hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log WHERE lineage_id='sp-V' ORDER BY chain_seq"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        let root = build_merkle_root(&hashes);
        let cp_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at) \
             VALUES (?1, 'sp-V', 1, 3, 3, ?2, '2026-01-01')",
            rusqlite::params![cp_id, root],
        ).unwrap();

        // Re-fetch current hashes and rebuild root → must match
        let current_hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log \
                 WHERE lineage_id='sp-V' AND chain_seq >= 1 AND chain_seq <= 3 \
                 ORDER BY chain_seq"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        let stored_root: String = conn.query_row(
            "SELECT merkle_root FROM audit_checkpoints WHERE id = ?1", rusqlite::params![cp_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(build_merkle_root(&current_hashes), stored_root, "intact chain must verify");
    }

    #[test]
    fn checkpoint_verification_detects_tamper() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-T"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-T"), None, None, None).unwrap();

        let hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log WHERE lineage_id='sp-T' ORDER BY chain_seq"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        let root = build_merkle_root(&hashes);
        let cp_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at) \
             VALUES (?1, 'sp-T', 1, 2, 2, ?2, '2026-01-01')",
            rusqlite::params![cp_id, root],
        ).unwrap();

        // Tamper: overwrite entry_hash of the first row with a fake hash
        conn.execute(
            "UPDATE audit_log SET entry_hash = ?1 WHERE lineage_id = 'sp-T' AND chain_seq = 1",
            rusqlite::params!["deadbeef".repeat(8)],
        ).unwrap();

        // Rebuild root from current (tampered) hashes → must differ
        let tampered_hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log \
                 WHERE lineage_id='sp-T' AND chain_seq >= 1 AND chain_seq <= 2 \
                 ORDER BY chain_seq"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        let stored_root: String = conn.query_row(
            "SELECT merkle_root FROM audit_checkpoints WHERE id = ?1", rusqlite::params![cp_id], |r| r.get(0),
        ).unwrap();
        assert_ne!(build_merkle_root(&tampered_hashes), stored_root, "tampered root must not match checkpoint");
    }

    #[test]
    fn checkpoint_verification_detects_removal() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-R"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-R"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-R"), None, None, None).unwrap();

        let hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log WHERE lineage_id='sp-R' ORDER BY chain_seq"
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        let expected_count = hashes.len() as i64;
        let root = build_merkle_root(&hashes);

        let cp_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_checkpoints (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at) \
             VALUES (?1, 'sp-R', 1, 3, ?2, ?3, '2026-01-01')",
            rusqlite::params![cp_id, expected_count, root],
        ).unwrap();

        // Remove an entry — simulates deletion attack
        conn.execute(
            "DELETE FROM audit_log WHERE lineage_id = 'sp-R' AND chain_seq = 2",
            [],
        ).unwrap();

        // Count detection: after removal the count in the range [1,3] is now 2, not 3
        let current_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE lineage_id='sp-R' AND chain_seq >= 1 AND chain_seq <= 3",
            [],
            |r| r.get(0),
        ).unwrap();
        assert_ne!(current_count, expected_count, "removal must be detected via count mismatch");
    }

    #[test]
    fn checkpoint_on_nonexistent_lineage_yields_no_hashes() {
        let conn = mem_conn_with_checkpoints();
        // No audit entries are inserted — query must return empty
        let mut stmt = conn.prepare(
            "SELECT entry_hash FROM audit_log \
             WHERE lineage_id='does-not-exist' AND entry_hash IS NOT NULL ORDER BY chain_seq",
        ).unwrap();
        let hashes: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert!(hashes.is_empty(), "nonexistent lineage must yield empty hash list");
        // The command layer rejects empty hashes with an error, but the Merkle
        // function itself handles the empty case gracefully.
        assert_eq!(build_merkle_root(&hashes), ZERO_HASH);
    }

    #[test]
    fn checkpoint_passes_after_entries_added_beyond_end_seq() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-X"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-X"), None, None, None).unwrap();

        // Snapshot hashes for seq 1-2 and create a checkpoint over that range
        let hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log \
                 WHERE lineage_id='sp-X' AND chain_seq >= 1 AND chain_seq <= 2 ORDER BY chain_seq",
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        assert_eq!(hashes.len(), 2);
        let root = build_merkle_root(&hashes);
        let cp_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO audit_checkpoints \
             (id, lineage_id, start_seq, end_seq, entry_count, merkle_root, created_at) \
             VALUES (?1, 'sp-X', 1, 2, 2, ?2, '2026-01-01')",
            rusqlite::params![cp_id, root],
        ).unwrap();

        // Add a third entry beyond the sealed end_seq
        log_audit(&conn, None, "update", "specimen", Some("sp-X"), None, None, None).unwrap();

        // Checkpoint covers only seq 1-2; the new entry must not affect the result
        let sealed_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE lineage_id='sp-X' AND chain_seq >= 1 AND chain_seq <= 2",
            [], |r| r.get(0),
        ).unwrap();
        let sealed_hashes: Vec<String> = {
            let mut s = conn.prepare(
                "SELECT entry_hash FROM audit_log \
                 WHERE lineage_id='sp-X' AND chain_seq >= 1 AND chain_seq <= 2 ORDER BY chain_seq",
            ).unwrap();
            s.query_map([], |r| r.get(0)).unwrap().filter_map(|r| r.ok()).collect()
        };
        let stored_root: String = conn.query_row(
            "SELECT merkle_root FROM audit_checkpoints WHERE id = ?1",
            rusqlite::params![cp_id], |r| r.get(0),
        ).unwrap();
        assert_eq!(sealed_count, 2, "count check must match despite new entry beyond end_seq");
        assert_eq!(build_merkle_root(&sealed_hashes), stored_root,
            "Merkle root must still match after entries added beyond end_seq");
    }

    #[test]
    fn checkpoint_seq_range_outside_actual_entries_yields_empty() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-Y"), None, None, None).unwrap();

        // Only seq 1 exists; query seq 100-200 — must return no rows
        let mut stmt = conn.prepare(
            "SELECT entry_hash FROM audit_log \
             WHERE lineage_id='sp-Y' AND chain_seq >= 100 AND chain_seq <= 200 \
             AND entry_hash IS NOT NULL ORDER BY chain_seq",
        ).unwrap();
        let hashes: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert!(hashes.is_empty(), "out-of-range seq window must return no hashes");
    }

    #[test]
    fn checkpoint_inverted_seq_range_yields_empty() {
        let conn = mem_conn_with_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-Z"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-Z"), None, None, None).unwrap();

        // start_seq (5) > end_seq (3) — SQL returns 0 rows;
        // the command layer rejects this before querying, but the SQL contract holds
        let mut stmt = conn.prepare(
            "SELECT entry_hash FROM audit_log \
             WHERE lineage_id='sp-Z' AND chain_seq >= 5 AND chain_seq <= 3 \
             AND entry_hash IS NOT NULL ORDER BY chain_seq",
        ).unwrap();
        let hashes: Vec<String> = stmt
            .query_map([], |r| r.get(0)).unwrap()
            .filter_map(|r| r.ok()).collect();
        assert!(hashes.is_empty(), "inverted seq range must yield no hashes");
    }

    // --- WP-21: Merkle path tests ---

    #[test]
    fn merkle_path_single_leaf_returns_empty() {
        let leaves = vec!["abc123".to_string()];
        let path = build_merkle_path(&leaves, 0);
        assert!(path.is_empty(), "single-leaf tree has no path nodes");
        // empty path: verify_merkle_path compares leaf directly to expected root
        assert!(verify_merkle_path(&leaves[0], &path, &leaves[0]));
    }

    #[test]
    fn merkle_path_verifies_for_four_leaves() {
        let leaves: Vec<String> = (0..4u8).map(|i| format!("{:064x}", i)).collect();
        let root = build_merkle_root(&leaves);
        for i in 0..leaves.len() {
            let path = build_merkle_path(&leaves, i);
            assert!(
                verify_merkle_path(&leaves[i], &path, &root),
                "path verification failed for leaf index {i}",
            );
        }
    }

    #[test]
    fn merkle_path_verifies_for_three_leaves_odd() {
        // Odd count forces duplicate-last padding; all three leaves must still verify.
        let leaves: Vec<String> = (0..3u8).map(|i| format!("{:064x}", i)).collect();
        let root = build_merkle_root(&leaves);
        for i in 0..leaves.len() {
            let path = build_merkle_path(&leaves, i);
            assert!(
                verify_merkle_path(&leaves[i], &path, &root),
                "odd-count path verification failed for leaf index {i}",
            );
        }
    }

    // --- WP-21: Auto-checkpoint tests ---

    fn mem_conn_with_auto_checkpoints() -> Connection {
        let conn = mem_conn_with_audit();
        conn.execute_batch(
            "CREATE TABLE audit_checkpoints (
                id TEXT PRIMARY KEY,
                lineage_id TEXT NOT NULL,
                start_seq INTEGER NOT NULL,
                end_seq INTEGER NOT NULL,
                entry_count INTEGER NOT NULL,
                merkle_root TEXT NOT NULL,
                created_at TEXT NOT NULL,
                created_by TEXT,
                anchored_txid TEXT,
                is_auto INTEGER NOT NULL DEFAULT 0,
                auto_source TEXT
            );",
        ).expect("create audit_checkpoints with auto columns");
        conn
    }

    #[test]
    fn auto_checkpoint_creates_for_eligible_lineage() {
        let conn = mem_conn_with_auto_checkpoints();
        log_audit(&conn, Some("u1"), "create", "specimen", Some("sp-A"), None, None, None).unwrap();
        log_audit(&conn, Some("u1"), "update", "specimen", Some("sp-A"), None, None, None).unwrap();

        // min_uncovered = 0 → any lineage with uncovered entries qualifies
        let created = auto_checkpoint_lineages(&conn, "u1", "test", 0).unwrap();
        assert_eq!(created.len(), 1, "should create exactly one checkpoint");

        let (is_auto, source): (i64, String) = conn.query_row(
            "SELECT is_auto, auto_source FROM audit_checkpoints WHERE id = ?1",
            rusqlite::params![&created[0]],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(is_auto, 1, "auto-checkpoint must be flagged is_auto=1");
        assert_eq!(source, "test");
    }

    #[test]
    fn auto_checkpoint_respects_min_uncovered_interval() {
        let conn = mem_conn_with_auto_checkpoints();
        for _ in 0..5 {
            log_audit(&conn, None, "update", "specimen", Some("sp-B"), None, None, None).unwrap();
        }

        // 5 entries < 10 threshold → no checkpoint
        let created = auto_checkpoint_lineages(&conn, "u1", "test", 10).unwrap();
        assert!(created.is_empty(), "should not checkpoint when below min_uncovered threshold");

        // 5 entries == 5 threshold → should checkpoint
        let created = auto_checkpoint_lineages(&conn, "u1", "test", 5).unwrap();
        assert_eq!(created.len(), 1, "should checkpoint when exactly at the min_uncovered threshold");
    }

    #[test]
    fn auto_checkpoint_skips_if_not_enough_entries() {
        let conn = mem_conn_with_auto_checkpoints();
        log_audit(&conn, None, "create", "specimen", Some("sp-C"), None, None, None).unwrap();

        // 1 entry < 3 threshold → skip
        let created = auto_checkpoint_lineages(&conn, "u1", "test", 3).unwrap();
        assert!(created.is_empty(), "lineage with too few entries must be skipped");

        // Add 2 more (total = 3) → now qualifies
        log_audit(&conn, None, "update", "specimen", Some("sp-C"), None, None, None).unwrap();
        log_audit(&conn, None, "update", "specimen", Some("sp-C"), None, None, None).unwrap();
        let created = auto_checkpoint_lineages(&conn, "u1", "test", 3).unwrap();
        assert_eq!(created.len(), 1, "lineage with exactly min_uncovered entries must qualify");
    }

    // ── check_profile_change_allowed ──────────────────────────────────────────

    #[test]
    fn profile_change_no_specimens_always_allowed() {
        assert!(check_profile_change_allowed(0, None).is_ok());
    }

    #[test]
    fn profile_change_no_specimens_ignores_confirmation() {
        // Confirmation is accepted but not required when the lab is empty.
        assert!(check_profile_change_allowed(0, Some("CHANGE PROFILE")).is_ok());
    }

    #[test]
    fn profile_change_with_specimens_blocked_without_confirmation() {
        let err = check_profile_change_allowed(5, None).unwrap_err();
        assert!(err.contains("5 specimens"), "error should report count: {err}");
        assert!(err.contains("CHANGE PROFILE"), "error should name the phrase: {err}");
    }

    #[test]
    fn profile_change_with_specimens_blocked_on_wrong_confirmation() {
        let err = check_profile_change_allowed(3, Some("yes")).unwrap_err();
        assert!(err.contains("CHANGE PROFILE"), "error should name the phrase: {err}");
    }

    #[test]
    fn profile_change_with_specimens_allowed_on_correct_confirmation() {
        assert!(check_profile_change_allowed(12, Some("CHANGE PROFILE")).is_ok());
    }

    #[test]
    fn profile_change_confirmation_trimmed() {
        // Leading/trailing whitespace is accepted.
        assert!(check_profile_change_allowed(1, Some("  CHANGE PROFILE  ")).is_ok());
    }

    #[test]
    fn profile_change_singular_specimen_grammar() {
        let err = check_profile_change_allowed(1, None).unwrap_err();
        assert!(err.contains("1 specimen."), "should use singular 'specimen': {err}");
    }
}
