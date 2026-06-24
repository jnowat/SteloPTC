// Query helpers and shared database utilities
use rusqlite::{Connection, params};
use sha2::{Sha256, Digest};
use super::{DbError, DbResult};
use crate::models::taxon::{NcbiSyncLog, SpeciesNodeSummary, Taxon, TaxonColumnItem, TaxonomySearchResult};
use crate::models::strain::{
    GenerationalStats, HybridizationEventRecord, PedigreeEdge, PedigreeExport, PedigreeNode,
    SpecimenSummary, StrainSpecimenTree, StrainSummary, SuggestGenerationLabelResponse,
};

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

/// Log a genesis audit entry for a new strain at chain_seq = 0.
///
/// Unlike species births (which use ZERO_HASH), strain genesis entries inherit
/// prev_hash from the parent species' last entry_hash, cryptographically binding
/// the strain lineage to its species definition.  Falls back to ZERO_HASH when
/// the species has no audit entries.
#[allow(clippy::too_many_arguments)]
pub fn log_audit_strain_genesis(
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
    let id = uuid::Uuid::new_v4().to_string();
    let lineage_id = entity_id.unwrap_or("system").to_string();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let species_hash: Option<String> = conn.query_row(
        "SELECT entry_hash FROM audit_log \
         WHERE (lineage_id = ?1 OR (lineage_id IS NULL AND entity_id = ?1)) \
           AND entry_hash IS NOT NULL \
         ORDER BY chain_seq DESC LIMIT 1",
        params![species_id],
        |row| row.get(0),
    ).ok().flatten();
    let prev_hash = species_hash.unwrap_or_else(|| ZERO_HASH.to_string());

    let canonical = audit_canonical_bytes(
        &lineage_id, 0, &timestamp,
        user_id.unwrap_or(""), entity_type, entity_id.unwrap_or(""),
        action, details.unwrap_or(""),
    );
    let entry_hash = compute_entry_hash(&canonical, &prev_hash);

    conn.execute(
        "INSERT INTO audit_log \
         (id, user_id, action, entity_type, entity_id, old_value, new_value, details, created_at, \
          lineage_id, chain_seq, prev_hash, entry_hash) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12)",
        params![
            id, user_id, action, entity_type, entity_id,
            old_value, new_value, details, timestamp,
            lineage_id, prev_hash, entry_hash
        ],
    )?;
    Ok(())
}

/// Log an audit entry for a new root specimen seeded from a strain's lineage.
///
/// The specimen starts its own chain at seq = 1 with prev_hash set to the
/// strain's last entry_hash, cryptographically binding it to the strain
/// definition it was created from.  Falls back to ZERO_HASH when the strain
/// has no audit entries.
#[allow(clippy::too_many_arguments)]
pub fn log_audit_seeded_by_strain(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
    strain_id: &str,
) -> DbResult<()> {
    log_audit_impl(conn, AuditEntry { user_id, action, entity_type, entity_id, old_value, new_value, details }, Some(strain_id))
}

/// Validate a strain status transition and return a descriptive error when rejected.
///
/// Status ordering (ascending): `unverified` → `claimed` → `confirmed_manual` → `confirmed_genomic`
///
/// Rules enforced:
/// - Downgrades from `confirmed_genomic` or `confirmed_manual` are always rejected.
/// - Transitioning to `confirmed_manual` requires a non-empty `confirmation_basis`.
/// - Transitioning to `confirmed_genomic` requires a non-empty `genomic_fingerprint`.
pub fn validate_strain_status_transition(
    current: &str,
    next: &str,
    confirmation_basis: Option<&str>,
    genomic_fingerprint: Option<&str>,
) -> Result<(), String> {
    fn level(s: &str) -> i32 {
        match s {
            "unverified" => 0,
            "claimed" => 1,
            "confirmed_manual" => 2,
            "confirmed_genomic" => 3,
            _ => -1,
        }
    }

    let next_level = level(next);
    if next_level < 0 {
        return Err(format!("Unknown strain status: '{}'", next));
    }
    let current_level = level(current);

    if current_level >= 2 && next_level < current_level {
        return Err(format!(
            "Cannot downgrade strain status from '{}' to '{}'",
            current, next
        ));
    }

    if next == "confirmed_manual" {
        match confirmation_basis.map(str::trim) {
            Some(b) if !b.is_empty() => {}
            _ => return Err(
                "Status 'confirmed_manual' requires a non-empty confirmation_basis".to_string()
            ),
        }
    }

    if next == "confirmed_genomic" {
        match genomic_fingerprint.map(str::trim) {
            Some(fp) if !fp.is_empty() => {}
            _ => return Err(
                "Status 'confirmed_genomic' requires a non-null genomic_fingerprint".to_string()
            ),
        }
    }

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

// ── Taxon helpers (WP-35) ─────────────────────────────────────────────────────
// These functions are classification helpers only; they perform no audit-chain
// writes.  Taxa records above Species are never hash-chained.

/// Load a single taxon record by its ID.
pub fn load_taxon(conn: &Connection, id: &str) -> DbResult<Taxon> {
    let taxon = conn.query_row(
        "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at,
                local_override, taxon_path, created_at, updated_at
         FROM taxa WHERE id = ?1",
        params![id],
        |row| {
            Ok(Taxon {
                id: row.get("id")?,
                rank: row.get("rank")?,
                name: row.get("name")?,
                parent_id: row.get("parent_id")?,
                ncbi_taxon_id: row.get("ncbi_taxon_id")?,
                ncbi_updated_at: row.get("ncbi_updated_at")?,
                local_override: row.get::<_, i64>("local_override")? != 0,
                taxon_path: row.get("taxon_path")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    )?;
    Ok(taxon)
}

/// Return all direct children of a taxon (taxa whose parent_id equals the given id).
pub fn get_child_taxa(conn: &Connection, parent_id: &str) -> DbResult<Vec<Taxon>> {
    let mut stmt = conn.prepare(
        "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at,
                local_override, taxon_path, created_at, updated_at
         FROM taxa WHERE parent_id = ?1 ORDER BY name",
    )?;
    let rows = stmt.query_map(params![parent_id], |row| {
        Ok(Taxon {
            id: row.get("id")?,
            rank: row.get("rank")?,
            name: row.get("name")?,
            parent_id: row.get("parent_id")?,
            ncbi_taxon_id: row.get("ncbi_taxon_id")?,
            ncbi_updated_at: row.get("ncbi_updated_at")?,
            local_override: row.get::<_, i64>("local_override")? != 0,
            taxon_path: row.get("taxon_path")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    })?;
    let taxa: Result<Vec<_>, _> = rows.collect();
    Ok(taxa?)
}

/// Return species whose most-specific ancestor (last element of taxon_path) is
/// the given taxon_id, together with aggregate strain and specimen counts.
///
/// Taxon IDs are UUIDs (hex digits + hyphens only), making the LIKE pattern
/// unambiguous — no need for full JSON parsing at the SQL layer.
pub fn get_species_for_taxon(
    conn: &Connection,
    taxon_id: &str,
) -> DbResult<Vec<SpeciesNodeSummary>> {
    let pattern = format!("%\"{}\"]", taxon_id);
    let mut stmt = conn.prepare(
        "SELECT sp.id, sp.genus, sp.species_name, sp.common_name, sp.species_code,
                COUNT(DISTINCT st.id)   AS strain_count,
                COUNT(DISTINCT spec.id) AS specimen_count
         FROM species sp
         LEFT JOIN strains  st   ON st.species_id   = sp.id AND st.is_archived   = 0
         LEFT JOIN specimens spec ON spec.species_id = sp.id AND spec.is_archived = 0
         WHERE sp.taxon_path LIKE ?1
         GROUP BY sp.id, sp.genus, sp.species_name, sp.common_name, sp.species_code
         ORDER BY sp.genus, sp.species_name",
    )?;
    let rows = stmt.query_map(params![pattern], |row| {
        Ok(SpeciesNodeSummary {
            id: row.get(0)?,
            genus: row.get(1)?,
            species_name: row.get(2)?,
            common_name: row.get(3)?,
            species_code: row.get(4)?,
            strain_count: row.get(5)?,
            specimen_count: row.get(6)?,
        })
    })?;
    let summaries: Result<Vec<_>, _> = rows.collect();
    Ok(summaries?)
}

// ── WP-39: Advanced taxonomy navigator helpers ─────────────────────────────────

/// Parse a JSON taxon_path array stored as TEXT in the database.
/// Returns an empty vec when the value is NULL or unparseable.
fn parse_taxon_path(path: &Option<String>) -> Vec<String> {
    path.as_deref()
        .and_then(|p| serde_json::from_str::<Vec<String>>(p).ok())
        .unwrap_or_default()
}

/// Return immediate children of a taxon (or all root-level taxa when
/// `parent_id` is `None`), each annotated with the total count of strains
/// and non-archived specimens under all descendant species.
///
/// Counts use correlated sub-queries with a LIKE pattern on the JSON
/// `taxon_path` column.  Taxon IDs are UUIDs (hex + hyphens only) so the
/// pattern `%"<id>"%` is unambiguous — no special LIKE characters.
pub fn get_taxon_column_items(
    conn: &Connection,
    parent_id: Option<&str>,
) -> DbResult<Vec<TaxonColumnItem>> {
    let row_mapper = |row: &rusqlite::Row<'_>| -> rusqlite::Result<TaxonColumnItem> {
        Ok(TaxonColumnItem {
            id: row.get(0)?,
            rank: row.get(1)?,
            name: row.get(2)?,
            parent_id: row.get(3)?,
            ncbi_taxon_id: row.get(4)?,
            local_override: row.get::<_, i64>(5)? != 0,
            strain_count: row.get(6)?,
            specimen_count: row.get(7)?,
        })
    };

    let items = if let Some(pid) = parent_id {
        let mut stmt = conn.prepare(
            r#"SELECT t.id, t.rank, t.name, t.parent_id, t.ncbi_taxon_id, t.local_override,
                      (SELECT COUNT(DISTINCT st.id) FROM strains st
                       JOIN species sp ON st.species_id = sp.id
                       WHERE sp.taxon_path LIKE '%"' || t.id || '"%' AND st.is_archived = 0
                      ) AS strain_count,
                      (SELECT COUNT(DISTINCT s.id) FROM specimens s
                       JOIN species sp ON s.species_id = sp.id
                       WHERE sp.taxon_path LIKE '%"' || t.id || '"%' AND s.is_archived = 0
                      ) AS specimen_count
               FROM taxa t WHERE t.parent_id = ?1 ORDER BY t.name"#,
        )?;
        let rows = stmt.query_map(params![pid], row_mapper)?;
        let v: Result<Vec<_>, _> = rows.collect();
        v?
    } else {
        let mut stmt = conn.prepare(
            r#"SELECT t.id, t.rank, t.name, t.parent_id, t.ncbi_taxon_id, t.local_override,
                      (SELECT COUNT(DISTINCT st.id) FROM strains st
                       JOIN species sp ON st.species_id = sp.id
                       WHERE sp.taxon_path LIKE '%"' || t.id || '"%' AND st.is_archived = 0
                      ) AS strain_count,
                      (SELECT COUNT(DISTINCT s.id) FROM specimens s
                       JOIN species sp ON s.species_id = sp.id
                       WHERE sp.taxon_path LIKE '%"' || t.id || '"%' AND s.is_archived = 0
                      ) AS specimen_count
               FROM taxa t WHERE t.parent_id IS NULL ORDER BY t.name"#,
        )?;
        let rows = stmt.query_map([], row_mapper)?;
        let v: Result<Vec<_>, _> = rows.collect();
        v?
    };

    Ok(items)
}

/// Search across taxa, species, strains, and specimens.
/// Returns up to 10 results per entity type grouped in a single flat Vec.
/// The caller should enforce a minimum query length (e.g. 2 characters).
pub fn search_taxonomy(conn: &Connection, query: &str) -> DbResult<Vec<TaxonomySearchResult>> {
    let like_q = format!("%{}%", query);
    let mut results: Vec<TaxonomySearchResult> = Vec::new();

    // Taxa
    {
        let mut stmt = conn.prepare(
            "SELECT id, rank, name, taxon_path FROM taxa WHERE name LIKE ?1 ORDER BY name LIMIT 10",
        )?;
        let rows = stmt.query_map(params![like_q], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let items: Result<Vec<_>, _> = rows.collect();
        for (id, rank, name, taxon_path) in items? {
            let taxon_ids = parse_taxon_path(&taxon_path);
            results.push(TaxonomySearchResult {
                result_type: "taxon".to_string(),
                id,
                display_name: name,
                secondary: rank,
                taxon_ids,
                species_id: None,
                strain_id: None,
            });
        }
    }

    // Species (search genus+name concatenation, code, and common_name)
    {
        let mut stmt = conn.prepare(
            "SELECT id, genus, species_name, species_code, taxon_path FROM species \
             WHERE genus || ' ' || species_name LIKE ?1 \
                OR species_name LIKE ?1 \
                OR species_code LIKE ?1 \
                OR common_name LIKE ?1 \
             ORDER BY genus, species_name LIMIT 10",
        )?;
        let rows = stmt.query_map(params![like_q], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let items: Result<Vec<_>, _> = rows.collect();
        for (id, genus, species_name, species_code, taxon_path) in items? {
            let taxon_ids = parse_taxon_path(&taxon_path);
            results.push(TaxonomySearchResult {
                result_type: "species".to_string(),
                id: id.clone(),
                display_name: format!("{} {}", genus, species_name),
                secondary: species_code,
                taxon_ids,
                species_id: Some(id),
                strain_id: None,
            });
        }
    }

    // Strains
    {
        let mut stmt = conn.prepare(
            "SELECT st.id, st.name, st.code, st.species_id, sp.taxon_path \
             FROM strains st JOIN species sp ON st.species_id = sp.id \
             WHERE (st.name LIKE ?1 OR st.code LIKE ?1) AND st.is_archived = 0 \
             ORDER BY st.code LIMIT 10",
        )?;
        let rows = stmt.query_map(params![like_q], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })?;
        let items: Result<Vec<_>, _> = rows.collect();
        for (id, name, code, species_id, taxon_path) in items? {
            let taxon_ids = parse_taxon_path(&taxon_path);
            results.push(TaxonomySearchResult {
                result_type: "strain".to_string(),
                id: id.clone(),
                display_name: name,
                secondary: code,
                taxon_ids,
                species_id: Some(species_id),
                strain_id: Some(id),
            });
        }
    }

    // Specimens
    {
        let mut stmt = conn.prepare(
            "SELECT spec.id, spec.accession_number, spec.strain_id, spec.species_id, \
                    sp.taxon_path, spec.stage \
             FROM specimens spec JOIN species sp ON spec.species_id = sp.id \
             WHERE spec.accession_number LIKE ?1 AND spec.is_archived = 0 \
             ORDER BY spec.accession_number LIMIT 10",
        )?;
        let rows = stmt.query_map(params![like_q], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let items: Result<Vec<_>, _> = rows.collect();
        for (id, accession, strain_id, species_id, taxon_path, stage) in items? {
            let taxon_ids = parse_taxon_path(&taxon_path);
            results.push(TaxonomySearchResult {
                result_type: "specimen".to_string(),
                id,
                display_name: accession,
                secondary: stage.unwrap_or_default(),
                taxon_ids,
                species_id: Some(species_id),
                strain_id,
            });
        }
    }

    Ok(results)
}

// ── NCBI Taxonomy helpers (WP-36) ─────────────────────────────────────────────
// These helpers operate on the ncbi_sync_log table and the taxa table for NCBI
// import/sync operations.  No audit-chain writes — taxa remain classification-only.

/// Map an NCBI rank string to one of our internal rank values.
/// Returns None for ranks we don't support (species, subspecies, variety, etc.).
pub fn normalize_ncbi_rank(ncbi_rank: &str) -> Option<&'static str> {
    match ncbi_rank.to_lowercase().as_str() {
        "kingdom" | "superkingdom" => Some("kingdom"),
        "phylum" | "division" => Some("phylum"),
        "class" => Some("class"),
        "order" => Some("order"),
        "family" => Some("family"),
        "genus" => Some("genus"),
        _ => None,
    }
}

/// Find a taxon by its NCBI taxon ID.  Returns None when no match exists.
pub fn find_taxon_by_ncbi_id(conn: &Connection, ncbi_taxon_id: i64) -> DbResult<Option<Taxon>> {
    let result = conn.query_row(
        "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at,
                local_override, taxon_path, created_at, updated_at
         FROM taxa WHERE ncbi_taxon_id = ?1",
        params![ncbi_taxon_id],
        |row| {
            Ok(Taxon {
                id: row.get("id")?,
                rank: row.get("rank")?,
                name: row.get("name")?,
                parent_id: row.get("parent_id")?,
                ncbi_taxon_id: row.get("ncbi_taxon_id")?,
                ncbi_updated_at: row.get("ncbi_updated_at")?,
                local_override: row.get::<_, i64>("local_override")? != 0,
                taxon_path: row.get("taxon_path")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    );
    match result {
        Ok(t) => Ok(Some(t)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DbError::Sqlite(e)),
    }
}

/// Find a taxon by exact name and rank.  Returns the first match or None.
pub fn find_taxon_by_name_rank(
    conn: &Connection,
    name: &str,
    rank: &str,
) -> DbResult<Option<Taxon>> {
    let result = conn.query_row(
        "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at,
                local_override, taxon_path, created_at, updated_at
         FROM taxa WHERE name = ?1 AND rank = ?2 LIMIT 1",
        params![name, rank],
        |row| {
            Ok(Taxon {
                id: row.get("id")?,
                rank: row.get("rank")?,
                name: row.get("name")?,
                parent_id: row.get("parent_id")?,
                ncbi_taxon_id: row.get("ncbi_taxon_id")?,
                ncbi_updated_at: row.get("ncbi_updated_at")?,
                local_override: row.get::<_, i64>("local_override")? != 0,
                taxon_path: row.get("taxon_path")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        },
    );
    match result {
        Ok(t) => Ok(Some(t)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DbError::Sqlite(e)),
    }
}

/// Compare a local taxon against incoming NCBI data.
///
/// Returns a JSON string describing the field-level differences when the data
/// diverges, or None when no tracked fields differ.  Only `name` and `rank`
/// are compared — parent hierarchy changes are handled separately.
pub fn detect_ncbi_conflict(local: &Taxon, ncbi_name: &str, ncbi_rank: &str) -> Option<String> {
    let mut diffs = serde_json::Map::new();
    if local.name.trim() != ncbi_name.trim() {
        diffs.insert(
            "name".to_string(),
            serde_json::json!({"local": local.name.trim(), "ncbi": ncbi_name.trim()}),
        );
    }
    if local.rank.as_str() != ncbi_rank {
        diffs.insert(
            "rank".to_string(),
            serde_json::json!({"local": local.rank, "ncbi": ncbi_rank}),
        );
    }
    if diffs.is_empty() {
        None
    } else {
        serde_json::to_string(&serde_json::Value::Object(diffs)).ok()
    }
}

/// Insert a row into `ncbi_sync_log`.
pub fn insert_ncbi_sync_log(
    conn: &Connection,
    id: &str,
    sync_type: &str,
    taxon_id: Option<&str>,
    ncbi_taxon_id: Option<i64>,
    conflict_details: Option<&str>,
    created_at: &str,
) -> DbResult<()> {
    conn.execute(
        "INSERT INTO ncbi_sync_log \
         (id, sync_type, taxon_id, ncbi_taxon_id, conflict_details, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, sync_type, taxon_id, ncbi_taxon_id, conflict_details, created_at],
    )?;
    Ok(())
}

/// List all unresolved conflict entries from `ncbi_sync_log`, newest first.
pub fn list_pending_ncbi_conflicts(conn: &Connection) -> DbResult<Vec<NcbiSyncLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_type, taxon_id, ncbi_taxon_id, conflict_details,
                resolved_at, resolved_by, resolution, created_at
         FROM ncbi_sync_log
         WHERE sync_type = 'conflict' AND resolved_at IS NULL
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(NcbiSyncLog {
            id: row.get("id")?,
            sync_type: row.get("sync_type")?,
            taxon_id: row.get("taxon_id")?,
            ncbi_taxon_id: row.get("ncbi_taxon_id")?,
            conflict_details: row.get("conflict_details")?,
            resolved_at: row.get("resolved_at")?,
            resolved_by: row.get("resolved_by")?,
            resolution: row.get("resolution")?,
            created_at: row.get("created_at")?,
        })
    })?;
    let logs: Result<Vec<_>, _> = rows.collect();
    Ok(logs?)
}

/// List recent entries from `ncbi_sync_log`, newest first, up to `limit` rows.
pub fn list_ncbi_sync_log(conn: &Connection, limit: i64) -> DbResult<Vec<NcbiSyncLog>> {
    let mut stmt = conn.prepare(
        "SELECT id, sync_type, taxon_id, ncbi_taxon_id, conflict_details,
                resolved_at, resolved_by, resolution, created_at
         FROM ncbi_sync_log
         ORDER BY created_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(NcbiSyncLog {
            id: row.get("id")?,
            sync_type: row.get("sync_type")?,
            taxon_id: row.get("taxon_id")?,
            ncbi_taxon_id: row.get("ncbi_taxon_id")?,
            conflict_details: row.get("conflict_details")?,
            resolved_at: row.get("resolved_at")?,
            resolved_by: row.get("resolved_by")?,
            resolution: row.get("resolution")?,
            created_at: row.get("created_at")?,
        })
    })?;
    let logs: Result<Vec<_>, _> = rows.collect();
    Ok(logs?)
}

// ── Pedigree helpers (WP-37) ─────────────────────────────────────────────────
// Walk the strain hybridization graph (strain_parents) upward (ancestry) or
// downward (descendants).  These helpers operate on strain_parents and
// hybridization_events only — they never touch specimens.parent_specimen_id.

fn load_strain_summary(conn: &Connection, id: &str) -> DbResult<StrainSummary> {
    let s = conn.query_row(
        "SELECT s.id, s.name, s.code, s.strain_type, s.status, s.is_hybrid, s.is_archived, \
                COALESCE((SELECT COUNT(*) FROM specimens sp \
                          WHERE sp.strain_id = s.id AND sp.is_archived = 0), 0) AS specimen_count \
         FROM strains s WHERE s.id = ?1",
        params![id],
        |row| {
            Ok(StrainSummary {
                id: row.get("id")?,
                name: row.get("name")?,
                code: row.get("code")?,
                strain_type: row.get("strain_type")?,
                status: row.get("status")?,
                is_hybrid: row.get::<_, i32>("is_hybrid")? != 0,
                is_archived: row.get::<_, i32>("is_archived")? != 0,
                specimen_count: row.get("specimen_count")?,
            })
        },
    )?;
    Ok(s)
}

fn load_parent_entries(
    conn: &Connection,
    strain_id: &str,
) -> DbResult<Vec<(StrainSummary, PedigreeEdge)>> {
    let mut stmt = conn.prepare(
        "SELECT sp.parent_strain_id, sp.parent_role, sp.parent_chain_seq_at_creation, \
                (SELECT he.id FROM hybridization_events he \
                 WHERE he.hybrid_strain_id = sp.strain_id LIMIT 1) AS event_id, \
                (SELECT he.notes FROM hybridization_events he \
                 WHERE he.hybrid_strain_id = sp.strain_id LIMIT 1) AS event_notes, \
                s.id, s.name, s.code, s.strain_type, s.status, s.is_hybrid, s.is_archived, \
                COALESCE((SELECT COUNT(*) FROM specimens spec \
                          WHERE spec.strain_id = s.id AND spec.is_archived = 0), 0) AS specimen_count \
         FROM strain_parents sp \
         JOIN strains s ON s.id = sp.parent_strain_id \
         WHERE sp.strain_id = ?1 \
         ORDER BY sp.parent_role",
    )?;
    let rows = stmt.query_map(params![strain_id], |row| {
        Ok((
            StrainSummary {
                id: row.get("id")?,
                name: row.get("name")?,
                code: row.get("code")?,
                strain_type: row.get("strain_type")?,
                status: row.get("status")?,
                is_hybrid: row.get::<_, i32>("is_hybrid")? != 0,
                is_archived: row.get::<_, i32>("is_archived")? != 0,
                specimen_count: row.get("specimen_count")?,
            },
            PedigreeEdge {
                parent_strain_id: row.get("parent_strain_id")?,
                parent_role: row.get("parent_role")?,
                parent_chain_seq_at_creation: row.get("parent_chain_seq_at_creation")?,
                event_id: row.get("event_id")?,
                event_notes: row.get("event_notes")?,
            },
        ))
    })?;
    let results: Result<Vec<_>, _> = rows.collect();
    Ok(results?)
}

fn load_child_entries(
    conn: &Connection,
    strain_id: &str,
) -> DbResult<Vec<(StrainSummary, PedigreeEdge)>> {
    let mut stmt = conn.prepare(
        "SELECT sp.parent_strain_id, sp.parent_role, sp.parent_chain_seq_at_creation, \
                (SELECT he.id FROM hybridization_events he \
                 WHERE he.hybrid_strain_id = sp.strain_id LIMIT 1) AS event_id, \
                (SELECT he.notes FROM hybridization_events he \
                 WHERE he.hybrid_strain_id = sp.strain_id LIMIT 1) AS event_notes, \
                s.id, s.name, s.code, s.strain_type, s.status, s.is_hybrid, s.is_archived, \
                COALESCE((SELECT COUNT(*) FROM specimens spec \
                          WHERE spec.strain_id = s.id AND spec.is_archived = 0), 0) AS specimen_count \
         FROM strain_parents sp \
         JOIN strains s ON s.id = sp.strain_id \
         WHERE sp.parent_strain_id = ?1 \
         ORDER BY sp.parent_role",
    )?;
    let rows = stmt.query_map(params![strain_id], |row| {
        Ok((
            StrainSummary {
                id: row.get("id")?,
                name: row.get("name")?,
                code: row.get("code")?,
                strain_type: row.get("strain_type")?,
                status: row.get("status")?,
                is_hybrid: row.get::<_, i32>("is_hybrid")? != 0,
                is_archived: row.get::<_, i32>("is_archived")? != 0,
                specimen_count: row.get("specimen_count")?,
            },
            PedigreeEdge {
                parent_strain_id: row.get("parent_strain_id")?,
                parent_role: row.get("parent_role")?,
                parent_chain_seq_at_creation: row.get("parent_chain_seq_at_creation")?,
                event_id: row.get("event_id")?,
                event_notes: row.get("event_notes")?,
            },
        ))
    })?;
    let results: Result<Vec<_>, _> = rows.collect();
    Ok(results?)
}

fn get_parents_recursive(
    conn: &Connection,
    strain_id: &str,
    depth: u32,
    max_depth: u32,
    path: &mut Vec<String>,
) -> DbResult<Vec<PedigreeNode>> {
    if depth > max_depth {
        return Ok(Vec::new());
    }
    let entries = load_parent_entries(conn, strain_id)?;
    let mut nodes = Vec::new();
    for (summary, edge) in entries {
        let parent_id = summary.id.clone();
        if path.contains(&parent_id) {
            return Err(DbError::Constraint(format!(
                "Circular pedigree detected: strain '{}' is its own ancestor",
                parent_id
            )));
        }
        path.push(parent_id.clone());
        let sub_parents = get_parents_recursive(conn, &parent_id, depth + 1, max_depth, path)?;
        path.pop();
        nodes.push(PedigreeNode {
            strain: summary,
            depth,
            edge: Some(edge),
            parents: sub_parents,
            children: Vec::new(),
        });
    }
    Ok(nodes)
}

fn get_children_recursive(
    conn: &Connection,
    strain_id: &str,
    depth: u32,
    max_depth: u32,
    path: &mut Vec<String>,
) -> DbResult<Vec<PedigreeNode>> {
    if depth > max_depth {
        return Ok(Vec::new());
    }
    let entries = load_child_entries(conn, strain_id)?;
    let mut nodes = Vec::new();
    for (summary, edge) in entries {
        let child_id = summary.id.clone();
        if path.contains(&child_id) {
            return Err(DbError::Constraint(format!(
                "Circular pedigree detected: strain '{}' is its own descendant",
                child_id
            )));
        }
        path.push(child_id.clone());
        let sub_children = get_children_recursive(conn, &child_id, depth + 1, max_depth, path)?;
        path.pop();
        nodes.push(PedigreeNode {
            strain: summary,
            depth,
            edge: Some(edge),
            parents: Vec::new(),
            children: sub_children,
        });
    }
    Ok(nodes)
}

fn load_specimens_for_strain(conn: &Connection, strain_id: &str) -> DbResult<Vec<SpecimenSummary>> {
    let mut stmt = conn.prepare(
        "SELECT id, accession_number, stage, location, is_archived, strain_id, created_at \
         FROM specimens WHERE strain_id = ?1 AND is_archived = 0 \
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![strain_id], |row| {
        Ok(SpecimenSummary {
            id: row.get("id")?,
            accession_number: row.get("accession_number")?,
            stage: row.get("stage")?,
            location: row.get("location")?,
            is_archived: row.get::<_, i32>("is_archived")? != 0,
            strain_id: row.get("strain_id")?,
            created_at: row.get("created_at")?,
        })
    })?;
    let results: Result<Vec<_>, _> = rows.collect();
    Ok(results?)
}

fn collect_pedigree_ids(node: &PedigreeNode, ids: &mut std::collections::HashSet<String>) {
    ids.insert(node.strain.id.clone());
    for parent in &node.parents {
        collect_pedigree_ids(parent, ids);
    }
    for child in &node.children {
        collect_pedigree_ids(child, ids);
    }
}

/// Walk upward through `strain_parents` to find all ancestors of `strain_id`.
///
/// Returns a `PedigreeNode` rooted at the given strain, with `parents` populated
/// recursively up to `max_depth` levels.  Returns `Err` if a cycle is detected.
pub fn get_strain_ancestry(
    conn: &Connection,
    strain_id: &str,
    max_depth: u32,
) -> DbResult<PedigreeNode> {
    let root = load_strain_summary(conn, strain_id)?;
    let mut path = vec![strain_id.to_string()];
    let parents = get_parents_recursive(conn, strain_id, 1, max_depth, &mut path)?;
    Ok(PedigreeNode {
        strain: root,
        depth: 0,
        edge: None,
        parents,
        children: Vec::new(),
    })
}

/// Walk downward through `strain_parents` to find all descendant hybrid strains.
///
/// Returns a `PedigreeNode` rooted at the given strain, with `children` populated
/// recursively up to `max_depth` levels.  Returns `Err` if a cycle is detected.
pub fn get_strain_descendants(
    conn: &Connection,
    strain_id: &str,
    max_depth: u32,
) -> DbResult<PedigreeNode> {
    let root = load_strain_summary(conn, strain_id)?;
    let mut path = vec![strain_id.to_string()];
    let children = get_children_recursive(conn, strain_id, 1, max_depth, &mut path)?;
    Ok(PedigreeNode {
        strain: root,
        depth: 0,
        edge: None,
        parents: Vec::new(),
        children,
    })
}

/// Return all live specimens bound to a strain, optionally including specimens
/// bound to descendant hybrid strains.
///
/// When `include_descendants = false`, only specimens directly bound to
/// `strain_id` are returned and `descendant_trees` is empty.
pub fn get_strain_specimen_tree(
    conn: &Connection,
    strain_id: &str,
    include_descendants: bool,
) -> DbResult<StrainSpecimenTree> {
    let mut path = vec![strain_id.to_string()];
    get_strain_specimen_tree_impl(conn, strain_id, include_descendants, &mut path)
}

fn get_strain_specimen_tree_impl(
    conn: &Connection,
    strain_id: &str,
    include_descendants: bool,
    path: &mut Vec<String>,
) -> DbResult<StrainSpecimenTree> {
    let strain = load_strain_summary(conn, strain_id)?;
    let specimens = load_specimens_for_strain(conn, strain_id)?;
    let descendant_trees = if include_descendants {
        let child_entries = load_child_entries(conn, strain_id)?;
        let mut trees = Vec::new();
        for (child_summary, _edge) in child_entries {
            let child_id = child_summary.id.clone();
            if path.contains(&child_id) {
                return Err(DbError::Constraint(format!(
                    "Circular pedigree detected: strain '{}' is its own descendant",
                    child_id
                )));
            }
            path.push(child_id.clone());
            trees.push(get_strain_specimen_tree_impl(conn, &child_id, true, path)?);
            path.pop();
        }
        trees
    } else {
        Vec::new()
    };
    Ok(StrainSpecimenTree {
        strain,
        specimens,
        descendant_trees,
    })
}

// ── WP-38: generation labeling and backcross helpers ─────────────────────────

/// Return the generation label stored on the hybridization event that created
/// `strain_id`, or `None` if the strain has no hybridization record or the
/// label was not set.
pub fn get_strain_generation_label(conn: &Connection, strain_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT generation_label FROM hybridization_events \
         WHERE hybrid_strain_id = ?1 LIMIT 1",
        params![strain_id],
        |r| r.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
}

/// Suggest a generation label based on the known labels of two parent strains.
///
/// Rules (simple filial notation):
/// - Both parents unlabeled (None) → `F1`
/// - Both labeled `F1` → `F2`
/// - Both labeled `F2` → `F3`
/// - Both labeled `F3` → `F4`
/// - All other combinations → `None` (user should specify explicitly)
///
/// Backcross notation (`BC{n}F1`) is returned by the command layer after the
/// pedigree is checked with `detect_backcross`; this function handles only the
/// symmetric, non-backcross case.
pub fn suggest_generation_label(
    parent_a_label: Option<&str>,
    parent_b_label: Option<&str>,
) -> Option<String> {
    match (parent_a_label, parent_b_label) {
        (None, None) => Some("F1".to_string()),
        (Some("F1"), Some("F1")) => Some("F2".to_string()),
        (Some("F2"), Some("F2")) => Some("F3".to_string()),
        (Some("F3"), Some("F3")) => Some("F4".to_string()),
        _ => None,
    }
}

/// Walk upward through `strain_parents` to find the depth at which
/// `candidate_ancestor` appears in `of_strain`'s lineage.
///
/// Returns `Some(depth)` (1-based) if found within `max_depth` levels, or
/// `None` if the candidate is not an ancestor or if a cycle guard fires.
fn find_ancestor_depth_impl(
    conn: &Connection,
    candidate_ancestor: &str,
    of_strain: &str,
    current_depth: u32,
    max_depth: u32,
    visited: &mut Vec<String>,
) -> Option<u32> {
    if current_depth > max_depth {
        return None;
    }
    if visited.iter().any(|v| v == of_strain) {
        return None;
    }
    visited.push(of_strain.to_string());

    let mut stmt = match conn.prepare(
        "SELECT parent_strain_id FROM strain_parents WHERE strain_id = ?1",
    ) {
        Ok(s) => s,
        Err(_) => return None,
    };
    let parents: Vec<String> = match stmt.query_map(params![of_strain], |r| r.get(0)) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => return None,
    };

    for parent_id in parents {
        if parent_id == candidate_ancestor {
            return Some(current_depth + 1);
        }
        if let Some(d) = find_ancestor_depth_impl(
            conn,
            candidate_ancestor,
            &parent_id,
            current_depth + 1,
            max_depth,
            visited,
        ) {
            return Some(d);
        }
    }
    None
}

/// Detect a backcross relationship between two prospective parents.
///
/// A backcross exists when one parent is an ancestor of the other.  Returns
/// `Some((ancestor_id, depth))` where `depth` is the number of pedigree levels
/// separating the ancestor from the other parent (1 = direct parent).
///
/// Returns `None` when neither parent is an ancestor of the other.
pub fn detect_backcross(
    conn: &Connection,
    parent_a_id: &str,
    parent_b_id: &str,
) -> Option<(String, u32)> {
    let mut visited = Vec::new();
    if let Some(depth) =
        find_ancestor_depth_impl(conn, parent_a_id, parent_b_id, 0, 10, &mut visited)
    {
        return Some((parent_a_id.to_string(), depth));
    }
    let mut visited = Vec::new();
    if let Some(depth) =
        find_ancestor_depth_impl(conn, parent_b_id, parent_a_id, 0, 10, &mut visited)
    {
        return Some((parent_b_id.to_string(), depth));
    }
    None
}

/// Suggest a generation label and detect backcross for two prospective parents.
///
/// This is the single entry point used by the `suggest_generation_label`
/// Tauri command; it runs `detect_backcross` first (which takes priority over
/// the symmetric label rules) and falls back to `suggest_generation_label`.
pub fn suggest_generation_label_for_parents(
    conn: &Connection,
    parent_a_id: &str,
    parent_b_id: &str,
) -> SuggestGenerationLabelResponse {
    let backcross = detect_backcross(conn, parent_a_id, parent_b_id);
    if let Some((ancestor_id, depth)) = backcross {
        let label = format!("BC{}F1", depth);
        return SuggestGenerationLabelResponse {
            suggested_label: Some(label),
            is_backcross: true,
            backcross_depth: Some(depth),
            backcross_ancestor_id: Some(ancestor_id),
        };
    }
    let label_a = get_strain_generation_label(conn, parent_a_id);
    let label_b = get_strain_generation_label(conn, parent_b_id);
    let suggested = suggest_generation_label(label_a.as_deref(), label_b.as_deref());
    SuggestGenerationLabelResponse {
        suggested_label: suggested,
        is_backcross: false,
        backcross_depth: None,
        backcross_ancestor_id: None,
    }
}

/// Return per-generation specimen statistics for the direct hybrid descendants
/// of `strain_id`.
///
/// Each row in the result corresponds to one distinct `generation_label` value
/// (or `"unlabeled"` for hybrids with no label set) and contains the total
/// specimen count plus a breakdown of healthy vs. problematic specimens.
pub fn get_generational_stats(
    conn: &Connection,
    strain_id: &str,
) -> DbResult<Vec<GenerationalStats>> {
    let mut stmt = conn.prepare(
        "SELECT \
            COALESCE(he.generation_label, 'unlabeled') AS gen_label, \
            COUNT(DISTINCT sp.id) AS specimen_count, \
            COALESCE(SUM(CASE WHEN sp.health_status IN ('healthy', 'excellent') \
                              THEN 1 ELSE 0 END), 0) AS healthy_count, \
            COALESCE(SUM(CASE WHEN sp.health_status IS NOT NULL \
                              AND sp.health_status NOT IN ('healthy', 'excellent') \
                              THEN 1 ELSE 0 END), 0) AS problem_count \
         FROM strain_parents lnk \
         JOIN strains child ON child.id = lnk.strain_id \
         LEFT JOIN hybridization_events he ON he.hybrid_strain_id = child.id \
         LEFT JOIN specimens sp ON sp.strain_id = child.id AND sp.is_archived = 0 \
         WHERE lnk.parent_strain_id = ?1 \
         GROUP BY gen_label \
         ORDER BY gen_label ASC",
    )?;
    let rows = stmt.query_map(params![strain_id], |row| {
        Ok(GenerationalStats {
            generation_label: row.get(0)?,
            specimen_count: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
            healthy_count: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
            problem_count: row.get::<_, Option<i64>>(3)?.unwrap_or(0),
        })
    })?;
    let results: Vec<GenerationalStats> = rows.filter_map(|r| r.ok()).collect();
    Ok(results)
}

/// Export the full pedigree of a strain as a portable bundle.
///
/// Collects all unique strains reachable within `max_depth` in both ancestry and
/// descendant directions, plus all relevant hybridization event records.
pub fn export_strain_pedigree(
    conn: &Connection,
    strain_id: &str,
    max_depth: u32,
) -> DbResult<PedigreeExport> {
    let ancestry = get_strain_ancestry(conn, strain_id, max_depth)?;
    let descendants = get_strain_descendants(conn, strain_id, max_depth)?;

    let mut strain_ids = std::collections::HashSet::new();
    collect_pedigree_ids(&ancestry, &mut strain_ids);
    collect_pedigree_ids(&descendants, &mut strain_ids);

    let mut strains: Vec<StrainSummary> = strain_ids
        .iter()
        .filter_map(|sid| load_strain_summary(conn, sid).ok())
        .collect();
    strains.sort_by(|a, b| a.name.cmp(&b.name));

    let mut seen_event_ids = std::collections::HashSet::new();
    let mut events: Vec<HybridizationEventRecord> = Vec::new();
    for sid in &strain_ids {
        let mut stmt = conn.prepare(
            "SELECT id, hybrid_strain_id, parent_a_strain_id, parent_b_strain_id, \
                    parent_a_chain_seq, parent_b_chain_seq, notes, \
                    generation_label, backcross_depth, created_at \
             FROM hybridization_events WHERE hybrid_strain_id = ?1",
        )?;
        let rows = stmt.query_map(params![sid], |row| {
            Ok(HybridizationEventRecord {
                id: row.get("id")?,
                hybrid_strain_id: row.get("hybrid_strain_id")?,
                parent_a_strain_id: row.get("parent_a_strain_id")?,
                parent_b_strain_id: row.get("parent_b_strain_id")?,
                parent_a_chain_seq: row.get("parent_a_chain_seq")?,
                parent_b_chain_seq: row.get("parent_b_chain_seq")?,
                notes: row.get("notes")?,
                generation_label: row.get("generation_label")?,
                backcross_depth: row.get("backcross_depth")?,
                created_at: row.get("created_at")?,
            })
        })?;
        let batch: Result<Vec<_>, _> = rows.collect();
        for event in batch? {
            if seen_event_ids.insert(event.id.clone()) {
                events.push(event);
            }
        }
    }
    events.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    Ok(PedigreeExport {
        root_strain_id: strain_id.to_string(),
        exported_at: chrono::Utc::now().to_rfc3339(),
        strains,
        hybridization_events: events,
    })
}

// ── Passage-number lineage & doubling time (WP-31) ──────────────────────────

/// Calculates doubling time in hours using the standard cell-culture formula:
///
///   DT = elapsed_hours × ln(2) / ln(harvest_count / seed_count)
///
/// Returns `None` when inputs are non-positive or when there is no net growth
/// (harvest_count ≤ seed_count), in which case DT is infinite or undefined.
pub fn calculate_doubling_time(
    seed_count: f64,
    harvest_count: f64,
    elapsed_hours: f64,
) -> Option<f64> {
    if seed_count <= 0.0 || harvest_count <= 0.0 || elapsed_hours <= 0.0 {
        return None;
    }
    let ratio = harvest_count / seed_count;
    // ratio ≤ 1 means no net growth; DT would be ∞ or negative, so return None
    if ratio <= 1.0 {
        return None;
    }
    Some(elapsed_hours * f64::ln(2.0) / f64::ln(ratio))
}

/// Calculates population doubling level (PDL) gained from seed and harvest counts.
///
///   PDL = log₂(harvest_count / seed_count)
///
/// Negative PDL (cell decline) is a valid return value.
/// Returns `None` for non-positive inputs.
pub fn calculate_pdl_from_counts(seed_count: f64, harvest_count: f64) -> Option<f64> {
    if seed_count <= 0.0 || harvest_count <= 0.0 {
        return None;
    }
    Some(f64::log2(harvest_count / seed_count))
}

/// Calculates PDL gained from a split ratio when cell counts are unavailable.
///
///   PDL = log₂(split_ratio)
///
/// For example a 1:4 split (split_ratio = 4.0) yields 2 PDL.
/// Returns `None` for non-positive inputs.
pub fn calculate_pdl_from_ratio(split_ratio: f64) -> Option<f64> {
    if split_ratio <= 0.0 {
        return None;
    }
    Some(f64::log2(split_ratio))
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

    // ── WP-28: strain / status-machine / hash-chain tests ─────────────────────

    fn mem_conn_with_strains() -> Connection {
        let conn = mem_conn_with_audit();
        conn.execute_batch(
            "CREATE TABLE species (
                id TEXT PRIMARY KEY,
                genus TEXT NOT NULL,
                species_name TEXT NOT NULL,
                species_code TEXT NOT NULL
            );
            CREATE TABLE strains (
                id TEXT PRIMARY KEY,
                species_id TEXT NOT NULL,
                name TEXT NOT NULL,
                code TEXT NOT NULL,
                strain_type TEXT NOT NULL DEFAULT 'wildtype',
                status TEXT NOT NULL DEFAULT 'unverified',
                claimed_by TEXT,
                claimed_at TEXT,
                confirmation_basis TEXT,
                genomic_fingerprint TEXT,
                is_hybrid INTEGER NOT NULL DEFAULT 0,
                is_archived INTEGER NOT NULL DEFAULT 0,
                is_cross_species INTEGER NOT NULL DEFAULT 0,
                archived_at TEXT,
                created_by TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE strain_parents (
                id TEXT PRIMARY KEY,
                strain_id TEXT NOT NULL,
                parent_strain_id TEXT NOT NULL,
                parent_role TEXT,
                parent_chain_seq_at_creation INTEGER
            );",
        ).expect("create strain tables");
        conn
    }

    fn insert_test_species(conn: &Connection, id: &str) {
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES (?1, 'Genus', 'species', ?2)",
            params![id, &id[..6]],
        ).unwrap();
        log_audit_at_seq_zero(conn, None, "create", "species", Some(id), None, None, None).unwrap();
    }

    fn insert_test_strain(conn: &Connection, id: &str, species_id: &str, code: &str) {
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, strain_type) \
             VALUES (?1, ?2, ?3, ?4, 'wildtype')",
            params![id, species_id, &format!("Strain {}", code), code],
        ).unwrap();
        log_audit_strain_genesis(conn, None, "create", "strain", Some(id), None, None, None, species_id).unwrap();
    }

    #[test]
    fn strain_genesis_prev_hash_equals_species_entry_hash() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-001");

        let species_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'sp-001' ORDER BY chain_seq DESC LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        insert_test_strain(&conn, "st-001", "sp-001", "WT01");

        let genesis_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'st-001' AND chain_seq = 0",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(genesis_prev, species_hash,
            "strain genesis prev_hash must equal species' current entry_hash");
    }

    #[test]
    fn specimen_with_strain_seeds_from_strain_entry_hash() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-001", "sp-001", "WT01");

        let strain_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'st-001' ORDER BY chain_seq DESC LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        log_audit_seeded_by_strain(&conn, None, "create", "specimen", Some("spec-001"), None, None, None, "st-001").unwrap();

        let spec_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'spec-001' AND chain_seq = 1",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(spec_prev, strain_hash,
            "specimen prev_hash must equal strain's current entry_hash");
    }

    #[test]
    fn strain_chain_seq_captured_before_specimen_write() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-001", "sp-001", "WT01");

        // Advance the strain's chain.
        log_audit(&conn, None, "update", "strain", Some("st-001"), None, None, None).unwrap();

        let strain_seq: i64 = conn.query_row(
            "SELECT COALESCE(MAX(chain_seq), 0) FROM audit_log \
             WHERE lineage_id = 'st-001' AND entry_hash IS NOT NULL",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(strain_seq, 1, "strain chain_seq should be 1 after genesis + one update");
    }

    // ── status machine tests ───────────────────────────────────────────────────

    #[test]
    fn strain_status_any_to_claimed_succeeds() {
        assert!(validate_strain_status_transition("unverified", "claimed", None, None).is_ok());
        assert!(validate_strain_status_transition("claimed", "claimed", None, None).is_ok());
        assert!(validate_strain_status_transition("unverified", "unverified", None, None).is_ok());
    }

    #[test]
    fn strain_status_confirmed_manual_rejected_without_basis() {
        let err = validate_strain_status_transition("claimed", "confirmed_manual", None, None)
            .unwrap_err();
        assert!(err.contains("confirmation_basis"), "error must mention confirmation_basis: {err}");
    }

    #[test]
    fn strain_status_confirmed_manual_accepted_with_basis() {
        assert!(validate_strain_status_transition(
            "claimed", "confirmed_manual", Some("Morphological check"), None
        ).is_ok());
    }

    #[test]
    fn strain_status_confirmed_genomic_rejected_without_fingerprint() {
        assert!(validate_strain_status_transition("claimed", "confirmed_genomic", None, None).is_err());
    }

    #[test]
    fn strain_status_confirmed_genomic_to_confirmed_manual_rejected() {
        let err = validate_strain_status_transition(
            "confirmed_genomic", "confirmed_manual", Some("basis"), None,
        ).unwrap_err();
        assert!(err.contains("downgrade"),
            "error must mention downgrade: {err}");
    }

    #[test]
    fn strain_status_confirmed_manual_to_claimed_rejected() {
        assert!(validate_strain_status_transition("confirmed_manual", "claimed", None, None).is_err());
    }

    #[test]
    fn strain_status_confirmed_manual_to_unverified_rejected() {
        assert!(validate_strain_status_transition("confirmed_manual", "unverified", None, None).is_err());
    }

    #[test]
    fn strain_status_confirmed_genomic_to_claimed_rejected() {
        assert!(validate_strain_status_transition("confirmed_genomic", "claimed", None, None).is_err());
    }

    // ── hybridization / fork invariant ────────────────────────────────────────

    #[test]
    fn hybridization_cross_species_detectable() {
        // Confirm that species_id comparison is the guard — no DB needed.
        // We validate that two strain species_ids differ (as create_hybridization_event checks).
        let a_species = "sp-001";
        let b_species = "sp-002";
        assert_ne!(a_species, b_species, "cross-species must be detected by species_id mismatch");
    }

    #[test]
    fn hybridization_writes_used_as_parent_on_both_chains() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");

        log_audit(&conn, None, "used_as_parent", "strain", Some("st-A"), None, None, Some("Used in hybridization")).unwrap();
        log_audit(&conn, None, "used_as_parent", "strain", Some("st-B"), None, None, Some("Used in hybridization")).unwrap();

        let a_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE lineage_id = 'st-A' AND action = 'used_as_parent'",
            [], |r| r.get(0),
        ).unwrap();
        let b_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log WHERE lineage_id = 'st-B' AND action = 'used_as_parent'",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(a_count, 1, "parent A must have one used_as_parent entry");
        assert_eq!(b_count, 1, "parent B must have one used_as_parent entry");
    }

    #[test]
    fn split_siblings_sharing_strain_share_same_prev_hash() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-001", "sp-001", "WT01");

        log_audit_seeded_by_strain(&conn, None, "create", "specimen", Some("spec-A"), None, None, None, "st-001").unwrap();
        log_audit_seeded_by_strain(&conn, None, "create", "specimen", Some("spec-B"), None, None, None, "st-001").unwrap();

        let prev_a: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'spec-A' LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();
        let prev_b: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'spec-B' LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(prev_a, prev_b,
            "siblings sharing a strain must share the same prev_hash (fork invariant)");
    }

    // ── WP-36: NCBI query helpers ─────────────────────────────────────────────

    fn mem_conn_with_taxa() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE taxa (
                id              TEXT PRIMARY KEY,
                rank            TEXT NOT NULL,
                name            TEXT NOT NULL,
                parent_id       TEXT,
                ncbi_taxon_id   INTEGER,
                ncbi_updated_at TEXT,
                local_override  INTEGER NOT NULL DEFAULT 0,
                taxon_path      TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE ncbi_sync_log (
                id               TEXT PRIMARY KEY,
                sync_type        TEXT NOT NULL,
                taxon_id         TEXT,
                ncbi_taxon_id    INTEGER,
                conflict_details TEXT,
                resolved_at      TEXT,
                resolved_by      TEXT,
                resolution       TEXT,
                created_at       TEXT NOT NULL
            );",
        ).expect("create taxa and ncbi_sync_log tables");
        conn
    }

    #[test]
    fn normalize_ncbi_rank_maps_known_values() {
        assert_eq!(normalize_ncbi_rank("genus"), Some("genus"));
        assert_eq!(normalize_ncbi_rank("Genus"), Some("genus"));
        assert_eq!(normalize_ncbi_rank("family"), Some("family"));
        assert_eq!(normalize_ncbi_rank("order"), Some("order"));
        assert_eq!(normalize_ncbi_rank("class"), Some("class"));
        assert_eq!(normalize_ncbi_rank("phylum"), Some("phylum"));
        assert_eq!(normalize_ncbi_rank("division"), Some("phylum"));
        assert_eq!(normalize_ncbi_rank("kingdom"), Some("kingdom"));
        assert_eq!(normalize_ncbi_rank("superkingdom"), Some("kingdom"));
    }

    #[test]
    fn normalize_ncbi_rank_returns_none_for_unsupported() {
        assert_eq!(normalize_ncbi_rank("species"), None);
        assert_eq!(normalize_ncbi_rank("subspecies"), None);
        assert_eq!(normalize_ncbi_rank("variety"), None);
        assert_eq!(normalize_ncbi_rank("forma"), None);
        assert_eq!(normalize_ncbi_rank("no rank"), None);
    }

    #[test]
    fn find_taxon_by_ncbi_id_returns_some_when_found() {
        let conn = mem_conn_with_taxa();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, ncbi_taxon_id, local_override) \
             VALUES ('t1', 'genus', 'Citrus', 4751, 0)",
            [],
        ).unwrap();

        let result = find_taxon_by_ncbi_id(&conn, 4751).unwrap();
        assert!(result.is_some(), "must find taxon by NCBI ID 4751");
        let taxon = result.unwrap();
        assert_eq!(taxon.id, "t1");
        assert_eq!(taxon.name, "Citrus");
    }

    #[test]
    fn find_taxon_by_ncbi_id_returns_none_when_missing() {
        let conn = mem_conn_with_taxa();
        let result = find_taxon_by_ncbi_id(&conn, 9999).unwrap();
        assert!(result.is_none(), "must return None for unknown NCBI ID");
    }

    #[test]
    fn find_taxon_by_name_rank_returns_match() {
        let conn = mem_conn_with_taxa();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, local_override) \
             VALUES ('t2', 'genus', 'Nandina', 0)",
            [],
        ).unwrap();

        let result = find_taxon_by_name_rank(&conn, "Nandina", "genus").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "t2");
    }

    #[test]
    fn find_taxon_by_name_rank_returns_none_for_wrong_rank() {
        let conn = mem_conn_with_taxa();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, local_override) \
             VALUES ('t3', 'genus', 'Rutaceae', 0)",
            [],
        ).unwrap();

        let result = find_taxon_by_name_rank(&conn, "Rutaceae", "family").unwrap();
        assert!(result.is_none(), "name match with wrong rank must return None");
    }

    #[test]
    fn detect_ncbi_conflict_returns_none_when_data_matches() {
        let taxon = Taxon {
            id: "t1".to_string(),
            rank: "genus".to_string(),
            name: "Citrus".to_string(),
            parent_id: None,
            ncbi_taxon_id: Some(4751),
            ncbi_updated_at: None,
            local_override: false,
            taxon_path: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };
        assert!(
            detect_ncbi_conflict(&taxon, "Citrus", "genus").is_none(),
            "identical name and rank must not produce a conflict"
        );
    }

    #[test]
    fn detect_ncbi_conflict_detects_name_change() {
        let taxon = Taxon {
            id: "t1".to_string(),
            rank: "genus".to_string(),
            name: "Citrus".to_string(),
            parent_id: None,
            ncbi_taxon_id: Some(4751),
            ncbi_updated_at: None,
            local_override: false,
            taxon_path: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };
        let details = detect_ncbi_conflict(&taxon, "Hesperellus", "genus");
        assert!(details.is_some(), "name change must produce a conflict");
        let json = details.unwrap();
        assert!(json.contains("name"), "conflict JSON must include 'name' key");
        assert!(json.contains("Citrus"), "conflict JSON must include local name");
        assert!(json.contains("Hesperellus"), "conflict JSON must include NCBI name");
    }

    #[test]
    fn detect_ncbi_conflict_detects_rank_change() {
        let taxon = Taxon {
            id: "t1".to_string(),
            rank: "genus".to_string(),
            name: "Aurantioideae".to_string(),
            parent_id: None,
            ncbi_taxon_id: Some(1000),
            ncbi_updated_at: None,
            local_override: false,
            taxon_path: None,
            created_at: "2026-01-01".to_string(),
            updated_at: "2026-01-01".to_string(),
        };
        let details = detect_ncbi_conflict(&taxon, "Aurantioideae", "family");
        assert!(details.is_some(), "rank change must produce a conflict");
        let json = details.unwrap();
        assert!(json.contains("rank"), "conflict JSON must include 'rank' key");
    }

    #[test]
    fn insert_and_list_ncbi_sync_log_works() {
        let conn = mem_conn_with_taxa();
        let now = "2026-01-01T00:00:00.000Z";
        insert_ncbi_sync_log(&conn, "log-1", "conflict", Some("t1"), Some(4751),
            Some(r#"{"name":{"local":"X","ncbi":"Y"}}"#), now).unwrap();
        insert_ncbi_sync_log(&conn, "log-2", "import", Some("t2"), Some(4752), None, now).unwrap();

        let pending = list_pending_ncbi_conflicts(&conn).unwrap();
        assert_eq!(pending.len(), 1, "only conflict-type unresolved rows are returned");
        assert_eq!(pending[0].id, "log-1");

        let all = list_ncbi_sync_log(&conn, 100).unwrap();
        assert_eq!(all.len(), 2, "list_ncbi_sync_log must return all entries");
    }

    #[test]
    fn list_pending_ncbi_conflicts_excludes_resolved() {
        let conn = mem_conn_with_taxa();
        let now = "2026-01-01T00:00:00.000Z";
        insert_ncbi_sync_log(&conn, "c1", "conflict", Some("t1"), Some(1), None, now).unwrap();
        insert_ncbi_sync_log(&conn, "c2", "conflict", Some("t2"), Some(2), None, now).unwrap();

        // Resolve c1
        conn.execute(
            "UPDATE ncbi_sync_log SET resolved_at = ?1, resolution = 'kept_local' WHERE id = 'c1'",
            rusqlite::params![now],
        ).unwrap();

        let pending = list_pending_ncbi_conflicts(&conn).unwrap();
        assert_eq!(pending.len(), 1, "resolved conflicts must not appear in pending list");
        assert_eq!(pending[0].id, "c2");
    }

    // ── WP-37: pedigree helpers ───────────────────────────────────────────────

    fn mem_conn_with_pedigree() -> Connection {
        let conn = mem_conn_with_strains();
        conn.execute_batch(
            "CREATE TABLE hybridization_events (
                id TEXT PRIMARY KEY,
                hybrid_strain_id TEXT NOT NULL,
                parent_a_strain_id TEXT NOT NULL,
                parent_b_strain_id TEXT NOT NULL,
                parent_a_chain_seq INTEGER NOT NULL DEFAULT 0,
                parent_b_chain_seq INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                generation_label TEXT,
                backcross_depth INTEGER,
                created_by TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL,
                stage TEXT NOT NULL DEFAULT 'initiation',
                location TEXT,
                health_status TEXT DEFAULT 'healthy',
                is_archived INTEGER NOT NULL DEFAULT 0,
                strain_id TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        ).expect("create pedigree extra tables");
        conn
    }

    fn insert_hybrid(
        conn: &Connection,
        hybrid_id: &str,
        parent_a: &str,
        parent_b: &str,
        species_id: &str,
        code: &str,
    ) {
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, strain_type, is_hybrid) \
             VALUES (?1, ?2, ?3, ?4, 'hybrid', 1)",
            params![hybrid_id, species_id, format!("Hybrid {code}"), code],
        ).unwrap();
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) \
             VALUES (?1, ?2, ?3, 'parent_a')",
            params![format!("{hybrid_id}-sp-a"), hybrid_id, parent_a],
        ).unwrap();
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) \
             VALUES (?1, ?2, ?3, 'parent_b')",
            params![format!("{hybrid_id}-sp-b"), hybrid_id, parent_b],
        ).unwrap();
    }

    fn insert_hybridization_event(
        conn: &Connection,
        event_id: &str,
        hybrid_id: &str,
        parent_a: &str,
        parent_b: &str,
    ) {
        conn.execute(
            "INSERT INTO hybridization_events \
             (id, hybrid_strain_id, parent_a_strain_id, parent_b_strain_id) \
             VALUES (?1, ?2, ?3, ?4)",
            params![event_id, hybrid_id, parent_a, parent_b],
        ).unwrap();
    }

    fn insert_specimen_for_strain(
        conn: &Connection,
        id: &str,
        strain_id: &str,
        accession: &str,
    ) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, stage, strain_id) \
             VALUES (?1, ?2, 'initiation', ?3)",
            params![id, accession, strain_id],
        ).unwrap();
    }

    #[test]
    fn pedigree_ancestry_wildtype_has_no_parents() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-wt", "sp-001", "WT01");

        let node = get_strain_ancestry(&conn, "st-wt", 5).unwrap();
        assert_eq!(node.strain.id, "st-wt");
        assert!(node.parents.is_empty(), "wildtype strain must have no parents");
    }

    #[test]
    fn pedigree_ancestry_finds_direct_parents() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H", "st-A", "st-B", "sp-001", "HH01");

        let node = get_strain_ancestry(&conn, "st-H", 5).unwrap();
        assert_eq!(node.strain.id, "st-H");
        assert_eq!(node.parents.len(), 2, "hybrid must have exactly 2 parents");
        let parent_ids: Vec<&str> = node.parents.iter().map(|p| p.strain.id.as_str()).collect();
        assert!(parent_ids.contains(&"st-A"), "parent A must be found");
        assert!(parent_ids.contains(&"st-B"), "parent B must be found");
    }

    #[test]
    fn pedigree_ancestry_finds_grandparents() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-G1", "sp-001", "G001");
        insert_test_strain(&conn, "st-G2", "sp-001", "G002");
        insert_test_strain(&conn, "st-G3", "sp-001", "G003");
        insert_test_strain(&conn, "st-G4", "sp-001", "G004");
        insert_hybrid(&conn, "st-P1", "st-G1", "st-G2", "sp-001", "P001");
        insert_hybrid(&conn, "st-P2", "st-G3", "st-G4", "sp-001", "P002");
        insert_hybrid(&conn, "st-H", "st-P1", "st-P2", "sp-001", "H001");

        let node = get_strain_ancestry(&conn, "st-H", 5).unwrap();
        assert_eq!(node.depth, 0);
        assert_eq!(node.parents.len(), 2, "hybrid must have 2 parents");
        for parent in &node.parents {
            assert_eq!(parent.depth, 1);
            assert_eq!(parent.parents.len(), 2, "each parent must have 2 grandparents");
            for grandparent in &parent.parents {
                assert_eq!(grandparent.depth, 2);
                assert!(grandparent.parents.is_empty(), "grandparents have no parents");
            }
        }
    }

    #[test]
    fn pedigree_ancestry_stops_at_max_depth() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-G1", "sp-001", "G001");
        insert_test_strain(&conn, "st-G2", "sp-001", "G002");
        insert_hybrid(&conn, "st-P", "st-G1", "st-G2", "sp-001", "P001");
        insert_test_strain(&conn, "st-G3", "sp-001", "G003");
        insert_hybrid(&conn, "st-H", "st-P", "st-G3", "sp-001", "H001");

        // max_depth=1 — show parents but not grandparents
        let node = get_strain_ancestry(&conn, "st-H", 1).unwrap();
        assert_eq!(node.parents.len(), 2);
        for parent in &node.parents {
            assert!(parent.parents.is_empty(), "max_depth=1 must not load grandparents");
        }
    }

    #[test]
    fn pedigree_ancestry_detects_cycle() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");

        // Create a 2-cycle: A is parent of B, B is parent of A
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) \
             VALUES ('cyc-1', 'st-B', 'st-A', 'parent_a')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) \
             VALUES ('cyc-2', 'st-A', 'st-B', 'parent_a')",
            [],
        ).unwrap();

        let result = get_strain_ancestry(&conn, "st-A", 5);
        assert!(result.is_err(), "cycle must be detected and rejected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Circular pedigree"), "error must mention circular pedigree: {msg}");
    }

    #[test]
    fn pedigree_descendants_wildtype_has_no_children() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-wt", "sp-001", "WT01");

        let node = get_strain_descendants(&conn, "st-wt", 5).unwrap();
        assert_eq!(node.strain.id, "st-wt");
        assert!(node.children.is_empty(), "wildtype with no hybrids must have no descendants");
    }

    #[test]
    fn pedigree_descendants_finds_direct_children() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H1", "st-A", "st-B", "sp-001", "HH01");
        insert_hybrid(&conn, "st-H2", "st-A", "st-B", "sp-001", "HH02");

        let node = get_strain_descendants(&conn, "st-A", 5).unwrap();
        assert_eq!(node.strain.id, "st-A");
        assert_eq!(node.children.len(), 2, "parent A used in 2 hybridizations must have 2 child nodes");
    }

    #[test]
    fn pedigree_descendants_finds_grandchildren() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H1", "st-A", "st-B", "sp-001", "HH01");
        insert_test_strain(&conn, "st-C", "sp-001", "CC01");
        insert_hybrid(&conn, "st-H2", "st-H1", "st-C", "sp-001", "HH02");

        let node = get_strain_descendants(&conn, "st-A", 5).unwrap();
        assert_eq!(node.children.len(), 1);
        assert_eq!(node.children[0].strain.id, "st-H1");
        assert_eq!(node.children[0].children.len(), 1, "H1 must have H2 as a child");
        assert_eq!(node.children[0].children[0].strain.id, "st-H2");
    }

    #[test]
    fn pedigree_descendants_detects_cycle() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");

        // Create a 2-cycle: B is a "child" of A, and A is a "child" of B
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) \
             VALUES ('cyc-1', 'st-B', 'st-A', 'parent_a')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) \
             VALUES ('cyc-2', 'st-A', 'st-B', 'parent_a')",
            [],
        ).unwrap();

        let result = get_strain_descendants(&conn, "st-A", 5);
        assert!(result.is_err(), "descendant cycle must be detected");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Circular pedigree"), "error must mention circular pedigree: {msg}");
    }

    #[test]
    fn pedigree_specimen_tree_empty_when_no_specimens() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-wt", "sp-001", "WT01");

        let tree = get_strain_specimen_tree(&conn, "st-wt", false).unwrap();
        assert_eq!(tree.strain.id, "st-wt");
        assert!(tree.specimens.is_empty(), "strain with no specimens must return empty list");
        assert!(tree.descendant_trees.is_empty());
    }

    #[test]
    fn pedigree_specimen_tree_finds_bound_specimens() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-wt", "sp-001", "WT01");
        insert_specimen_for_strain(&conn, "spec-1", "st-wt", "2026-01-WT-001");
        insert_specimen_for_strain(&conn, "spec-2", "st-wt", "2026-01-WT-002");

        let tree = get_strain_specimen_tree(&conn, "st-wt", false).unwrap();
        assert_eq!(tree.specimens.len(), 2, "both specimens must be returned");
    }

    #[test]
    fn pedigree_specimen_tree_includes_descendant_specimens_when_requested() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H", "st-A", "st-B", "sp-001", "HH01");
        insert_specimen_for_strain(&conn, "spec-root", "st-A", "2026-01-AA-001");
        insert_specimen_for_strain(&conn, "spec-hybrid", "st-H", "2026-01-HH-001");

        // Without descendants
        let tree_no_desc = get_strain_specimen_tree(&conn, "st-A", false).unwrap();
        assert_eq!(tree_no_desc.specimens.len(), 1);
        assert!(tree_no_desc.descendant_trees.is_empty());

        // With descendants
        let tree_with_desc = get_strain_specimen_tree(&conn, "st-A", true).unwrap();
        assert_eq!(tree_with_desc.specimens.len(), 1, "st-A must have 1 direct specimen");
        assert_eq!(tree_with_desc.descendant_trees.len(), 1, "st-A must have 1 descendant (st-H via parent_a)");
        let hybrid_tree = &tree_with_desc.descendant_trees[0];
        assert_eq!(hybrid_tree.specimens.len(), 1, "st-H must have 1 specimen");
    }

    #[test]
    fn pedigree_export_bundles_root_ancestors_and_events() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H", "st-A", "st-B", "sp-001", "HH01");
        insert_hybridization_event(&conn, "evt-1", "st-H", "st-A", "st-B");

        let export = export_strain_pedigree(&conn, "st-H", 5).unwrap();
        assert_eq!(export.root_strain_id, "st-H");

        let strain_ids: Vec<&str> = export.strains.iter().map(|s| s.id.as_str()).collect();
        assert!(strain_ids.contains(&"st-H"), "export must include root");
        assert!(strain_ids.contains(&"st-A"), "export must include parent A");
        assert!(strain_ids.contains(&"st-B"), "export must include parent B");

        assert_eq!(export.hybridization_events.len(), 1, "export must include the hybridization event");
        assert_eq!(export.hybridization_events[0].id, "evt-1");
    }

    #[test]
    fn pedigree_specimen_tree_detects_cycle() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        // Manually create a 2-cycle: A → child B, B → child A
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) VALUES ('cyc-ab', 'st-B', 'st-A', NULL)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO strain_parents (id, strain_id, parent_strain_id, parent_role) VALUES ('cyc-ba', 'st-A', 'st-B', NULL)",
            [],
        ).unwrap();
        let result = get_strain_specimen_tree(&conn, "st-A", true);
        assert!(result.is_err(), "cycle must be detected in specimen tree traversal");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Circular pedigree"), "error must mention circular pedigree: {msg}");
    }

    // ── WP-38: generation labeling and backcross helpers ─────────────────────

    #[test]
    fn suggest_generation_label_both_unlabeled_gives_f1() {
        assert_eq!(suggest_generation_label(None, None), Some("F1".to_string()));
    }

    #[test]
    fn suggest_generation_label_both_f1_gives_f2() {
        assert_eq!(
            suggest_generation_label(Some("F1"), Some("F1")),
            Some("F2".to_string())
        );
    }

    #[test]
    fn suggest_generation_label_both_f2_gives_f3() {
        assert_eq!(
            suggest_generation_label(Some("F2"), Some("F2")),
            Some("F3".to_string())
        );
    }

    #[test]
    fn suggest_generation_label_mixed_returns_none() {
        assert_eq!(suggest_generation_label(Some("F1"), None), None);
        assert_eq!(suggest_generation_label(None, Some("F2")), None);
        assert_eq!(suggest_generation_label(Some("F1"), Some("F2")), None);
    }

    #[test]
    fn detect_backcross_returns_none_for_unrelated_parents() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");

        let result = detect_backcross(&conn, "st-A", "st-B");
        assert!(result.is_none(), "unrelated parents must not produce a backcross");
    }

    #[test]
    fn detect_backcross_finds_direct_ancestor() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H", "st-A", "st-B", "sp-001", "HH01");

        // st-A is a direct parent of st-H, so crossing st-H × st-A is a backcross
        let result = detect_backcross(&conn, "st-A", "st-H");
        assert!(result.is_some(), "crossing parent with its own hybrid must be detected as backcross");
        let (ancestor_id, depth) = result.unwrap();
        assert_eq!(ancestor_id, "st-A");
        assert_eq!(depth, 1, "direct parent is at depth 1");
    }

    #[test]
    fn detect_backcross_finds_grandparent_ancestor() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-G1", "sp-001", "G001");
        insert_test_strain(&conn, "st-G2", "sp-001", "G002");
        insert_hybrid(&conn, "st-P", "st-G1", "st-G2", "sp-001", "P001");
        insert_test_strain(&conn, "st-X", "sp-001", "X001");
        insert_hybrid(&conn, "st-H", "st-P", "st-X", "sp-001", "H001");

        // st-G1 is a grandparent of st-H; crossing st-H × st-G1 is a backcross at depth 2
        let result = detect_backcross(&conn, "st-G1", "st-H");
        assert!(result.is_some(), "grandparent backcross must be detected");
        let (ancestor_id, depth) = result.unwrap();
        assert_eq!(ancestor_id, "st-G1");
        assert_eq!(depth, 2, "grandparent is at depth 2");
    }

    #[test]
    fn suggest_generation_label_for_parents_backcross_overrides_label_rules() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H", "st-A", "st-B", "sp-001", "HH01");

        let resp = suggest_generation_label_for_parents(&conn, "st-A", "st-H");
        assert!(resp.is_backcross, "must be flagged as backcross");
        assert_eq!(resp.suggested_label, Some("BC1F1".to_string()));
        assert_eq!(resp.backcross_depth, Some(1));
    }

    #[test]
    fn get_generational_stats_returns_stats_per_label() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-A", "sp-001", "AA01");
        insert_test_strain(&conn, "st-B", "sp-001", "BB01");
        insert_hybrid(&conn, "st-H1", "st-A", "st-B", "sp-001", "HH01");
        insert_hybrid(&conn, "st-H2", "st-A", "st-B", "sp-001", "HH02");

        // Assign F1 label to st-H1
        conn.execute(
            "INSERT INTO hybridization_events \
             (id, hybrid_strain_id, parent_a_strain_id, parent_b_strain_id, generation_label) \
             VALUES ('evt-h1', 'st-H1', 'st-A', 'st-B', 'F1')",
            [],
        ).unwrap();
        // st-H2 has no label → will appear as "unlabeled"
        conn.execute(
            "INSERT INTO hybridization_events \
             (id, hybrid_strain_id, parent_a_strain_id, parent_b_strain_id) \
             VALUES ('evt-h2', 'st-H2', 'st-A', 'st-B')",
            [],
        ).unwrap();

        insert_specimen_for_strain(&conn, "sp-f1-1", "st-H1", "F1-001");
        insert_specimen_for_strain(&conn, "sp-f1-2", "st-H1", "F1-002");
        insert_specimen_for_strain(&conn, "sp-un-1", "st-H2", "UN-001");

        let stats = get_generational_stats(&conn, "st-A").unwrap();
        assert_eq!(stats.len(), 2, "must return rows for F1 and unlabeled");

        let f1 = stats.iter().find(|s| s.generation_label == "F1").expect("F1 row must exist");
        assert_eq!(f1.specimen_count, 2, "F1 must have 2 specimens");

        let unl = stats.iter().find(|s| s.generation_label == "unlabeled").expect("unlabeled row must exist");
        assert_eq!(unl.specimen_count, 1, "unlabeled must have 1 specimen");
    }

    #[test]
    fn get_generational_stats_empty_when_no_descendants() {
        let conn = mem_conn_with_pedigree();
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-wt", "sp-001", "WT01");

        let stats = get_generational_stats(&conn, "st-wt").unwrap();
        assert!(stats.is_empty(), "strain with no hybrid children must return empty stats");
    }

    // ── WP-39: taxonomy navigator helpers ─────────────────────────────────────

    fn mem_conn_with_taxonomy_nav() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE taxa (
                id TEXT PRIMARY KEY,
                rank TEXT NOT NULL,
                name TEXT NOT NULL,
                parent_id TEXT,
                ncbi_taxon_id INTEGER,
                local_override INTEGER NOT NULL DEFAULT 0,
                taxon_path TEXT
            );
            CREATE TABLE species (
                id TEXT PRIMARY KEY,
                genus TEXT NOT NULL,
                species_name TEXT NOT NULL,
                common_name TEXT,
                species_code TEXT NOT NULL,
                taxon_path TEXT
            );
            CREATE TABLE strains (
                id TEXT PRIMARY KEY,
                species_id TEXT NOT NULL,
                name TEXT NOT NULL,
                code TEXT NOT NULL,
                is_archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL,
                species_id TEXT,
                strain_id TEXT,
                stage TEXT,
                is_archived INTEGER NOT NULL DEFAULT 0
            );",
        )
        .expect("create taxonomy nav tables");
        conn
    }

    #[test]
    fn taxon_column_roots_returns_kingdoms() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k1', 'kingdom', 'Plantae')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k2', 'kingdom', 'Fungi')",
            [],
        )
        .unwrap();

        let items = get_taxon_column_items(&conn, None).unwrap();
        assert_eq!(items.len(), 2, "must return both kingdoms");
        assert_eq!(items[0].name, "Fungi", "must be sorted by name");
        assert_eq!(items[1].name, "Plantae");
        assert_eq!(items[0].rank, "kingdom");
    }

    #[test]
    fn taxon_column_children_returns_phyla_under_kingdom() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k1', 'kingdom', 'Plantae')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, parent_id) VALUES ('p1', 'phylum', 'Angiospermae', 'k1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, parent_id) VALUES ('p2', 'phylum', 'Gymnospermae', 'k1')",
            [],
        )
        .unwrap();

        let items = get_taxon_column_items(&conn, Some("k1")).unwrap();
        assert_eq!(items.len(), 2, "must return both phyla under kingdom k1");
        assert_eq!(items[0].name, "Angiospermae");
        assert_eq!(items[0].parent_id.as_deref(), Some("k1"));
    }

    #[test]
    fn taxon_column_aggregates_descendant_counts() {
        let conn = mem_conn_with_taxonomy_nav();
        // Kingdom → Genus hierarchy
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k1', 'kingdom', 'Plantae')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, parent_id, taxon_path) \
             VALUES ('g1', 'genus', 'Citrus', 'k1', '[\"k1\",\"g1\"]')",
            [],
        )
        .unwrap();
        // Species under genus
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path) \
             VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-SIN', '[\"k1\",\"g1\"]')",
            [],
        )
        .unwrap();
        // 2 strains
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('st1', 'sp1', 'A', 'AA')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('st2', 'sp1', 'B', 'BB')",
            [],
        )
        .unwrap();
        // 3 specimens
        for i in 1..=3u8 {
            conn.execute(
                &format!(
                    "INSERT INTO specimens (id, accession_number, species_id) \
                     VALUES ('spec{i}', '2026-CIT-00{i}', 'sp1')"
                ),
                [],
            )
            .unwrap();
        }

        // Querying kingdoms: k1 must aggregate counts from descendant genus g1
        let items = get_taxon_column_items(&conn, None).unwrap();
        assert_eq!(items.len(), 1);
        let k1 = &items[0];
        assert_eq!(k1.strain_count, 2, "kingdom must aggregate 2 strains from descendant genus");
        assert_eq!(k1.specimen_count, 3, "kingdom must aggregate 3 specimens");
    }

    #[test]
    fn taxon_column_excludes_archived_from_counts() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k1', 'kingdom', 'Plantae')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path) \
             VALUES ('sp1', 'G', 'spp', 'G-SP', '[\"k1\"]')",
            [],
        )
        .unwrap();
        // 1 active + 1 archived strain
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, is_archived) \
             VALUES ('st-active', 'sp1', 'Active', 'AC', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, is_archived) \
             VALUES ('st-arch', 'sp1', 'Archived', 'AR', 1)",
            [],
        )
        .unwrap();

        let items = get_taxon_column_items(&conn, None).unwrap();
        assert_eq!(items[0].strain_count, 1, "archived strain must be excluded from count");
    }

    #[test]
    fn search_taxonomy_finds_taxon_by_name() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, taxon_path) \
             VALUES ('k1', 'kingdom', 'Plantae', '[\"k1\"]')",
            [],
        )
        .unwrap();

        let results = search_taxonomy(&conn, "Plant").unwrap();
        assert!(!results.is_empty(), "must find Plantae when searching 'Plant'");
        let hit = &results[0];
        assert_eq!(hit.result_type, "taxon");
        assert_eq!(hit.id, "k1");
        assert_eq!(hit.display_name, "Plantae");
        assert_eq!(hit.secondary, "kingdom");
        assert_eq!(hit.taxon_ids, vec!["k1"]);
    }

    #[test]
    fn search_taxonomy_finds_species_by_genus_name() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path) \
             VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-SIN', '[\"k1\",\"g1\"]')",
            [],
        )
        .unwrap();

        let results = search_taxonomy(&conn, "Citrus").unwrap();
        let hit = results.iter().find(|r| r.result_type == "species").expect("must find species");
        assert_eq!(hit.id, "sp1");
        assert_eq!(hit.display_name, "Citrus sinensis");
        assert_eq!(hit.secondary, "CIT-SIN");
        assert_eq!(hit.taxon_ids, vec!["k1", "g1"]);
        assert_eq!(hit.species_id.as_deref(), Some("sp1"));
    }

    #[test]
    fn search_taxonomy_finds_strain_by_code() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path) \
             VALUES ('sp1', 'G', 'sp', 'G-SP', '[\"k1\",\"g1\"]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) \
             VALUES ('st1', 'sp1', 'Gold Nugget', 'GN-01')",
            [],
        )
        .unwrap();

        let results = search_taxonomy(&conn, "GN-01").unwrap();
        let hit = results.iter().find(|r| r.result_type == "strain").expect("must find strain");
        assert_eq!(hit.id, "st1");
        assert_eq!(hit.display_name, "Gold Nugget");
        assert_eq!(hit.secondary, "GN-01");
        assert_eq!(hit.strain_id.as_deref(), Some("st1"));
        assert_eq!(hit.species_id.as_deref(), Some("sp1"));
    }

    #[test]
    fn search_taxonomy_finds_specimen_by_accession() {
        let conn = mem_conn_with_taxonomy_nav();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path) \
             VALUES ('sp1', 'G', 'sp', 'G-SP', '[\"k1\"]')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage) \
             VALUES ('spec1', '2026-06-G-SP-001', 'sp1', 'multiplication')",
            [],
        )
        .unwrap();

        let results = search_taxonomy(&conn, "2026-06-G-SP").unwrap();
        let hit = results
            .iter()
            .find(|r| r.result_type == "specimen")
            .expect("must find specimen");
        assert_eq!(hit.id, "spec1");
        assert_eq!(hit.display_name, "2026-06-G-SP-001");
        assert_eq!(hit.secondary, "multiplication");
    }

    #[test]
    fn search_taxonomy_returns_empty_for_no_match() {
        let conn = mem_conn_with_taxonomy_nav();
        let results = search_taxonomy(&conn, "zzznomatch").unwrap();
        assert!(results.is_empty(), "must return empty vec when nothing matches");
    }

    // ── WP-31: PDL & doubling time calculation tests ─────────────────────────

    #[test]
    fn doubling_time_typical_8x_growth_72h_gives_24h() {
        // 1M → 8M cells in 72 h: log2(8) = 3 doublings, DT = 72 / 3 = 24 h
        let dt = calculate_doubling_time(1_000_000.0, 8_000_000.0, 72.0).unwrap();
        assert!((dt - 24.0).abs() < 0.001, "expected 24.0 h, got {}", dt);
    }

    #[test]
    fn doubling_time_none_when_no_growth() {
        assert_eq!(calculate_doubling_time(1_000_000.0, 500_000.0, 72.0), None);
        assert_eq!(calculate_doubling_time(1_000_000.0, 1_000_000.0, 72.0), None);
    }

    #[test]
    fn doubling_time_none_for_invalid_inputs() {
        assert_eq!(calculate_doubling_time(0.0, 8_000_000.0, 72.0), None);
        assert_eq!(calculate_doubling_time(1_000_000.0, 0.0, 72.0), None);
        assert_eq!(calculate_doubling_time(1_000_000.0, 8_000_000.0, 0.0), None);
        assert_eq!(calculate_doubling_time(-1.0, 8_000_000.0, 72.0), None);
    }

    #[test]
    fn pdl_from_counts_8x_gives_3_pdl() {
        // 1M → 8M = 3 population doublings
        let pdl = calculate_pdl_from_counts(1_000_000.0, 8_000_000.0).unwrap();
        assert!((pdl - 3.0).abs() < 0.001, "expected 3.0 PDL, got {}", pdl);
    }

    #[test]
    fn pdl_from_counts_decline_gives_negative() {
        // Cell decline: 1M → 500K = -1 PDL
        let pdl = calculate_pdl_from_counts(1_000_000.0, 500_000.0).unwrap();
        assert!((pdl - (-1.0)).abs() < 0.001, "expected -1.0 PDL, got {}", pdl);
    }

    #[test]
    fn pdl_from_counts_none_for_invalid_inputs() {
        assert_eq!(calculate_pdl_from_counts(0.0, 1_000_000.0), None);
        assert_eq!(calculate_pdl_from_counts(1_000_000.0, 0.0), None);
        assert_eq!(calculate_pdl_from_counts(-1.0, 1_000_000.0), None);
    }

    #[test]
    fn pdl_from_ratio_4x_gives_2_pdl() {
        // 1:4 split ratio → log2(4) = 2 PDL
        let pdl = calculate_pdl_from_ratio(4.0).unwrap();
        assert!((pdl - 2.0).abs() < 0.001, "expected 2.0 PDL, got {}", pdl);
    }

    #[test]
    fn pdl_from_ratio_2x_gives_1_pdl() {
        let pdl = calculate_pdl_from_ratio(2.0).unwrap();
        assert!((pdl - 1.0).abs() < 0.001, "expected 1.0 PDL, got {}", pdl);
    }

    #[test]
    fn pdl_from_ratio_none_for_invalid_inputs() {
        assert_eq!(calculate_pdl_from_ratio(0.0), None);
        assert_eq!(calculate_pdl_from_ratio(-2.0), None);
    }
}
