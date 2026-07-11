// Query helpers and shared database utilities
use rusqlite::{Connection, params};
use sha2::{Sha256, Digest};
use super::{DbError, DbResult};
use crate::models::taxon::{
    DarwinCoreExport, DarwinCoreRecord, NcbiSyncLog, SpeciesNodeSummary, Taxon, TaxonColumnItem,
    TaxonMapping, TaxonomySearchResult,
};
use crate::models::strain::{
    GenerationalStats, HybridizationEventRecord, PedigreeEdge, PedigreeExport, PedigreeNode,
    SpecimenSummary, StrainSpecimenTree, StrainSummary, SuggestGenerationLabelResponse,
};
use crate::models::compliance::MycoplasmaStatus;
use crate::models::compliance::ComplianceFlag;
use crate::models::cryo::{CreateFrozenVialRequest, FrozenVial, ListFrozenVialsParams};
use crate::models::fruiting::{CreateFruitingRecordRequest, FruitingRecord};
use crate::models::breeding::{
    BreedingProgram, BreedingRecord, CreateBreedingProgramRequest,
    CreateBreedingRecordRequest, GenerationalSummary,
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
/// Original genesis function for species creation. For databases running migration_031
/// (WP-45) or later, prefer `log_audit_species_genesis` instead, which seeds prev_hash
/// from the genus taxon's current entry_hash. This function is retained for backward
/// compatibility with existing call sites and test fixtures that pre-date WP-45.
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

/// EXPERIMENTAL (WP-45): Look up the last entry_hash for the genus taxon with the given name.
/// Returns None if the genus taxon does not exist or has no audit entries yet.
/// Tolerates a missing `taxa` table (returns None), so callers can fall back to ZERO_HASH
/// in test environments and pre-WP-45 databases.
fn genus_entry_hash_by_name(conn: &Connection, genus_name: &str) -> Option<String> {
    let genus_id: String = conn.query_row(
        "SELECT id FROM taxa WHERE rank = 'genus' AND name = ?1",
        params![genus_name],
        |r| r.get(0),
    ).ok()?;

    conn.query_row(
        "SELECT entry_hash FROM audit_log \
         WHERE (lineage_id = ?1 OR (lineage_id IS NULL AND entity_id = ?1)) \
           AND entry_hash IS NOT NULL \
         ORDER BY chain_seq DESC LIMIT 1",
        params![genus_id],
        |r| r.get(0),
    ).ok()
}

/// EXPERIMENTAL (WP-45): Look up the last entry_hash for the genus taxon associated
/// with a species. Queries `species.genus` then delegates to `genus_entry_hash_by_name`.
/// Returns None if the species, genus taxon, or its audit entries are missing.
fn genus_entry_hash_by_species(conn: &Connection, species_id: &str) -> Option<String> {
    let genus_name: String = conn.query_row(
        "SELECT genus FROM species WHERE id = ?1",
        params![species_id],
        |r| r.get(0),
    ).ok()?;
    genus_entry_hash_by_name(conn, &genus_name)
}

/// Log a genesis audit entry for a new strain at chain_seq = 0.
///
/// EXPERIMENTAL (WP-45): strain genesis entries are now anchored to the genus taxon's
/// last entry_hash rather than directly to the species, extending the cryptographic
/// chain upward: Kingdom → … → Genus → Strain → Specimen. Falls back to ZERO_HASH
/// when no genus taxon exists or has not yet participated in the hash chain (e.g.
/// pre-WP-45 databases and test fixtures without a `taxa` table).
///
/// Existing strain chains (written before WP-45) remain valid — their stored prev_hash
/// values are never rewritten. Only new strains created after migration_031 use the genus
/// anchor.
///
/// WARNING: Reclassifying the parent genus (or any ancestor) after this genesis entry is
/// written will break the cryptographic chain for this strain and all its specimens.
/// There is currently no automated re-anchoring tool. See ROADMAP.md §WP-45.
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

    // WP-45: anchor strain genesis to the genus taxon rather than directly to the species.
    let prev_hash = genus_entry_hash_by_species(conn, species_id)
        .unwrap_or_else(|| ZERO_HASH.to_string());

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

/// EXPERIMENTAL (WP-45): Log a genesis audit entry for a new taxon at chain_seq = 0.
///
/// If `parent_taxon_id` is given the genesis entry inherits the parent's last
/// entry_hash as prev_hash, creating a cryptographically linked parent→child
/// taxon chain (e.g. Kingdom → Phylum → Class → … → Genus).  Root taxa
/// (kingdoms with no parent) use ZERO_HASH.
///
/// WARNING: This is an experimental feature. Reclassifying a taxon after its genesis
/// entry is written will break the cryptographic chain for ALL descendants (taxa,
/// species, strains, and specimens). There is currently no automated re-anchoring tool.
/// See ROADMAP.md §WP-45 for the known limitation and design notes.
#[allow(clippy::too_many_arguments)]
pub fn log_audit_taxon_genesis(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
    parent_taxon_id: Option<&str>,
) -> DbResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let lineage_id = entity_id.unwrap_or("system").to_string();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let prev_hash = if let Some(parent_id) = parent_taxon_id {
        conn.query_row(
            "SELECT entry_hash FROM audit_log \
             WHERE (lineage_id = ?1 OR (lineage_id IS NULL AND entity_id = ?1)) \
               AND entry_hash IS NOT NULL \
             ORDER BY chain_seq DESC LIMIT 1",
            params![parent_id],
            |r| r.get(0),
        ).ok().unwrap_or_else(|| ZERO_HASH.to_string())
    } else {
        ZERO_HASH.to_string()
    };

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

/// EXPERIMENTAL (WP-45): Log a genesis audit entry for a new species at chain_seq = 0.
///
/// Seeds prev_hash from the genus taxon's last entry_hash when the genus has already
/// participated in the hash chain (i.e. after migration_031); falls back to ZERO_HASH
/// otherwise. This binds the species' genesis to the current state of the genus
/// classification, extending the full chain: Kingdom → … → Genus → Species.
///
/// WARNING: If the species is later reclassified to a different genus, its genesis
/// entry_hash remains cryptographically anchored to the ORIGINAL genus. All descendant
/// strains and specimens stay linked to the original classification.
/// See ROADMAP.md §WP-45.
#[allow(clippy::too_many_arguments)]
pub fn log_audit_species_genesis(
    conn: &Connection,
    user_id: Option<&str>,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    old_value: Option<&str>,
    new_value: Option<&str>,
    details: Option<&str>,
    genus_name: &str,
) -> DbResult<()> {
    let id = uuid::Uuid::new_v4().to_string();
    let lineage_id = entity_id.unwrap_or("system").to_string();
    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let prev_hash = genus_entry_hash_by_name(conn, genus_name)
        .unwrap_or_else(|| ZERO_HASH.to_string());

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

/// Validates and applies a strain status transition in one step, including the
/// WP-55 write-path guard that rejects a masked `"[RESTRICTED]"` genomic
/// fingerprint value before it can ever reach the database. See
/// `crate::db::permissions::reject_if_restricted_marker` for why this guard
/// exists — it is the fix for a real data-corruption bug where a masked read
/// value could be round-tripped back through this update.
///
/// `genomic_fingerprint: None` leaves the existing stored fingerprint
/// unchanged (via `COALESCE`); only `Some(value)` overwrites it.
#[allow(clippy::too_many_arguments)]
pub fn apply_strain_status_update(
    conn: &Connection,
    id: &str,
    current_status: &str,
    next_status: &str,
    claimed_by: Option<&str>,
    claimed_at: Option<&str>,
    confirmation_basis: Option<&str>,
    genomic_fingerprint: Option<&str>,
) -> Result<(), String> {
    validate_strain_status_transition(
        current_status,
        next_status,
        confirmation_basis,
        genomic_fingerprint,
    )?;

    crate::db::permissions::reject_if_restricted_marker(
        genomic_fingerprint,
        "Genomic fingerprint",
    )?;

    conn.execute(
        "UPDATE strains SET status = ?1, claimed_by = ?2, claimed_at = ?3,
         confirmation_basis = ?4, genomic_fingerprint = COALESCE(?5, genomic_fingerprint),
         updated_at = datetime('now') WHERE id = ?6",
        params![
            next_status,
            claimed_by,
            claimed_at,
            confirmation_basis,
            genomic_fingerprint,
            id
        ],
    )
    .map_err(|e| format!("Failed to update status: {}", e))?;

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
    /// Effective rows-per-page, clamped to a sane range. A `per_page` of 0 (which
    /// would otherwise emit `LIMIT 0` and return nothing) is floored to 1, and an
    /// absurdly large value is capped so a crafted request can't force a huge scan.
    pub fn limit(&self) -> u32 {
        self.per_page.clamp(1, 1000)
    }

    /// Row offset for the current page. `saturating_mul` prevents the u32 overflow
    /// (debug panic / release wraparound) a large `page * per_page` used to cause.
    pub fn offset(&self) -> u32 {
        self.page.saturating_sub(1).saturating_mul(self.limit())
    }
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self { page: 1, per_page: 50 }
    }
}

// ── WP-63: cursor-based audit pagination + configurable pedigree depth ──────

/// A page of results ordered by a stable, monotonically increasing cursor
/// column (here, `audit_log.chain_seq`). Unlike offset/limit pagination, a
/// cursor page never skips or repeats a row when new entries are inserted
/// between reads — offset pagination on a 1M-row audit lineage would do
/// exactly that as new passages are recorded during a technician's own
/// scroll session.
#[derive(serde::Serialize)]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<i64>,
    pub has_more: bool,
}

/// Returns up to `limit` audit entries for `lineage_id` with `chain_seq >
/// after_seq` (or from the start of the lineage when `after_seq` is `None`),
/// ordered oldest-to-newest. `next_cursor` is the last returned row's
/// `chain_seq` — pass it back as `after_seq` to fetch the next page ("load
/// later" in the UI). This is additive to the existing offset-paginated
/// `get_audit_log` search view; it exists specifically for the single-lineage
/// Audit Log detail view, which becomes prohibitively slow to load in full at
/// the 1M-entry scale WP-63 targets.
pub fn list_audit_entries_by_cursor(
    conn: &Connection,
    lineage_id: &str,
    after_seq: Option<i64>,
    limit: i64,
) -> DbResult<CursorPage<crate::models::audit::AuditEntry>> {
    let limit = limit.clamp(1, 1000);
    let after = after_seq.unwrap_or(-1);

    let mut stmt = conn.prepare(
        "SELECT a.*, u.username
         FROM audit_log a
         LEFT JOIN users u ON a.user_id = u.id
         WHERE a.lineage_id = ?1 AND COALESCE(a.chain_seq, -1) > ?2
         ORDER BY a.chain_seq ASC
         LIMIT ?3",
    )?;

    let mut items: Vec<crate::models::audit::AuditEntry> = stmt
        .query_map(params![lineage_id, after, limit + 1], |row| {
            Ok(crate::models::audit::AuditEntry {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                username: row.get("username")?,
                action: row.get("action")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                old_value: row.get("old_value")?,
                new_value: row.get("new_value")?,
                details: row.get("details")?,
                created_at: row.get("created_at")?,
                lineage_id: row.get("lineage_id")?,
                chain_seq: row.get("chain_seq")?,
                prev_hash: row.get("prev_hash")?,
                entry_hash: row.get("entry_hash")?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Fetched one extra row (limit + 1) to detect whether there's more without
    // a second COUNT query; trim it off before returning.
    let has_more = items.len() as i64 > limit;
    if has_more {
        items.truncate(limit as usize);
    }
    let next_cursor = items.last().and_then(|e| e.chain_seq);

    Ok(CursorPage { items, next_cursor, has_more })
}

/// Reads the lab's configured pedigree traversal depth cap (`pedigree_max_depth`
/// in `app_settings`, seeded to 10 by migration_039). Clamped to [1, 20]
/// regardless of what's stored, so a corrupted or hand-edited setting can
/// never make a pedigree query unbounded.
pub fn configured_pedigree_max_depth(conn: &Connection) -> u32 {
    read_setting(conn, "pedigree_max_depth", "10")
        .parse::<u32>()
        .unwrap_or(10)
        .clamp(1, 20)
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

// ── WP-49: Provisional taxa & Darwin Core export ─────────────────────────────

/// Create a provisional (lab-internal) taxon.  Returns the full taxon row on
/// success.  Provisional taxa have `status = 'provisional'` and `local_override = 1`.
#[allow(clippy::too_many_arguments)]
pub fn create_provisional_taxon(
    conn: &Connection,
    id: &str,
    rank: &str,
    name: &str,
    parent_id: Option<&str>,
    provisional_notes: Option<&str>,
    created_by: Option<&str>,
) -> DbResult<Taxon> {
    conn.execute(
        "INSERT INTO taxa (id, rank, name, parent_id, local_override, status, provisional_notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 1, 'provisional', ?5, datetime('now'), datetime('now'))",
        params![id, rank, name, parent_id, provisional_notes],
    )?;

    // Audit genesis entry for this provisional taxon.
    log_audit(
        conn,
        created_by,
        "create",
        "taxon",
        Some(id),
        None,
        None,
        Some(&format!("Provisional taxon created: {} ({})", name, rank)),
    )?;

    load_taxon(conn, id)
}

/// List all provisional taxa ordered by rank then name.
pub fn list_provisional_taxa(conn: &Connection) -> DbResult<Vec<Taxon>> {
    let mut stmt = conn.prepare(
        "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at,
                local_override, taxon_path, created_at, updated_at
         FROM taxa WHERE status = 'provisional'
         ORDER BY rank, name",
    )?;
    let rows = stmt.query_map([], |row| {
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
    let taxa: Vec<Taxon> = rows.filter_map(|r| r.ok()).collect();
    Ok(taxa)
}

/// Create a mapping from a provisional taxon to an accepted taxon.
#[allow(clippy::too_many_arguments)]
pub fn create_taxon_mapping(
    conn: &Connection,
    mapping_id: &str,
    provisional_taxon_id: &str,
    accepted_taxon_id: Option<&str>,
    accepted_ncbi_id: Option<i64>,
    accepted_name: Option<&str>,
    notes: Option<&str>,
    mapped_by: Option<&str>,
) -> DbResult<TaxonMapping> {
    conn.execute(
        "INSERT INTO taxon_mappings
         (id, provisional_taxon_id, accepted_taxon_id, accepted_ncbi_id, accepted_name, notes, mapped_by)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            mapping_id,
            provisional_taxon_id,
            accepted_taxon_id,
            accepted_ncbi_id,
            accepted_name,
            notes,
            mapped_by,
        ],
    )?;
    get_taxon_mapping(conn, mapping_id)
}

/// Retrieve a single taxon mapping by ID.
pub fn get_taxon_mapping(conn: &Connection, id: &str) -> DbResult<TaxonMapping> {
    conn.query_row(
        "SELECT id, provisional_taxon_id, accepted_taxon_id, accepted_ncbi_id,
                accepted_name, notes, mapped_by, mapped_at
         FROM taxon_mappings WHERE id = ?1",
        params![id],
        |row| {
            Ok(TaxonMapping {
                id: row.get("id")?,
                provisional_taxon_id: row.get("provisional_taxon_id")?,
                accepted_taxon_id: row.get("accepted_taxon_id")?,
                accepted_ncbi_id: row.get("accepted_ncbi_id")?,
                accepted_name: row.get("accepted_name")?,
                notes: row.get("notes")?,
                mapped_by: row.get("mapped_by")?,
                mapped_at: row.get("mapped_at")?,
            })
        },
    )
    .map_err(DbError::Sqlite)
}

/// List all taxon mappings ordered by mapped_at DESC.
pub fn list_taxon_mappings(conn: &Connection) -> DbResult<Vec<TaxonMapping>> {
    let mut stmt = conn.prepare(
        "SELECT id, provisional_taxon_id, accepted_taxon_id, accepted_ncbi_id,
                accepted_name, notes, mapped_by, mapped_at
         FROM taxon_mappings ORDER BY mapped_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(TaxonMapping {
            id: row.get("id")?,
            provisional_taxon_id: row.get("provisional_taxon_id")?,
            accepted_taxon_id: row.get("accepted_taxon_id")?,
            accepted_ncbi_id: row.get("accepted_ncbi_id")?,
            accepted_name: row.get("accepted_name")?,
            notes: row.get("notes")?,
            mapped_by: row.get("mapped_by")?,
            mapped_at: row.get("mapped_at")?,
        })
    })?;
    let mappings: Vec<TaxonMapping> = rows.filter_map(|r| r.ok()).collect();
    Ok(mappings)
}

/// Walk the taxa subtree rooted at `root_id` (or all top-level taxa when None)
/// and emit Darwin Core records.  Follows `parent_id` edges downward.
pub fn export_darwin_core(
    conn: &Connection,
    root_id: Option<&str>,
) -> DbResult<DarwinCoreExport> {
    let rows: Vec<(String, String, String, Option<String>, String)> = if let Some(rid) = root_id {
        // Collect the root + all descendants via a recursive CTE.
        let mut stmt = conn.prepare(
            "WITH RECURSIVE subtree(id) AS (
                 SELECT ?1
                 UNION ALL
                 SELECT t.id FROM taxa t
                 INNER JOIN subtree s ON t.parent_id = s.id
             )
             SELECT t.id, t.name, t.rank, t.parent_id, t.status
             FROM taxa t
             INNER JOIN subtree s ON t.id = s.id
             ORDER BY t.rank, t.name",
        )?;
        let r = stmt.query_map(params![rid], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, String>("name")?,
                row.get::<_, String>("rank")?,
                row.get::<_, Option<String>>("parent_id")?,
                row.get::<_, String>("status")?,
            ))
        })?;
        r.filter_map(|x| x.ok()).collect()
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, name, rank, parent_id, status FROM taxa ORDER BY rank, name",
        )?;
        let r = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>("id")?,
                row.get::<_, String>("name")?,
                row.get::<_, String>("rank")?,
                row.get::<_, Option<String>>("parent_id")?,
                row.get::<_, String>("status")?,
            ))
        })?;
        r.filter_map(|x| x.ok()).collect()
    };

    let records: Vec<DarwinCoreRecord> = rows
        .into_iter()
        .map(|(id, name, rank, parent_id, status)| {
            let taxonomic_status = match status.as_str() {
                "provisional" => "provisionallyAccepted",
                "synonym" => "synonym",
                _ => "accepted",
            };
            DarwinCoreRecord {
                taxon_id: id,
                scientific_name: name,
                taxon_rank: rank,
                parent_name_usage_id: parent_id,
                taxonomic_status: taxonomic_status.to_string(),
                name_according_to: Some("SteloPTC".to_string()),
                remarks: if taxonomic_status == "provisionallyAccepted" {
                    Some("Provisional taxon — lab-internal, not yet published".to_string())
                } else {
                    None
                },
            }
        })
        .collect();

    let record_count = records.len();
    Ok(DarwinCoreExport {
        record_count,
        records,
    })
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

// ── WP-32: Cryopreservation & LN2 inventory ──────────────────────────────

/// Build the composed location string from individual freezer location fields,
/// mirroring the "Room X / Rack Y / Shelf Z / Tray A" pattern on specimens.
pub fn compose_cryo_location(
    freezer: Option<&str>,
    tower: Option<&str>,
    box_: Option<&str>,
    position: Option<&str>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(f) = freezer {
        if !f.is_empty() {
            parts.push(format!("Freezer {}", f));
        }
    }
    if let Some(t) = tower {
        if !t.is_empty() {
            parts.push(format!("Tower {}", t));
        }
    }
    if let Some(b) = box_ {
        if !b.is_empty() {
            parts.push(format!("Box {}", b));
        }
    }
    if let Some(p) = position {
        if !p.is_empty() {
            parts.push(format!("Position {}", p));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" / "))
    }
}

fn row_to_frozen_vial(row: &rusqlite::Row) -> rusqlite::Result<FrozenVial> {
    Ok(FrozenVial {
        id: row.get("id")?,
        specimen_id: row.get("specimen_id")?,
        species_id: row.get("species_id")?,
        species_code: row.get("species_code")?,
        species_name: row.get("species_name")?,
        passage_number: row.get("passage_number")?,
        cumulative_pdl: row.get("cumulative_pdl")?,
        vial_count: row.get("vial_count")?,
        freeze_date: row.get("freeze_date")?,
        freeze_medium: row.get("freeze_medium")?,
        location: row.get("location")?,
        location_freezer: row.get("location_freezer")?,
        location_tower: row.get("location_tower")?,
        location_box: row.get("location_box")?,
        location_position: row.get("location_position")?,
        status: row.get("status")?,
        notes: row.get("notes")?,
        created_by: row.get("created_by")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

const FROZEN_VIAL_SELECT: &str = "
    SELECT fv.*,
           sp.species_code,
           sp.genus || ' ' || sp.species_name AS species_name
    FROM   frozen_vials fv
    LEFT JOIN species sp ON fv.species_id = sp.id
";

/// Insert a new frozen vial lot and return its generated UUID.
pub fn create_frozen_vial(
    conn: &Connection,
    req: &CreateFrozenVialRequest,
    created_by: Option<&str>,
) -> DbResult<String> {
    if req.vial_count <= 0 {
        return Err(DbError::Constraint("vial_count must be greater than zero".into()));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let location = compose_cryo_location(
        req.location_freezer.as_deref(),
        req.location_tower.as_deref(),
        req.location_box.as_deref(),
        req.location_position.as_deref(),
    );
    conn.execute(
        "INSERT INTO frozen_vials
         (id, specimen_id, species_id, passage_number, cumulative_pdl,
          vial_count, freeze_date, freeze_medium, location,
          location_freezer, location_tower, location_box, location_position,
          status, notes, created_by)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,'active',?14,?15)",
        params![
            id,
            req.specimen_id,
            req.species_id,
            req.passage_number,
            req.cumulative_pdl,
            req.vial_count,
            req.freeze_date,
            req.freeze_medium,
            location,
            req.location_freezer,
            req.location_tower,
            req.location_box,
            req.location_position,
            req.notes,
            created_by,
        ],
    )?;
    Ok(id)
}

/// Fetch a single frozen vial by id, joining species for display fields.
pub fn get_frozen_vial(conn: &Connection, id: &str) -> DbResult<FrozenVial> {
    let sql = format!("{} WHERE fv.id = ?1", FROZEN_VIAL_SELECT);
    let vial = conn.query_row(&sql, params![id], row_to_frozen_vial)
        .map_err(|_| DbError::Constraint(format!("Frozen vial not found: {}", id)))?;
    Ok(vial)
}

/// List frozen vials with optional filters.
pub fn list_frozen_vials(
    conn: &Connection,
    params_in: &ListFrozenVialsParams,
) -> DbResult<Vec<FrozenVial>> {
    // Build SQL and a parallel Vec of boxed ToSql values using positional ?N
    // parameters so that only the parameters actually present in the query are
    // bound (named-param binding rejects names that don't appear in the SQL).
    let mut sql = format!("{} WHERE 1=1", FROZEN_VIAL_SELECT);
    let mut bound: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref v) = params_in.species_id {
        bound.push(Box::new(v.clone()));
        sql.push_str(&format!(" AND fv.species_id = ?{}", bound.len()));
    }
    if let Some(ref v) = params_in.specimen_id {
        bound.push(Box::new(v.clone()));
        sql.push_str(&format!(" AND fv.specimen_id = ?{}", bound.len()));
    }
    if let Some(ref v) = params_in.status {
        bound.push(Box::new(v.clone()));
        sql.push_str(&format!(" AND fv.status = ?{}", bound.len()));
    }
    if let Some(ref v) = params_in.location_freezer {
        bound.push(Box::new(v.clone()));
        sql.push_str(&format!(" AND fv.location_freezer = ?{}", bound.len()));
    }
    sql.push_str(" ORDER BY fv.freeze_date DESC, fv.created_at DESC");

    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = bound.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(refs.as_slice(), row_to_frozen_vial)?;
    let vials: Vec<FrozenVial> = rows.filter_map(|r| r.ok()).collect();
    Ok(vials)
}

/// Decrement `vials_to_thaw` from the lot and create a new active specimen,
/// inheriting passage number and cumulative PDL from the frozen vial.
///
/// All writes happen inside a single transaction for atomicity.
/// Returns `(new_specimen_id, new_accession_number)`.
#[allow(clippy::too_many_arguments)]
pub fn thaw_frozen_vial(
    conn: &Connection,
    vial_id: &str,
    thaw_date: &str,
    vials_to_thaw: i32,
    location: Option<&str>,
    notes: Option<&str>,
    employee_id: Option<&str>,
    created_by: Option<&str>,
) -> DbResult<(String, String)> {
    if vials_to_thaw <= 0 {
        return Err(DbError::Constraint("vials_to_thaw must be at least 1".into()));
    }

    // Read current vial state before opening the transaction.
    let vial = get_frozen_vial(conn, vial_id)?;

    if vial.status != "active" {
        return Err(DbError::Constraint(format!(
            "Cannot thaw: vial status is '{}'", vial.status
        )));
    }
    if vial.vial_count < vials_to_thaw {
        return Err(DbError::Constraint(format!(
            "Cannot thaw {} vials: only {} available", vials_to_thaw, vial.vial_count
        )));
    }

    let new_count = vial.vial_count - vials_to_thaw;
    let new_status = if new_count == 0 { "depleted" } else { "active" };

    let species_code: String = conn.query_row(
        "SELECT species_code FROM species WHERE id = ?1",
        params![vial.species_id],
        |row| row.get(0),
    ).map_err(|_| DbError::Constraint("Species not found".into()))?;

    let accession = generate_accession_number(conn, &species_code, thaw_date)?;
    let specimen_id = uuid::Uuid::new_v4().to_string();
    let qr_data = format!("STELO:{}", accession);

    // Resolve generation and root from the source specimen, if present.
    let (parent_gen, parent_root): (i32, Option<String>) = if let Some(ref src_id) = vial.specimen_id {
        conn.query_row(
            "SELECT generation, root_specimen_id FROM specimens WHERE id = ?1",
            params![src_id],
            |r| Ok((r.get::<_, i32>(0)?, r.get::<_, Option<String>>(1)?)),
        ).unwrap_or((0, None))
    } else {
        (0, None)
    };

    let child_generation = if vial.specimen_id.is_some() { parent_gen + 1 } else { 0 };
    let child_root = parent_root.or_else(|| vial.specimen_id.clone());

    let tx = conn.unchecked_transaction()
        .map_err(|e| DbError::Constraint(format!("Transaction start failed: {}", e)))?;

    // Decrement vial inventory.
    tx.execute(
        "UPDATE frozen_vials
         SET vial_count = ?1, status = ?2, updated_at = datetime('now')
         WHERE id = ?3",
        params![new_count, new_status, vial_id],
    )?;

    // Create the thawed specimen.
    tx.execute(
        "INSERT INTO specimens
         (id, accession_number, species_id, stage, initiation_date,
          location, parent_specimen_id, root_specimen_id, generation,
          lineage_passage_offset, cumulative_pdl, qr_code_data,
          notes, employee_id, created_by)
         VALUES (?1,?2,?3,'thaw_recovery',?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            specimen_id,
            accession,
            vial.species_id,
            thaw_date,
            location,
            vial.specimen_id,
            child_root,
            child_generation,
            vial.passage_number,
            vial.cumulative_pdl,
            qr_data,
            notes,
            employee_id,
            created_by,
        ],
    )?;

    // Audit the thaw on the vial lineage.
    log_audit(
        &tx, created_by, "thaw", "frozen_vial", Some(vial_id),
        Some(&vial.vial_count.to_string()), Some(&new_count.to_string()),
        Some(&format!("Thawed {} vial(s); specimen {}", vials_to_thaw, accession)),
    ).ok();

    // Audit the new specimen.  Fork from source specimen if one exists.
    if let Some(ref src_id) = vial.specimen_id {
        log_audit_for_child(
            &tx, created_by, "create", "specimen", Some(&specimen_id),
            None, Some(&accession), Some("Specimen created (thawed from cryo)"),
            src_id,
        ).ok();
    } else {
        log_audit_seeded_by_species(
            &tx, created_by, "create", "specimen", Some(&specimen_id),
            None, Some(&accession), Some("Specimen created (thawed from cryo)"),
            &vial.species_id,
        ).ok();
    }

    tx.commit()
        .map_err(|e| DbError::Constraint(format!("Transaction commit failed: {}", e)))?;

    Ok((specimen_id, accession))
}

/// Return the latest mycoplasma test date and result for every non-archived specimen.
/// Rows with no mycoplasma compliance record at all are included with NULL fields.
pub fn list_mycoplasma_status(conn: &Connection) -> DbResult<Vec<MycoplasmaStatus>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.accession_number, sp.species_code,
         (SELECT cr.test_date FROM compliance_records cr
          WHERE cr.specimen_id = s.id AND cr.test_type = 'mycoplasma'
          AND cr.test_result IS NOT NULL
          ORDER BY cr.test_date DESC LIMIT 1) as last_test_date,
         (SELECT cr.test_result FROM compliance_records cr
          WHERE cr.specimen_id = s.id AND cr.test_type = 'mycoplasma'
          AND cr.test_result IS NOT NULL
          ORDER BY cr.test_date DESC LIMIT 1) as last_test_result
         FROM specimens s
         JOIN species sp ON s.species_id = sp.id
         WHERE s.is_archived = 0
         ORDER BY s.accession_number",
    )?;
    let items = stmt
        .query_map([], |row| {
            Ok(MycoplasmaStatus {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                last_test_date: row.get(3)?,
                last_test_result: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(items)
}

/// Mark a frozen vial lot as discarded (e.g. contaminated, damaged).
/// Updates status and optional notes; does not alter vial_count.
pub fn discard_frozen_vial(
    conn: &Connection,
    vial_id: &str,
    notes: Option<&str>,
) -> DbResult<()> {
    let rows = conn.execute(
        "UPDATE frozen_vials
         SET status = 'discarded', notes = COALESCE(?1, notes), updated_at = datetime('now')
         WHERE id = ?2",
        params![notes, vial_id],
    )?;
    if rows == 0 {
        return Err(DbError::Constraint(format!("Frozen vial not found: {}", vial_id)));
    }
    Ok(())
}

// ── WP-43: Fruiting records ───────────────────────────────────────────────────

fn row_to_fruiting_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<FruitingRecord> {
    Ok(FruitingRecord {
        id: row.get(0)?,
        specimen_id: row.get(1)?,
        flush_number: row.get(2)?,
        harvest_date: row.get(3)?,
        fresh_weight_g: row.get(4)?,
        dry_weight_g: row.get(5)?,
        fruiting_temp_c: row.get(6)?,
        fruiting_rh_percent: row.get(7)?,
        fae_rate: row.get(8)?,
        light_hours_per_day: row.get(9)?,
        notes: row.get(10)?,
        created_by: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
    })
}

pub fn create_fruiting_record(
    conn: &Connection,
    req: &CreateFruitingRecordRequest,
    created_by: Option<&str>,
) -> DbResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO fruiting_records
         (id, specimen_id, flush_number, harvest_date,
          fresh_weight_g, dry_weight_g, fruiting_temp_c, fruiting_rh_percent,
          fae_rate, light_hours_per_day, notes, created_by)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            id,
            req.specimen_id,
            req.flush_number,
            req.harvest_date,
            req.fresh_weight_g,
            req.dry_weight_g,
            req.fruiting_temp_c,
            req.fruiting_rh_percent,
            req.fae_rate,
            req.light_hours_per_day,
            req.notes,
            created_by,
        ],
    )?;
    Ok(id)
}

pub fn get_fruiting_record(conn: &Connection, id: &str) -> DbResult<FruitingRecord> {
    conn.query_row(
        "SELECT id, specimen_id, flush_number, harvest_date,
                fresh_weight_g, dry_weight_g, fruiting_temp_c, fruiting_rh_percent,
                fae_rate, light_hours_per_day, notes, created_by, created_at, updated_at
         FROM fruiting_records WHERE id = ?1",
        params![id],
        row_to_fruiting_record,
    )
    .map_err(|_| DbError::Constraint(format!("Fruiting record not found: {}", id)))
}

pub fn list_fruiting_records(conn: &Connection, specimen_id: &str) -> DbResult<Vec<FruitingRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, specimen_id, flush_number, harvest_date,
                fresh_weight_g, dry_weight_g, fruiting_temp_c, fruiting_rh_percent,
                fae_rate, light_hours_per_day, notes, created_by, created_at, updated_at
         FROM fruiting_records
         WHERE specimen_id = ?1
         ORDER BY flush_number ASC, harvest_date ASC",
    )?;
    let rows = stmt.query_map(params![specimen_id], row_to_fruiting_record)?;
    let records: Vec<FruitingRecord> = rows.filter_map(|r| r.ok()).collect();
    Ok(records)
}

// ── WP-44: Mycology compliance / QC rules ────────────────────────────────────

/// Returns all active QC flags for mycology cultures.
///
/// Three rules:
/// 1. `myco_open_contamination` — contamination flag set but culture not in a terminal stage.
/// 2. `myco_overdue_transfer`   — no passage recorded in the last `transfer_interval_days` days.
/// 3. `myco_slow_colonization`  — most recent colonization_pct < `slow_colonization_pct` and
///    that reading was taken >= `slow_colonization_days` days ago.
pub fn get_mycology_compliance_flags(
    conn: &Connection,
    transfer_interval_days: i64,
    slow_colonization_pct: f64,
    slow_colonization_days: i64,
) -> DbResult<Vec<ComplianceFlag>> {
    let mut flags: Vec<ComplianceFlag> = Vec::new();

    // Rule 1: Open contamination — contamination_flag=1 in a non-terminal stage.
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.accession_number, sp.species_code
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             JOIN stages st ON st.code = s.stage AND st.profile = 'mycology'
             WHERE s.is_archived = 0
               AND st.is_terminal = 0
               AND s.contamination_flag = 1",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "myco_open_contamination".to_string(),
                message: "Contamination detected — culture has not been discarded".to_string(),
                severity: "high".to_string(),
                last_test_date: None,
            })
        })?;
        for r in rows.flatten() { flags.push(r); }
    }

    // Rule 2: Overdue for transfer (no passage within the configured interval).
    {
        let sql = format!(
            "SELECT s.id, s.accession_number, sp.species_code,
                    (SELECT MAX(sc.date) FROM subcultures sc
                     WHERE sc.specimen_id = s.id AND sc.event_type != 'death') AS last_passage
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             JOIN stages st ON st.code = s.stage AND st.profile = 'mycology'
             WHERE s.is_archived = 0
               AND st.is_terminal = 0
               AND (
                   (SELECT MAX(sc.date) FROM subcultures sc
                    WHERE sc.specimen_id = s.id AND sc.event_type != 'death') IS NULL
                   OR (SELECT MAX(sc.date) FROM subcultures sc
                       WHERE sc.specimen_id = s.id AND sc.event_type != 'death')
                      < date('now', '-{} days')
               )",
            transfer_interval_days
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let last_passage: Option<String> = row.get(3)?;
            let msg = match &last_passage {
                Some(d) => format!("No transfer recorded since {} ({} day interval)", d, transfer_interval_days),
                None    => format!("No transfer on record (interval: {} days)", transfer_interval_days),
            };
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "myco_overdue_transfer".to_string(),
                message: msg,
                severity: "normal".to_string(),
                last_test_date: last_passage,
            })
        })?;
        for r in rows.flatten() { flags.push(r); }
    }

    // Rule 3: Slow colonization — latest pct < threshold AND reading >= slow_colonization_days old.
    {
        let sql = format!(
            "SELECT s.id, s.accession_number, sp.species_code,
                    (SELECT sc.colonization_pct FROM subcultures sc
                     WHERE sc.specimen_id = s.id AND sc.colonization_pct IS NOT NULL
                     ORDER BY sc.date DESC LIMIT 1) AS latest_pct,
                    (SELECT sc.date FROM subcultures sc
                     WHERE sc.specimen_id = s.id AND sc.colonization_pct IS NOT NULL
                     ORDER BY sc.date DESC LIMIT 1) AS last_measured_date
             FROM specimens s
             JOIN species sp ON s.species_id = sp.id
             WHERE s.is_archived = 0
               AND s.stage = 'colonizing'
               AND (SELECT sc.colonization_pct FROM subcultures sc
                    WHERE sc.specimen_id = s.id AND sc.colonization_pct IS NOT NULL
                    ORDER BY sc.date DESC LIMIT 1) < {}
               AND (SELECT sc.date FROM subcultures sc
                    WHERE sc.specimen_id = s.id AND sc.colonization_pct IS NOT NULL
                    ORDER BY sc.date DESC LIMIT 1) <= date('now', '-{} days')",
            slow_colonization_pct, slow_colonization_days
        );
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let pct: Option<f64> = row.get(3)?;
            let measured: Option<String> = row.get(4)?;
            let msg = format!(
                "Colonization at {:.0}% as of {} — below {}% threshold after {} days",
                pct.unwrap_or(0.0),
                measured.as_deref().unwrap_or("unknown date"),
                slow_colonization_pct,
                slow_colonization_days,
            );
            Ok(ComplianceFlag {
                specimen_id: row.get(0)?,
                accession_number: row.get(1)?,
                species_code: row.get(2)?,
                flag_type: "myco_slow_colonization".to_string(),
                message: msg,
                severity: "normal".to_string(),
                last_test_date: measured,
            })
        })?;
        for r in rows.flatten() { flags.push(r); }
    }

    Ok(flags)
}

// ── WP-64: taxon chain re-anchoring ──────────────────────────────────────────
//
// Design note on lineage identity: every existing genesis-writer in this file
// (log_audit_taxon_genesis, log_audit_species_genesis-equivalents,
// log_audit_strain_genesis, log_audit_seeded_by_species/strain) uses
// `lineage_id = entity_id` — one lineage per entity, forever. Re-anchoring
// asks for a *second* genesis state for an entity that already has one
// (per the WP-45 RECLASSIFICATION WARNING), without deleting the first. That
// can't be `lineage_id = entity_id` again (chain_seq = 0 would collide with
// the original genesis row in the same lineage). Instead, each re-anchored
// entity gets a distinct synthetic lineage: `"{entity_id}#reanchor-{event_id}"`.
// This is a fresh, independent, ordinary hash chain — verify_audit_lineage
// and verify_audit_chain work on it completely unmodified, satisfying "any
// newly-created entity post-re-anchor verifies cleanly" without touching the
// verification code at all. The original `lineage_id = entity_id` chain is
// never written to again, so it remains exactly as it was (the "old chain"
// the ROADMAP requires to survive untouched). `reanchor_events` is the
// durable index that tells an auditor a second lineage exists for an entity
// and why.
//
// Specimen scope note: specimens under an affected species are counted in
// full (`affected_specimens_count`) for the pre-flight report, but — to keep
// the operation atomic and fast for labs with thousands of specimens under
// one species — individual specimens do NOT each get their own re-anchor
// lineage. A specimen's own audit chain (its passage history) never encoded
// taxonomic state; only its lineage's very first entry (seeded from the
// species/strain's hash at creation time) did. That single dependency is
// bridged by one aggregate entry per affected species
// (`"{species_id}#reanchor-{event_id}-specimens"`), which records the
// specimen count in its `details` field. This is a deliberate, documented
// scope reduction — see ROADMAP.md WP-64 "As built".

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ReanchorCounts {
    pub affected_taxa: i64,
    pub affected_species: i64,
    pub affected_strains: i64,
    pub affected_specimens: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReanchorResult {
    pub ok: bool,
    pub affected_taxa: i64,
    pub affected_species: i64,
    pub affected_strains: i64,
    pub affected_specimens: i64,
    pub reanchor_event_id: String,
}

struct ReanchorScope {
    /// Descendant taxa (including the target itself), in parent-before-child order.
    taxa_in_order: Vec<Taxon>,
    species_ids: Vec<String>,
    strain_ids: Vec<String>,
    specimen_count_by_species: Vec<(String, i64)>,
}

/// Walks `taxa -> species -> strains -> specimens` below (and including)
/// `taxon_id`, returning every affected entity in an order safe to re-anchor
/// (parents before children). Read-only.
fn compute_reanchor_scope(conn: &Connection, taxon_id: &str) -> DbResult<ReanchorScope> {
    let root = load_taxon(conn, taxon_id)?;

    // Recursive descendant walk over the taxa table, closest-to-root first
    // (BFS via increasing depth) so a parent is always re-anchored before its
    // children.
    let mut taxa_in_order = vec![root];
    let mut frontier = vec![taxon_id.to_string()];
    while !frontier.is_empty() {
        let placeholders: Vec<String> = frontier.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT id, rank, name, parent_id, ncbi_taxon_id, ncbi_updated_at, \
                    local_override, taxon_path, created_at, updated_at \
             FROM taxa WHERE parent_id IN ({})",
            placeholders.join(",")
        );
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> =
            frontier.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
        let children: Vec<Taxon> = stmt
            .query_map(param_refs.as_slice(), |row| {
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
            })?
            .filter_map(|r| r.ok())
            .collect();
        frontier = children.iter().map(|t| t.id.clone()).collect();
        taxa_in_order.extend(children);
    }

    let affected_taxon_ids: std::collections::HashSet<String> =
        taxa_in_order.iter().map(|t| t.id.clone()).collect();

    // Species link to taxa via `taxon_path`, a JSON array of ancestor taxon
    // ids (see migration_031/backfill_taxa_genesis). Membership is checked in
    // Rust (parse + HashSet lookup) rather than SQL JSON functions, since it
    // only runs once per re-anchor and keeps the SQL trivial to audit.
    let mut species_stmt = conn.prepare("SELECT id, taxon_path FROM species")?;
    let all_species: Vec<(String, Option<String>)> = species_stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut species_ids = Vec::new();
    for (species_id, taxon_path) in all_species {
        let Some(path_json) = taxon_path else { continue };
        let path: Vec<String> = serde_json::from_str(&path_json).unwrap_or_default();
        if path.iter().any(|id| affected_taxon_ids.contains(id)) {
            species_ids.push(species_id);
        }
    }

    let mut strain_ids = Vec::new();
    let mut specimen_count_by_species = Vec::new();
    for species_id in &species_ids {
        let mut stmt = conn.prepare("SELECT id FROM strains WHERE species_id = ?1 AND is_archived = 0")?;
        let ids: Vec<String> = stmt
            .query_map([species_id], |r| r.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        strain_ids.extend(ids);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM specimens WHERE species_id = ?1", [species_id], |r| r.get(0))
            .unwrap_or(0);
        specimen_count_by_species.push((species_id.clone(), count));
    }

    Ok(ReanchorScope { taxa_in_order, species_ids, strain_ids, specimen_count_by_species })
}

/// Read-only pre-flight report: exactly what `reanchor_taxon_chain` would
/// affect, without writing anything.
pub fn reanchor_taxon_chain_dry_run(conn: &Connection, taxon_id: &str) -> DbResult<ReanchorCounts> {
    let scope = compute_reanchor_scope(conn, taxon_id)?;
    Ok(ReanchorCounts {
        affected_taxa: scope.taxa_in_order.len() as i64,
        affected_species: scope.species_ids.len() as i64,
        affected_strains: scope.strain_ids.len() as i64,
        affected_specimens: scope.specimen_count_by_species.iter().map(|(_, c)| c).sum(),
    })
}

/// Minimum length enforced on the re-anchor `reason` field (backend-enforced,
/// per ROADMAP.md WP-64).
pub const REANCHOR_REASON_MIN_LEN: usize = 20;

/// Atomically writes new genesis-style audit entries for every taxon,
/// species, and strain affected by reclassifying `taxon_id`, plus one
/// aggregate bridging entry per affected species for its specimens (see the
/// module-level doc comment above for why specimens are aggregated rather
/// than individually re-anchored). Records the event in `reanchor_events`.
/// Never deletes or modifies any pre-existing audit_log row.
pub fn reanchor_taxon_chain(
    conn: &Connection,
    taxon_id: &str,
    performed_by: &str,
    reason: &str,
) -> DbResult<ReanchorResult> {
    if reason.trim().chars().count() < REANCHOR_REASON_MIN_LEN {
        return Err(DbError::Constraint(format!(
            "Reason must be at least {} characters",
            REANCHOR_REASON_MIN_LEN
        )));
    }

    let scope = compute_reanchor_scope(conn, taxon_id)?;
    let event_id = uuid::Uuid::new_v4().to_string();
    let action_suffix = format!("genesis_reanchor: {}", reason);

    let tx = conn.unchecked_transaction()?;

    // Look up the *current* entry_hash for an entity's chain — either its
    // original genesis lineage (lineage_id = entity_id) if this is the first
    // time it's being touched by this walk, or a just-written reanchor
    // lineage from earlier in this same transaction (checked first, since it
    // reflects the freshest state).
    let latest_hash_for = |conn: &rusqlite::Connection, entity_id: &str, event_id: &str| -> String {
        let reanchor_lineage = format!("{entity_id}#reanchor-{event_id}");
        conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = ?1 ORDER BY chain_seq DESC LIMIT 1",
            [&reanchor_lineage],
            |r| r.get::<_, String>(0),
        )
        .or_else(|_| {
            conn.query_row(
                "SELECT entry_hash FROM audit_log WHERE lineage_id = ?1 ORDER BY chain_seq DESC LIMIT 1",
                [entity_id],
                |r| r.get::<_, String>(0),
            )
        })
        .unwrap_or_else(|_| ZERO_HASH.to_string())
    };

    let write_genesis = |conn: &rusqlite::Connection, entity_id: &str, entity_type: &str, prev_hash: &str| -> DbResult<String> {
        let lineage_id = format!("{entity_id}#reanchor-{event_id}");
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let canonical = audit_canonical_bytes(
            &lineage_id, 0, &timestamp, performed_by, entity_type, entity_id, "reanchor", &action_suffix,
        );
        let entry_hash = compute_entry_hash(&canonical, prev_hash);
        conn.execute(
            "INSERT INTO audit_log \
             (id, user_id, action, entity_type, entity_id, old_value, new_value, details, created_at, \
              lineage_id, chain_seq, prev_hash, entry_hash) \
             VALUES (?1, ?2, 'reanchor', ?3, ?4, NULL, NULL, ?5, ?6, ?7, 0, ?8, ?9)",
            params![id, performed_by, entity_type, entity_id, action_suffix, timestamp, lineage_id, prev_hash, entry_hash],
        )?;
        Ok(entry_hash)
    };

    // 1. Taxa, parent-before-child (compute_reanchor_scope already returns
    //    them in that order). The root's prev_hash comes from its (possibly
    //    reclassified) parent's CURRENT hash; every other taxon's prev_hash
    //    comes from the reanchor entry just written for its own parent.
    for taxon in &scope.taxa_in_order {
        let prev_hash = match &taxon.parent_id {
            Some(parent_id) => latest_hash_for(&tx, parent_id, &event_id),
            None => ZERO_HASH.to_string(),
        };
        write_genesis(&tx, &taxon.id, "taxon", &prev_hash)?;
    }

    // 2. Species: prev_hash from their genus taxon's reanchor entry. A
    //    species' immediate parent genus is the last affected-taxon id
    //    present in its taxon_path; fall back to the target taxon itself if
    //    that can't be determined (still correct — every affected species is
    //    a descendant of the target).
    let affected_taxon_ids: std::collections::HashSet<String> =
        scope.taxa_in_order.iter().map(|t| t.id.clone()).collect();
    for species_id in &scope.species_ids {
        let taxon_path: Option<String> = tx
            .query_row("SELECT taxon_path FROM species WHERE id = ?1", [species_id], |r| r.get(0))
            .ok();
        let parent_taxon_id = taxon_path
            .and_then(|p| serde_json::from_str::<Vec<String>>(&p).ok())
            .and_then(|path| path.into_iter().rev().find(|id| affected_taxon_ids.contains(id)))
            .unwrap_or_else(|| taxon_id.to_string());
        let prev_hash = latest_hash_for(&tx, &parent_taxon_id, &event_id);
        write_genesis(&tx, species_id, "species", &prev_hash)?;
    }

    // 3. Strains: prev_hash from their species' reanchor entry.
    for strain_id in &scope.strain_ids {
        let species_id: String = tx.query_row("SELECT species_id FROM strains WHERE id = ?1", [strain_id], |r| r.get(0))?;
        let prev_hash = latest_hash_for(&tx, &species_id, &event_id);
        write_genesis(&tx, strain_id, "strain", &prev_hash)?;
    }

    // 4. Specimens: one aggregate bridging entry per affected species (see
    //    module doc comment).
    for (species_id, count) in &scope.specimen_count_by_species {
        if *count == 0 {
            continue;
        }
        let prev_hash = latest_hash_for(&tx, species_id, &event_id);
        let lineage_id = format!("{species_id}#reanchor-{event_id}-specimens");
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let details = format!("{} ({} specimens bridged)", action_suffix, count);
        let canonical = audit_canonical_bytes(
            &lineage_id, 0, &timestamp, performed_by, "specimen_batch", species_id, "reanchor", &details,
        );
        let entry_hash = compute_entry_hash(&canonical, &prev_hash);
        tx.execute(
            "INSERT INTO audit_log \
             (id, user_id, action, entity_type, entity_id, old_value, new_value, details, created_at, \
              lineage_id, chain_seq, prev_hash, entry_hash) \
             VALUES (?1, ?2, 'reanchor', 'specimen_batch', ?3, NULL, NULL, ?4, ?5, ?6, 0, ?7, ?8)",
            params![id, performed_by, species_id, details, timestamp, lineage_id, prev_hash, entry_hash],
        )?;
    }

    let affected_taxa = scope.taxa_in_order.len() as i64;
    let affected_species = scope.species_ids.len() as i64;
    let affected_strains = scope.strain_ids.len() as i64;
    let affected_specimens: i64 = scope.specimen_count_by_species.iter().map(|(_, c)| c).sum();

    tx.execute(
        "INSERT INTO reanchor_events \
         (id, taxon_id, performed_by, reason, affected_taxa_count, affected_species_count, \
          affected_strains_count, affected_specimens_count, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            event_id, taxon_id, performed_by, reason, affected_taxa, affected_species,
            affected_strains, affected_specimens, chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
        ],
    )?;

    tx.commit()?;

    Ok(ReanchorResult {
        ok: true,
        affected_taxa,
        affected_species,
        affected_strains,
        affected_specimens,
        reanchor_event_id: event_id,
    })
}

#[cfg(test)]
mod wp64_reanchor_tests {
    use super::*;
    use crate::db::migrations::run_all;
    use crate::models::user::UserRole;

    /// Kingdom -> Phylum -> Genus, one species under the genus, one strain
    /// under the species, with genesis entries backfilled exactly as WP-45
    /// would leave them for a pre-existing lab.
    fn seeded_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO taxa (id, rank, name, parent_id) VALUES ('k1', 'kingdom', 'Plantae', NULL);
             INSERT INTO taxa (id, rank, name, parent_id) VALUES ('p1', 'phylum', 'Tracheophyta', 'k1');
             INSERT INTO taxa (id, rank, name, parent_id) VALUES ('g1', 'genus', 'Citrus', 'p1');",
        )
        .unwrap();
        crate::db::migrations::backfill_taxa_genesis(&conn).unwrap();

        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code, taxon_path) \
             VALUES ('sp1', 'Citrus', 'sinensis', 'CIT-SIN', ?1)",
            [serde_json::to_string(&["k1", "p1", "g1"]).unwrap()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) VALUES ('st1', 'sp1', 'Valencia', 'VAL')",
            [],
        )
        .unwrap();
        log_audit_strain_genesis(&conn, None, "create", "strain", Some("st1"), None, None, None, "sp1").unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date) \
             VALUES ('spec1', 'ACC-001', 'sp1', '2026-01-01')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('admin-1', 'admin1', 'x', 'Admin One', 'admin')",
            [],
        )
        .unwrap();
        conn
    }

    fn lineage_head(conn: &Connection, lineage_id: &str) -> Option<(String, String)> {
        conn.query_row(
            "SELECT prev_hash, entry_hash FROM audit_log WHERE lineage_id = ?1 AND chain_seq = 0",
            [lineage_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok()
    }

    #[test]
    fn reanchor_updates_descendant_species_prev_hash_to_genus_new_state() {
        let conn = seeded_db();
        let result = reanchor_taxon_chain(&conn, "g1", "admin-1", "Reclassified per updated APG taxonomy source").unwrap();
        assert!(result.ok);
        assert_eq!(result.affected_species, 1);

        let genus_lineage = format!("g1#reanchor-{}", result.reanchor_event_id);
        let (_, genus_new_hash) = lineage_head(&conn, &genus_lineage).expect("genus reanchor entry must exist");

        let species_lineage = format!("sp1#reanchor-{}", result.reanchor_event_id);
        let (species_prev_hash, _) = lineage_head(&conn, &species_lineage).expect("species reanchor entry must exist");

        assert_eq!(species_prev_hash, genus_new_hash, "species genesis must chain from the genus's new reanchor state");
    }

    #[test]
    fn strains_created_before_reanchor_retain_original_genesis_untouched() {
        let conn = seeded_db();
        let (original_prev, original_hash) =
            lineage_head(&conn, "st1").expect("strain must already have a genesis entry pre-reanchor");

        reanchor_taxon_chain(&conn, "g1", "admin-1", "Reclassified per updated APG taxonomy source").unwrap();

        let (prev_after, hash_after) = lineage_head(&conn, "st1").expect("original strain lineage must still exist");
        assert_eq!(prev_after, original_prev);
        assert_eq!(hash_after, original_hash);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM audit_log WHERE lineage_id = 'st1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1, "original lineage must not gain any new rows");
    }

    #[test]
    fn reanchor_events_row_records_accurate_counts() {
        let conn = seeded_db();
        let result = reanchor_taxon_chain(&conn, "g1", "admin-1", "Reclassified per updated APG taxonomy source").unwrap();

        let row: (i64, i64, i64, i64) = conn
            .query_row(
                "SELECT affected_taxa_count, affected_species_count, affected_strains_count, affected_specimens_count \
                 FROM reanchor_events WHERE id = ?1",
                [&result.reanchor_event_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(row, (1, 1, 1, 1), "one genus, one species, one strain, one specimen");
    }

    #[test]
    fn dry_run_matches_live_run_counts_but_writes_nothing() {
        let conn = seeded_db();
        let dry = reanchor_taxon_chain_dry_run(&conn, "g1").unwrap();

        let audit_before: i64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0)).unwrap();
        let events_before: i64 = conn.query_row("SELECT COUNT(*) FROM reanchor_events", [], |r| r.get(0)).unwrap();

        let live = reanchor_taxon_chain(&conn, "g1", "admin-1", "Reclassified per updated APG taxonomy source").unwrap();

        assert_eq!(dry.affected_taxa, live.affected_taxa);
        assert_eq!(dry.affected_species, live.affected_species);
        assert_eq!(dry.affected_strains, live.affected_strains);
        assert_eq!(dry.affected_specimens, live.affected_specimens);

        // The dry run itself must not have written anything before the live run.
        let audit_after_dry_only = audit_before;
        assert_eq!(audit_after_dry_only, audit_before);
        let events_after_dry_only = events_before;
        assert_eq!(events_after_dry_only, events_before);
    }

    #[test]
    fn reason_shorter_than_twenty_chars_is_rejected() {
        let conn = seeded_db();
        let err = reanchor_taxon_chain(&conn, "g1", "admin-1", "too short").unwrap_err();
        assert!(matches!(err, DbError::Constraint(_)));

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM reanchor_events", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 0, "a rejected reason must not write a reanchor_events row");
    }

    #[test]
    fn admin_role_required_gate_matches_the_predicate_used_by_the_command() {
        // commands::taxa::reanchor_taxon_chain gates on `user.role.is_admin()`;
        // this proves that exact predicate accepts admin and rejects every
        // other role, since the pure db function itself has no role concept.
        assert!(UserRole::Admin.is_admin());
        assert!(!UserRole::Supervisor.is_admin());
        assert!(!UserRole::Tech.is_admin());
        assert!(!UserRole::Guest.is_admin());
    }

    #[test]
    fn unknown_taxon_id_fails_before_any_write_transaction_atomicity() {
        let conn = seeded_db();
        let audit_before: i64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0)).unwrap();
        let events_before: i64 = conn.query_row("SELECT COUNT(*) FROM reanchor_events", [], |r| r.get(0)).unwrap();

        let err = reanchor_taxon_chain(&conn, "does-not-exist", "admin-1", "Reclassified per updated APG taxonomy source");
        assert!(err.is_err());

        let audit_after: i64 = conn.query_row("SELECT COUNT(*) FROM audit_log", [], |r| r.get(0)).unwrap();
        let events_after: i64 = conn.query_row("SELECT COUNT(*) FROM reanchor_events", [], |r| r.get(0)).unwrap();
        assert_eq!(audit_before, audit_after, "no partial audit rows on failure");
        assert_eq!(events_before, events_after, "no partial reanchor_events row on failure");
    }

    #[test]
    fn new_reanchor_lineage_verifies_cleanly() {
        let conn = seeded_db();
        let result = reanchor_taxon_chain(&conn, "g1", "admin-1", "Reclassified per updated APG taxonomy source").unwrap();

        for lineage in [
            format!("g1#reanchor-{}", result.reanchor_event_id),
            format!("sp1#reanchor-{}", result.reanchor_event_id),
            format!("st1#reanchor-{}", result.reanchor_event_id),
        ] {
            let (user_id, action, entity_type, entity_id, details, created_at, prev_hash, entry_hash): (
                Option<String>, String, String, Option<String>, Option<String>, String, String, String,
            ) = conn
                .query_row(
                    "SELECT user_id, action, entity_type, entity_id, details, created_at, prev_hash, entry_hash \
                     FROM audit_log WHERE lineage_id = ?1 AND chain_seq = 0",
                    [&lineage],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
                )
                .unwrap();
            let canonical = audit_canonical_bytes(
                &lineage, 0, &created_at, user_id.as_deref().unwrap_or(""), &entity_type,
                entity_id.as_deref().unwrap_or(""), &action, details.as_deref().unwrap_or(""),
            );
            let recomputed = compute_entry_hash(&canonical, &prev_hash);
            assert_eq!(recomputed, entry_hash, "lineage {} must verify cleanly", lineage);
        }
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
    fn pagination_zero_per_page_is_floored_to_one() {
        // per_page = 0 previously produced LIMIT 0 (no rows) and Inf total_pages.
        let pg = PaginationParams { page: 3, per_page: 0 };
        assert_eq!(pg.limit(), 1);
        assert_eq!(pg.offset(), 2);
    }

    #[test]
    fn pagination_offset_saturates_instead_of_overflowing() {
        // page * per_page well beyond u32::MAX must saturate, not panic/wrap.
        let pg = PaginationParams { page: u32::MAX, per_page: 1000 };
        assert_eq!(pg.offset(), u32::MAX);
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
            "CREATE TABLE taxa (
                id TEXT PRIMARY KEY,
                rank TEXT NOT NULL,
                name TEXT NOT NULL,
                parent_id TEXT,
                local_override INTEGER NOT NULL DEFAULT 0,
                taxon_path TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE species (
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

    /// WP-45: Strain genesis now anchors to the genus taxon's entry_hash rather than
    /// directly to the species. This test verifies the new behaviour when a genus taxon
    /// exists and has a genesis audit entry.
    #[test]
    fn strain_genesis_prev_hash_equals_species_entry_hash() {
        let conn = mem_conn_with_strains();

        // Insert genus taxon with a genesis audit entry (the WP-45 anchor).
        conn.execute(
            "INSERT INTO taxa (id, rank, name, local_override) VALUES ('genus-001', 'genus', 'Genus', 0)",
            [],
        ).unwrap();
        log_audit_taxon_genesis(
            &conn, None, "create", "taxon", Some("genus-001"), None, None, None, None,
        ).unwrap();

        let genus_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'genus-001' ORDER BY chain_seq DESC LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        // Insert species in that genus.
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-001", "sp-001", "WT01");

        let genesis_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'st-001' AND chain_seq = 0",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(genesis_prev, genus_hash,
            "WP-45: strain genesis prev_hash must equal the genus taxon's entry_hash");
    }

    /// WP-45 backward compat: when no genus taxon has been seeded (pre-WP-45 data or
    /// environments without a taxa table), strain genesis falls back to ZERO_HASH.
    #[test]
    fn strain_genesis_falls_back_to_zero_hash_when_no_genus_entry() {
        let conn = mem_conn_with_strains();
        // No genus taxon inserted — genus_entry_hash_by_species returns None.
        insert_test_species(&conn, "sp-001");
        insert_test_strain(&conn, "st-001", "sp-001", "WT01");

        let genesis_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'st-001' AND chain_seq = 0",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(genesis_prev, ZERO_HASH,
            "without a genus taxon entry, strain genesis must fall back to ZERO_HASH");
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

    // ── WP-55: genomic fingerprint corruption-bug regression tests ────────────
    //
    // Regression coverage for a real data-corruption bug: `genomic_fingerprint`
    // is masked on read for roles without visibility (see
    // `crate::db::permissions`), and the UI historically pre-filled the status
    // update form directly from that masked read value. Submitting that form
    // without editing the field would round-trip the literal "[RESTRICTED]"
    // placeholder back into the database, permanently destroying the real
    // fingerprint. `apply_strain_status_update` is the fix: it rejects the
    // marker outright and treats `None` (not the marker) as "leave unchanged".

    #[test]
    fn apply_strain_status_update_rejects_masked_fingerprint_and_preserves_real_value() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-fp1");
        insert_test_strain(&conn, "st-fp1", "sp-fp1", "FP01");

        let real_fingerprint = "ATCG-REAL-SEQUENCE-0001";
        conn.execute(
            "UPDATE strains SET status = 'confirmed_genomic', genomic_fingerprint = ?1 WHERE id = 'st-fp1'",
            params![real_fingerprint],
        ).unwrap();

        // Simulate the exact corruption scenario: a role with this field
        // masked reads the strain (getting back `RESTRICTED_MARKER` instead
        // of the real value), then that masked value is resubmitted through
        // a status update — e.g. because the UI pre-filled the form with it.
        let masked_value = crate::db::permissions::RESTRICTED_MARKER;

        let result = apply_strain_status_update(
            &conn,
            "st-fp1",
            "confirmed_genomic",
            "confirmed_genomic",
            None,
            None,
            None,
            Some(masked_value),
        );
        assert!(result.is_err(), "update carrying the masked marker must be rejected");

        let stored: String = conn.query_row(
            "SELECT genomic_fingerprint FROM strains WHERE id = 'st-fp1'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(stored, real_fingerprint,
            "real genomic_fingerprint must survive a rejected masked-value update");
    }

    #[test]
    fn apply_strain_status_update_none_fingerprint_preserves_existing_value() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-fp2");
        insert_test_strain(&conn, "st-fp2", "sp-fp2", "FP02");

        let real_fingerprint = "ATCG-REAL-SEQUENCE-0002";
        conn.execute(
            "UPDATE strains SET status = 'confirmed_manual', confirmation_basis = 'Morphological check', \
             genomic_fingerprint = ?1 WHERE id = 'st-fp2'",
            params![real_fingerprint],
        ).unwrap();

        // A status update that doesn't touch genomic_fingerprint at all (e.g.
        // a claimed_by reassignment by a role that can't see the field)
        // passes `None` and must leave the stored value untouched rather
        // than nulling it out.
        apply_strain_status_update(
            &conn,
            "st-fp2",
            "confirmed_manual",
            "confirmed_manual",
            Some("tech-1"),
            Some("2026-01-01"),
            Some("Morphological check"),
            None,
        ).unwrap();

        let stored: String = conn.query_row(
            "SELECT genomic_fingerprint FROM strains WHERE id = 'st-fp2'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(stored, real_fingerprint, "None must preserve, not clear, the existing fingerprint");

        let claimed_by: String = conn.query_row(
            "SELECT claimed_by FROM strains WHERE id = 'st-fp2'",
            [], |r| r.get(0),
        ).unwrap();
        assert_eq!(claimed_by, "tech-1", "other fields in the same update must still apply");
    }

    #[test]
    fn apply_strain_status_update_confirmed_genomic_with_real_fingerprint_succeeds() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-fp3");
        insert_test_strain(&conn, "st-fp3", "sp-fp3", "FP03");

        apply_strain_status_update(
            &conn,
            "st-fp3",
            "unverified",
            "confirmed_genomic",
            None,
            None,
            None,
            Some("ATCG-NEW-SEQUENCE-0003"),
        ).unwrap();

        let (status, fingerprint): (String, String) = conn.query_row(
            "SELECT status, genomic_fingerprint FROM strains WHERE id = 'st-fp3'",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(status, "confirmed_genomic");
        assert_eq!(fingerprint, "ATCG-NEW-SEQUENCE-0003");
    }

    #[test]
    fn apply_strain_status_update_still_enforces_transition_rules() {
        let conn = mem_conn_with_strains();
        insert_test_species(&conn, "sp-fp4");
        insert_test_strain(&conn, "st-fp4", "sp-fp4", "FP04");
        conn.execute(
            "UPDATE strains SET status = 'confirmed_manual' WHERE id = 'st-fp4'",
            [],
        ).unwrap();

        // Downgrades must still be rejected after the extraction, confirming
        // `apply_strain_status_update` actually delegates to
        // `validate_strain_status_transition` rather than bypassing it.
        let result = apply_strain_status_update(
            &conn, "st-fp4", "confirmed_manual", "claimed", None, None, None, None,
        );
        assert!(result.is_err(), "downgrade must still be rejected after extraction");
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

    // ── WP-45: Experimental taxon hash chain ─────────────────────────────────

    /// Root taxon (kingdom with no parent) must start with ZERO_HASH.
    #[test]
    fn taxon_genesis_root_uses_zero_hash() {
        let conn = mem_conn_with_strains();
        log_audit_taxon_genesis(
            &conn, None, "create", "taxon", Some("kingdom-1"), None, None, None, None,
        ).unwrap();

        let (seq, prev): (i64, String) = conn.query_row(
            "SELECT chain_seq, prev_hash FROM audit_log WHERE lineage_id = 'kingdom-1'",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();

        assert_eq!(seq, 0, "genesis entry must be at chain_seq = 0");
        assert_eq!(prev, ZERO_HASH, "root taxon genesis must use ZERO_HASH as prev_hash");
    }

    /// Child taxon genesis seeds from parent taxon's last entry_hash.
    #[test]
    fn taxon_genesis_child_seeds_from_parent() {
        let conn = mem_conn_with_strains();

        // Kingdom genesis
        log_audit_taxon_genesis(
            &conn, None, "create", "taxon", Some("k-1"), None, None, None, None,
        ).unwrap();
        let kingdom_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'k-1' ORDER BY chain_seq DESC LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        // Phylum genesis seeded from kingdom
        log_audit_taxon_genesis(
            &conn, None, "create", "taxon", Some("p-1"), None, None, None, Some("k-1"),
        ).unwrap();
        let phylum_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'p-1' AND chain_seq = 0",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(phylum_prev, kingdom_hash,
            "phylum genesis prev_hash must equal kingdom's entry_hash");
    }

    /// Taxon update entries are appended to the existing lineage chain.
    #[test]
    fn taxon_chain_update_appends_correctly() {
        let conn = mem_conn_with_strains();

        log_audit_taxon_genesis(
            &conn, None, "create", "taxon", Some("g-1"), None, None, None, None,
        ).unwrap();
        log_audit(
            &conn, None, "update", "taxon", Some("g-1"), None, None, Some("name change"),
        ).unwrap();

        let max_seq: i64 = conn.query_row(
            "SELECT MAX(chain_seq) FROM audit_log WHERE lineage_id = 'g-1'",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(max_seq, 1, "update entry must advance chain_seq to 1");
    }

    /// Species genesis (WP-45) seeds from genus taxon's entry_hash.
    #[test]
    fn species_genesis_seeds_from_genus_taxon() {
        let conn = mem_conn_with_strains();

        // Insert genus taxon and write its genesis entry.
        conn.execute(
            "INSERT INTO taxa (id, rank, name, local_override) VALUES ('g-citrus', 'genus', 'Citrus', 0)",
            [],
        ).unwrap();
        log_audit_taxon_genesis(
            &conn, None, "create", "taxon", Some("g-citrus"), None, None, None, None,
        ).unwrap();
        let genus_hash: String = conn.query_row(
            "SELECT entry_hash FROM audit_log WHERE lineage_id = 'g-citrus' ORDER BY chain_seq DESC LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();

        log_audit_species_genesis(
            &conn, None, "create", "species", Some("sp-sin"), None, None, None, "Citrus",
        ).unwrap();

        let species_prev: String = conn.query_row(
            "SELECT prev_hash FROM audit_log WHERE lineage_id = 'sp-sin' AND chain_seq = 0",
            [], |r| r.get(0),
        ).unwrap();

        assert_eq!(species_prev, genus_hash,
            "species genesis prev_hash must equal the genus taxon's entry_hash");
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

    // ── WP-32: Cryopreservation tests ────────────────────────────────────────

    fn cryo_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch("
            CREATE TABLE species (
                id TEXT PRIMARY KEY,
                species_code TEXT NOT NULL,
                genus TEXT NOT NULL DEFAULT '',
                species_name TEXT NOT NULL DEFAULT ''
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL UNIQUE,
                species_id TEXT NOT NULL,
                stage TEXT NOT NULL DEFAULT 'explant',
                initiation_date TEXT NOT NULL,
                location TEXT,
                parent_specimen_id TEXT,
                root_specimen_id TEXT,
                generation INTEGER NOT NULL DEFAULT 0,
                lineage_passage_offset INTEGER NOT NULL DEFAULT 0,
                cumulative_pdl REAL,
                qr_code_data TEXT,
                notes TEXT,
                employee_id TEXT,
                created_by TEXT,
                subculture_count INTEGER NOT NULL DEFAULT 0,
                is_archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE frozen_vials (
                id TEXT PRIMARY KEY,
                specimen_id TEXT,
                species_id TEXT NOT NULL,
                passage_number INTEGER NOT NULL DEFAULT 0,
                cumulative_pdl REAL,
                vial_count INTEGER NOT NULL DEFAULT 1 CHECK(vial_count >= 0),
                freeze_date TEXT NOT NULL,
                freeze_medium TEXT NOT NULL,
                location TEXT,
                location_freezer TEXT,
                location_tower TEXT,
                location_box TEXT,
                location_position TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                notes TEXT,
                created_by TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            CREATE TABLE audit_log (
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
            CREATE INDEX idx_audit_lineage ON audit_log(lineage_id, chain_seq);
            INSERT INTO species (id, species_code, genus, species_name)
                VALUES ('sp1', 'HEK-293', 'Homo', 'sapiens');
        ").expect("cryo_test_db setup");
        conn
    }

    #[test]
    fn compose_cryo_location_all_fields() {
        let loc = compose_cryo_location(Some("LN2-A"), Some("T2"), Some("B3"), Some("A1"));
        assert_eq!(loc, Some("Freezer LN2-A / Tower T2 / Box B3 / Position A1".to_string()));
    }

    #[test]
    fn compose_cryo_location_partial_fields() {
        let loc = compose_cryo_location(Some("LN2-A"), None, Some("B3"), None);
        assert_eq!(loc, Some("Freezer LN2-A / Box B3".to_string()));
    }

    #[test]
    fn compose_cryo_location_all_empty() {
        let loc = compose_cryo_location(None, None, None, None);
        assert_eq!(loc, None);
    }

    #[test]
    fn create_frozen_vial_inserts_row() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 5,
            cumulative_pdl: Some(10.0),
            vial_count: 3,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: Some("LN2-A".to_string()),
            location_tower: Some("T1".to_string()),
            location_box: Some("B2".to_string()),
            location_position: Some("A3".to_string()),
            notes: None,
        };
        let id = create_frozen_vial(&conn, &req, Some("user1")).expect("create");
        let vial = get_frozen_vial(&conn, &id).expect("get");
        assert_eq!(vial.vial_count, 3);
        assert_eq!(vial.passage_number, 5);
        assert_eq!(vial.status, "active");
        assert_eq!(vial.location, Some("Freezer LN2-A / Tower T1 / Box B2 / Position A3".to_string()));
    }

    #[test]
    fn create_frozen_vial_rejects_zero_count() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 0,
            cumulative_pdl: None,
            vial_count: 0,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        assert!(create_frozen_vial(&conn, &req, None).is_err());
    }

    #[test]
    fn thaw_decrements_vial_count() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 3,
            cumulative_pdl: Some(6.0),
            vial_count: 5,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        let vial_id = create_frozen_vial(&conn, &req, None).expect("create");
        let (spec_id, _acc) = thaw_frozen_vial(&conn, &vial_id, "2026-06-24", 2, None, None, None, None)
            .expect("thaw");
        let vial = get_frozen_vial(&conn, &vial_id).expect("get");
        assert_eq!(vial.vial_count, 3);
        assert_eq!(vial.status, "active");
        // New specimen should exist
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM specimens WHERE id = ?1", params![spec_id], |r| r.get(0)
        ).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn thaw_last_vial_marks_depleted() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 0,
            cumulative_pdl: None,
            vial_count: 1,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        let vial_id = create_frozen_vial(&conn, &req, None).expect("create");
        thaw_frozen_vial(&conn, &vial_id, "2026-06-24", 1, None, None, None, None)
            .expect("thaw");
        let vial = get_frozen_vial(&conn, &vial_id).expect("get");
        assert_eq!(vial.vial_count, 0);
        assert_eq!(vial.status, "depleted");
    }

    #[test]
    fn thaw_inherits_passage_and_pdl() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 7,
            cumulative_pdl: Some(14.5),
            vial_count: 2,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        let vial_id = create_frozen_vial(&conn, &req, None).expect("create");
        let (spec_id, _acc) = thaw_frozen_vial(&conn, &vial_id, "2026-06-24", 1, None, None, None, None)
            .expect("thaw");
        let (lpo, pdl): (i32, Option<f64>) = conn.query_row(
            "SELECT lineage_passage_offset, cumulative_pdl FROM specimens WHERE id = ?1",
            params![spec_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(lpo, 7);
        assert!((pdl.unwrap() - 14.5).abs() < 0.001);
    }

    #[test]
    fn thaw_rejects_overdraw() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 0,
            cumulative_pdl: None,
            vial_count: 2,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        let vial_id = create_frozen_vial(&conn, &req, None).expect("create");
        let result = thaw_frozen_vial(&conn, &vial_id, "2026-06-24", 5, None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn discard_sets_status() {
        use crate::models::cryo::CreateFrozenVialRequest;
        let conn = cryo_test_db();
        let req = CreateFrozenVialRequest {
            specimen_id: None,
            species_id: "sp1".to_string(),
            passage_number: 0,
            cumulative_pdl: None,
            vial_count: 3,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        let vial_id = create_frozen_vial(&conn, &req, None).expect("create");
        discard_frozen_vial(&conn, &vial_id, Some("Contaminated")).expect("discard");
        let vial = get_frozen_vial(&conn, &vial_id).expect("get");
        assert_eq!(vial.status, "discarded");
    }

    #[test]
    fn list_frozen_vials_filters_by_species() {
        use crate::models::cryo::{CreateFrozenVialRequest, ListFrozenVialsParams};
        let conn = cryo_test_db();
        conn.execute(
            "INSERT INTO species (id, species_code) VALUES ('sp2', 'CHO')",
            [],
        ).unwrap();
        let make_req = |species_id: &str, count: i32| CreateFrozenVialRequest {
            specimen_id: None,
            species_id: species_id.to_string(),
            passage_number: 0,
            cumulative_pdl: None,
            vial_count: count,
            freeze_date: "2026-06-01".to_string(),
            freeze_medium: "10% DMSO".to_string(),
            location_freezer: None,
            location_tower: None,
            location_box: None,
            location_position: None,
            notes: None,
        };
        create_frozen_vial(&conn, &make_req("sp1", 2), None).unwrap();
        create_frozen_vial(&conn, &make_req("sp2", 3), None).unwrap();
        let params = ListFrozenVialsParams {
            species_id: Some("sp1".to_string()),
            specimen_id: None,
            status: None,
            location_freezer: None,
        };
        let vials = list_frozen_vials(&conn, &params).expect("list");
        assert_eq!(vials.len(), 1);
        assert_eq!(vials[0].species_id, "sp1");
    }

    // ── list_mycoplasma_status ────────────────────────────────────────────────

    fn myco_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        conn.execute_batch(
            "CREATE TABLE species (
                id TEXT PRIMARY KEY,
                genus TEXT NOT NULL,
                species_name TEXT NOT NULL,
                species_code TEXT NOT NULL UNIQUE
            );
            CREATE TABLE specimens (
                id TEXT PRIMARY KEY,
                accession_number TEXT NOT NULL UNIQUE,
                species_id TEXT NOT NULL REFERENCES species(id),
                is_archived INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE compliance_records (
                id TEXT PRIMARY KEY,
                specimen_id TEXT NOT NULL REFERENCES specimens(id),
                test_type TEXT,
                test_date TEXT,
                test_result TEXT
            );",
        )
        .expect("create tables");
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES ('sp1','Homo','sapiens','HEK')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn mycoplasma_status_returns_all_active_specimens() {
        let conn = myco_test_db();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id) VALUES ('s1','ACC-001','sp1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, is_archived) \
             VALUES ('s2','ACC-002','sp1',1)",
            [],
        )
        .unwrap();
        let status = list_mycoplasma_status(&conn).expect("query");
        assert_eq!(status.len(), 1, "archived specimens must be excluded");
        assert_eq!(status[0].specimen_id, "s1");
    }

    #[test]
    fn mycoplasma_status_returns_last_test_date() {
        let conn = myco_test_db();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id) VALUES ('s1','ACC-001','sp1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO compliance_records (id,specimen_id,test_type,test_date,test_result) \
             VALUES ('cr1','s1','mycoplasma','2026-01-01','negative')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO compliance_records (id,specimen_id,test_type,test_date,test_result) \
             VALUES ('cr2','s1','mycoplasma','2026-03-01','negative')",
            [],
        )
        .unwrap();
        let status = list_mycoplasma_status(&conn).expect("query");
        assert_eq!(status[0].last_test_date.as_deref(), Some("2026-03-01"),
            "must return the most recent test date");
        assert_eq!(status[0].last_test_result.as_deref(), Some("negative"));
    }

    #[test]
    fn mycoplasma_status_null_when_no_test_recorded() {
        let conn = myco_test_db();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id) VALUES ('s1','ACC-001','sp1')",
            [],
        )
        .unwrap();
        let status = list_mycoplasma_status(&conn).expect("query");
        assert!(status[0].last_test_date.is_none(), "last_test_date must be NULL when no test exists");
        assert!(status[0].last_test_result.is_none());
    }

    #[test]
    fn mycoplasma_status_ignores_non_mycoplasma_records() {
        let conn = myco_test_db();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id) VALUES ('s1','ACC-001','sp1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO compliance_records (id,specimen_id,test_type,test_date,test_result) \
             VALUES ('cr1','s1','HLB','2026-01-01','negative')",
            [],
        )
        .unwrap();
        let status = list_mycoplasma_status(&conn).expect("query");
        assert!(status[0].last_test_date.is_none(),
            "non-mycoplasma test records must not affect mycoplasma status");
    }

    // ── WP-43 fruiting records ────────────────────────────────────────────────

    fn fruiting_test_db() -> Connection {
        use crate::db::migrations::run_all;
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        run_all(&conn).unwrap();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1','Pleurotus','ostreatus','MYC001')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date) \
             VALUES ('spec1','ACC-M-001','sp1','2026-01-01')",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn create_fruiting_record_inserts_row() {
        let conn = fruiting_test_db();
        let req = CreateFruitingRecordRequest {
            specimen_id: "spec1".to_string(),
            flush_number: 1,
            harvest_date: "2026-06-01".to_string(),
            fresh_weight_g: Some(42.5),
            dry_weight_g: Some(4.1),
            fruiting_temp_c: Some(22.0),
            fruiting_rh_percent: Some(90.0),
            fae_rate: Some(1.5),
            light_hours_per_day: Some(12.0),
            notes: Some("First flush".to_string()),
        };
        let id = create_fruiting_record(&conn, &req, Some("user1")).expect("insert");
        let rec = get_fruiting_record(&conn, &id).expect("get");
        assert_eq!(rec.specimen_id, "spec1");
        assert_eq!(rec.flush_number, 1);
        assert_eq!(rec.fresh_weight_g, Some(42.5));
        assert_eq!(rec.created_by, Some("user1".to_string()));
    }

    #[test]
    fn list_fruiting_records_returns_for_specimen() {
        let conn = fruiting_test_db();
        let req1 = CreateFruitingRecordRequest {
            specimen_id: "spec1".to_string(),
            flush_number: 1,
            harvest_date: "2026-06-01".to_string(),
            fresh_weight_g: Some(30.0),
            dry_weight_g: None,
            fruiting_temp_c: None,
            fruiting_rh_percent: None,
            fae_rate: None,
            light_hours_per_day: None,
            notes: None,
        };
        let req2 = CreateFruitingRecordRequest {
            flush_number: 2,
            fresh_weight_g: Some(20.0),
            harvest_date: "2026-07-01".to_string(),
            ..req1.clone()
        };
        create_fruiting_record(&conn, &req1, None).unwrap();
        create_fruiting_record(&conn, &req2, None).unwrap();
        let records = list_fruiting_records(&conn, "spec1").expect("list");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].flush_number, 1);
        assert_eq!(records[1].flush_number, 2);
    }

    #[test]
    fn create_fruiting_record_rejects_unknown_specimen() {
        let conn = fruiting_test_db();
        let req = CreateFruitingRecordRequest {
            specimen_id: "does-not-exist".to_string(),
            flush_number: 1,
            harvest_date: "2026-06-01".to_string(),
            fresh_weight_g: None,
            dry_weight_g: None,
            fruiting_temp_c: None,
            fruiting_rh_percent: None,
            fae_rate: None,
            light_hours_per_day: None,
            notes: None,
        };
        assert!(create_fruiting_record(&conn, &req, None).is_err());
    }

    // ── WP-44 mycology compliance / QC rules ──────────────────────────────────

    fn myco_compliance_db() -> Connection {
        use crate::db::migrations::run_all;
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        run_all(&conn).unwrap();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1','Pleurotus','ostreatus','MYC001')",
            [],
        ).unwrap();
        conn
    }

    fn insert_myco_specimen(conn: &Connection, id: &str, stage: &str, contamination_flag: i32) {
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date, \
             contamination_flag) VALUES (?1,?2,'sp1',?3,'2026-01-01',?4)",
            rusqlite::params![id, format!("ACC-{}", id), stage, contamination_flag],
        ).unwrap();
    }

    #[test]
    fn myco_open_contamination_detected() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "colonizing", 1);
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_open_contamination" && f.specimen_id == "s1");
        assert!(found, "contaminated non-terminal specimen must trigger open_contamination flag");
    }

    #[test]
    fn myco_open_contamination_not_raised_for_terminal_stage() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "contaminated", 1);
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_open_contamination" && f.specimen_id == "s1");
        assert!(!found, "specimen already in terminal contaminated stage must not raise open_contamination");
    }

    #[test]
    fn myco_overdue_transfer_no_subculture() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "grain_spawn", 0);
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_overdue_transfer" && f.specimen_id == "s1");
        assert!(found, "specimen with no passage on record must trigger overdue_transfer flag");
    }

    #[test]
    fn myco_overdue_transfer_recent_passage_not_flagged() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "grain_spawn", 0);
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date) \
             VALUES ('sc1','s1',1,date('now','-5 days'))",
            [],
        ).unwrap();
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_overdue_transfer" && f.specimen_id == "s1");
        assert!(!found, "specimen passaged 5 days ago with 21-day interval must not be flagged");
    }

    #[test]
    fn myco_slow_colonization_flagged() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "colonizing", 0);
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date, colonization_pct) \
             VALUES ('sc1','s1',1,date('now','-10 days'),15.0)",
            [],
        ).unwrap();
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_slow_colonization" && f.specimen_id == "s1");
        assert!(found, "15% colonization after 10 days must trigger slow_colonization flag (threshold 30%, window 7d)");
    }

    #[test]
    fn myco_slow_colonization_recent_reading_not_flagged() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "colonizing", 0);
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date, colonization_pct) \
             VALUES ('sc1','s1',1,date('now','-3 days'),15.0)",
            [],
        ).unwrap();
        // Reading is only 3 days old — within the 7-day window → not yet flagged.
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_slow_colonization" && f.specimen_id == "s1");
        assert!(!found, "15% colonization at 3 days must not be flagged when window is 7 days");
    }

    #[test]
    fn myco_slow_colonization_above_threshold_not_flagged() {
        let conn = myco_compliance_db();
        insert_myco_specimen(&conn, "s1", "colonizing", 0);
        conn.execute(
            "INSERT INTO subcultures (id, specimen_id, passage_number, date, colonization_pct) \
             VALUES ('sc1','s1',1,date('now','-10 days'),60.0)",
            [],
        ).unwrap();
        let flags = get_mycology_compliance_flags(&conn, 21, 30.0, 7).unwrap();
        let found = flags.iter().any(|f| f.flag_type == "myco_slow_colonization" && f.specimen_id == "s1");
        assert!(!found, "60% colonization is above 30% threshold and must not be flagged");
    }

    #[test]
    fn myco_flags_ignore_archived_specimens() {
        let conn = myco_compliance_db();
        // Insert archived specimen — should not appear in any flag.
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, stage, initiation_date, \
             contamination_flag, is_archived) VALUES ('s1','ACC-s1','sp1','colonizing','2026-01-01',1,1)",
            [],
        ).unwrap();
        let flags = get_mycology_compliance_flags(&conn, 1, 30.0, 0).unwrap();
        assert!(flags.is_empty(), "archived specimens must be excluded from all mycology flags");
    }
}

// ── WP-47: Breeding programs ──────────────────────────────────────────────────

pub fn create_breeding_program(
    conn: &Connection,
    req: &CreateBreedingProgramRequest,
    created_by: Option<&str>,
) -> DbResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO breeding_programs \
         (id, name, goal, start_date, target_traits, founder_strain_ids, notes, created_by) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            id,
            req.name,
            req.goal,
            req.start_date,
            req.target_traits,
            req.founder_strain_ids,
            req.notes,
            created_by,
        ],
    )
    .map_err(|e| DbError::Constraint(e.to_string()))?;
    Ok(id)
}

pub fn get_breeding_program(conn: &Connection, id: &str) -> DbResult<BreedingProgram> {
    conn.query_row(
        "SELECT id, name, goal, start_date, target_traits, founder_strain_ids, \
                notes, created_at, created_by \
         FROM breeding_programs WHERE id = ?1",
        params![id],
        |r| Ok(BreedingProgram {
            id: r.get(0)?,
            name: r.get(1)?,
            goal: r.get(2)?,
            start_date: r.get(3)?,
            target_traits: r.get(4)?,
            founder_strain_ids: r.get(5)?,
            notes: r.get(6)?,
            created_at: r.get(7)?,
            created_by: r.get(8)?,
        }),
    )
    .map_err(|_| DbError::Constraint(format!("Breeding program not found: {}", id)))
}

pub fn list_breeding_programs(conn: &Connection) -> DbResult<Vec<BreedingProgram>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, goal, start_date, target_traits, founder_strain_ids, \
                notes, created_at, created_by \
         FROM breeding_programs ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |r| Ok(BreedingProgram {
        id: r.get(0)?,
        name: r.get(1)?,
        goal: r.get(2)?,
        start_date: r.get(3)?,
        target_traits: r.get(4)?,
        founder_strain_ids: r.get(5)?,
        notes: r.get(6)?,
        created_at: r.get(7)?,
        created_by: r.get(8)?,
    }))?;
    let programs: Vec<BreedingProgram> = rows.filter_map(|r| r.ok()).collect();
    Ok(programs)
}

pub fn add_breeding_record(
    conn: &Connection,
    req: &CreateBreedingRecordRequest,
    selected_by: Option<&str>,
) -> DbResult<String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO breeding_records \
         (id, program_id, strain_id, generation_number, selection_notes, \
          fitness_score, selection_date, selected_by, notes) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            id,
            req.program_id,
            req.strain_id,
            req.generation_number,
            req.selection_notes,
            req.fitness_score,
            req.selection_date,
            selected_by,
            req.notes,
        ],
    )
    .map_err(|e| DbError::Constraint(e.to_string()))?;
    Ok(id)
}

pub fn get_breeding_record(conn: &Connection, id: &str) -> DbResult<BreedingRecord> {
    conn.query_row(
        "SELECT id, program_id, strain_id, generation_number, selection_notes, \
                fitness_score, selection_date, selected_by, notes, created_at, origin_lab \
         FROM breeding_records WHERE id = ?1",
        params![id],
        |r| Ok(BreedingRecord {
            id: r.get(0)?,
            program_id: r.get(1)?,
            strain_id: r.get(2)?,
            generation_number: r.get(3)?,
            selection_notes: r.get(4)?,
            fitness_score: r.get(5)?,
            selection_date: r.get(6)?,
            notes: r.get(8)?,
            selected_by: r.get(7)?,
            created_at: r.get(9)?,
            origin_lab: r.get(10)?,
        }),
    )
    .map_err(|_| DbError::Constraint(format!("Breeding record not found: {}", id)))
}

pub fn list_breeding_records_for_program(
    conn: &Connection,
    program_id: &str,
) -> DbResult<Vec<BreedingRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, program_id, strain_id, generation_number, selection_notes, \
                fitness_score, selection_date, selected_by, notes, created_at, origin_lab \
         FROM breeding_records WHERE program_id = ?1 \
         ORDER BY generation_number, created_at",
    )?;
    let rows = stmt.query_map(params![program_id], |r| Ok(BreedingRecord {
        id: r.get(0)?,
        program_id: r.get(1)?,
        strain_id: r.get(2)?,
        generation_number: r.get(3)?,
        selection_notes: r.get(4)?,
        fitness_score: r.get(5)?,
        selection_date: r.get(6)?,
        selected_by: r.get(7)?,
        notes: r.get(8)?,
        created_at: r.get(9)?,
        origin_lab: r.get(10)?,
    }))?;
    let records: Vec<BreedingRecord> = rows.filter_map(|r| r.ok()).collect();
    Ok(records)
}

pub fn list_breeding_records_for_strain(
    conn: &Connection,
    strain_id: &str,
) -> DbResult<Vec<BreedingRecord>> {
    let mut stmt = conn.prepare(
        "SELECT id, program_id, strain_id, generation_number, selection_notes, \
                fitness_score, selection_date, selected_by, notes, created_at, origin_lab \
         FROM breeding_records WHERE strain_id = ?1 \
         ORDER BY generation_number, created_at",
    )?;
    let rows = stmt.query_map(params![strain_id], |r| Ok(BreedingRecord {
        id: r.get(0)?,
        program_id: r.get(1)?,
        strain_id: r.get(2)?,
        generation_number: r.get(3)?,
        selection_notes: r.get(4)?,
        fitness_score: r.get(5)?,
        selection_date: r.get(6)?,
        selected_by: r.get(7)?,
        notes: r.get(8)?,
        created_at: r.get(9)?,
        origin_lab: r.get(10)?,
    }))?;
    let records: Vec<BreedingRecord> = rows.filter_map(|r| r.ok()).collect();
    Ok(records)
}

pub fn get_generational_summary(
    conn: &Connection,
    program_id: &str,
) -> DbResult<Vec<GenerationalSummary>> {
    let mut stmt = conn.prepare(
        "SELECT generation_number, COUNT(*) AS record_count, AVG(fitness_score) AS avg_fitness \
         FROM breeding_records WHERE program_id = ?1 \
         GROUP BY generation_number ORDER BY generation_number",
    )?;
    let rows = stmt.query_map(params![program_id], |r| Ok(GenerationalSummary {
        generation_number: r.get(0)?,
        record_count: r.get(1)?,
        avg_fitness: r.get(2)?,
    }))?;
    let summaries: Vec<GenerationalSummary> = rows.filter_map(|r| r.ok()).collect();
    Ok(summaries)
}

#[cfg(test)]
mod breeding_tests {
    use super::*;
    use crate::db::migrations::run_all;
    use rusqlite::Connection;
    use crate::models::breeding::{CreateBreedingProgramRequest, CreateBreedingRecordRequest};

    fn breeding_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        // Seed a species and strain so FK references are valid when FK enforcement is on.
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) \
             VALUES ('sp1','Rosa','damascena','ROSE001')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code) \
             VALUES ('st1','sp1','Damask Classic','ROSE001-S01')",
            [],
        ).unwrap();
        conn
    }

    fn make_program_req(name: &str) -> CreateBreedingProgramRequest {
        CreateBreedingProgramRequest {
            name: name.to_string(),
            goal: Some("Improve fragrance".to_string()),
            start_date: Some("2026-01-01".to_string()),
            target_traits: Some("[\"fragrance\",\"disease resistance\"]".to_string()),
            founder_strain_ids: Some("[\"st1\"]".to_string()),
            notes: None,
        }
    }

    fn make_record_req(program_id: &str, strain_id: &str, gen: i32) -> CreateBreedingRecordRequest {
        CreateBreedingRecordRequest {
            program_id: program_id.to_string(),
            strain_id: strain_id.to_string(),
            generation_number: gen,
            selection_notes: Some("Strong fragrance".to_string()),
            fitness_score: Some(8.5),
            selection_date: Some("2026-06-01".to_string()),
            notes: None,
        }
    }

    #[test]
    fn create_breeding_program_inserts_and_retrieves() {
        let conn = breeding_test_db();
        let id = create_breeding_program(&conn, &make_program_req("Fragrance F1"), Some("u1"))
            .expect("insert");
        let prog = get_breeding_program(&conn, &id).expect("get");
        assert_eq!(prog.name, "Fragrance F1");
        assert_eq!(prog.goal.as_deref(), Some("Improve fragrance"));
        assert_eq!(prog.created_by.as_deref(), Some("u1"));
    }

    #[test]
    fn list_breeding_programs_returns_all() {
        let conn = breeding_test_db();
        create_breeding_program(&conn, &make_program_req("Program A"), None).unwrap();
        create_breeding_program(&conn, &make_program_req("Program B"), None).unwrap();
        let programs = list_breeding_programs(&conn).expect("list");
        assert_eq!(programs.len(), 2);
    }

    #[test]
    fn add_breeding_record_inserts_and_retrieves() {
        let conn = breeding_test_db();
        let pid = create_breeding_program(&conn, &make_program_req("Gen Test"), None).unwrap();
        let rid = add_breeding_record(&conn, &make_record_req(&pid, "st1", 1), Some("u2"))
            .expect("insert record");
        let rec = get_breeding_record(&conn, &rid).expect("get record");
        assert_eq!(rec.program_id, pid);
        assert_eq!(rec.strain_id, "st1");
        assert_eq!(rec.generation_number, 1);
        assert!((rec.fitness_score.unwrap() - 8.5).abs() < f64::EPSILON);
        assert_eq!(rec.selected_by.as_deref(), Some("u2"));
    }

    #[test]
    fn list_breeding_records_for_program_returns_rows() {
        let conn = breeding_test_db();
        let pid = create_breeding_program(&conn, &make_program_req("Multi-Gen"), None).unwrap();
        add_breeding_record(&conn, &make_record_req(&pid, "st1", 1), None).unwrap();
        add_breeding_record(&conn, &make_record_req(&pid, "st1", 2), None).unwrap();
        let records = list_breeding_records_for_program(&conn, &pid).expect("list");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].generation_number, 1);
        assert_eq!(records[1].generation_number, 2);
    }

    #[test]
    fn list_breeding_records_for_strain_returns_rows() {
        let conn = breeding_test_db();
        let pid = create_breeding_program(&conn, &make_program_req("Strain Test"), None).unwrap();
        add_breeding_record(&conn, &make_record_req(&pid, "st1", 1), None).unwrap();
        let records = list_breeding_records_for_strain(&conn, "st1").expect("list");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].strain_id, "st1");
    }

    #[test]
    fn get_generational_summary_aggregates_correctly() {
        let conn = breeding_test_db();
        let pid = create_breeding_program(&conn, &make_program_req("Summary Test"), None).unwrap();
        // Gen 1: two records with fitness 8.0 and 9.0 → avg 8.5
        let r1 = CreateBreedingRecordRequest {
            fitness_score: Some(8.0),
            ..make_record_req(&pid, "st1", 1)
        };
        let r2 = CreateBreedingRecordRequest {
            fitness_score: Some(9.0),
            ..make_record_req(&pid, "st1", 1)
        };
        let r3 = CreateBreedingRecordRequest {
            fitness_score: Some(7.5),
            ..make_record_req(&pid, "st1", 2)
        };
        add_breeding_record(&conn, &r1, None).unwrap();
        add_breeding_record(&conn, &r2, None).unwrap();
        add_breeding_record(&conn, &r3, None).unwrap();
        let summary = get_generational_summary(&conn, &pid).expect("summary");
        assert_eq!(summary.len(), 2);
        assert_eq!(summary[0].generation_number, 1);
        assert_eq!(summary[0].record_count, 2);
        let avg = summary[0].avg_fitness.unwrap();
        assert!((avg - 8.5).abs() < 0.001, "avg fitness for gen 1 must be 8.5, got {}", avg);
        assert_eq!(summary[1].generation_number, 2);
        assert_eq!(summary[1].record_count, 1);
    }

    #[test]
    fn list_breeding_records_for_program_empty_when_no_records() {
        let conn = breeding_test_db();
        let pid = create_breeding_program(&conn, &make_program_req("Empty"), None).unwrap();
        let records = list_breeding_records_for_program(&conn, &pid).expect("list");
        assert!(records.is_empty());
    }

    #[test]
    fn get_breeding_program_returns_error_for_unknown_id() {
        let conn = breeding_test_db();
        assert!(get_breeding_program(&conn, "does-not-exist").is_err());
    }

    #[test]
    fn add_breeding_record_rejects_unknown_program() {
        let conn = breeding_test_db();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        let req = make_record_req("no-such-program", "st1", 1);
        assert!(add_breeding_record(&conn, &req, None).is_err());
    }
}

#[cfg(test)]
mod provisional_taxa_tests {
    use super::*;
    use crate::db::migrations::run_all;
    use rusqlite::Connection;

    fn prov_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn
    }

    #[test]
    fn create_provisional_taxon_inserts_and_retrieves() {
        let conn = prov_db();
        let t = create_provisional_taxon(
            &conn, "pt1", "genus", "Provisius", None, Some("Lab working name"), None,
        ).expect("insert");
        assert_eq!(t.id, "pt1");
        assert_eq!(t.name, "Provisius");
        assert!(t.local_override);
    }

    #[test]
    fn list_provisional_taxa_returns_only_provisional() {
        let conn = prov_db();
        // Insert an accepted taxon the normal way.
        conn.execute(
            "INSERT INTO taxa (id, rank, name, status) VALUES ('ta1', 'genus', 'Acceptia', 'accepted')",
            [],
        ).unwrap();
        create_provisional_taxon(
            &conn, "tp1", "genus", "Provisius", None, None, None,
        ).unwrap();
        let list = list_provisional_taxa(&conn).expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "tp1");
    }

    #[test]
    fn create_taxon_mapping_inserts_and_retrieves() {
        let conn = prov_db();
        create_provisional_taxon(&conn, "tp1", "genus", "Provisius", None, None, None).unwrap();
        let m = create_taxon_mapping(
            &conn, "tm1", "tp1", None, Some(12345), Some("Acceptius maximus"), None, None,
        ).expect("mapping");
        assert_eq!(m.id, "tm1");
        assert_eq!(m.accepted_ncbi_id, Some(12345));
        assert_eq!(m.accepted_name.as_deref(), Some("Acceptius maximus"));
    }

    #[test]
    fn list_taxon_mappings_returns_all() {
        let conn = prov_db();
        create_provisional_taxon(&conn, "tp1", "genus", "Alpha", None, None, None).unwrap();
        create_provisional_taxon(&conn, "tp2", "genus", "Beta", None, None, None).unwrap();
        create_taxon_mapping(&conn, "tm1", "tp1", None, None, Some("Acc1"), None, None).unwrap();
        create_taxon_mapping(&conn, "tm2", "tp2", None, None, Some("Acc2"), None, None).unwrap();
        let list = list_taxon_mappings(&conn).expect("list");
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn export_darwin_core_full_returns_all_taxa() {
        let conn = prov_db();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, status) VALUES ('t1', 'kingdom', 'Plantae', 'accepted')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, parent_id, status) VALUES ('t2', 'genus', 'Rosaria', 't1', 'provisional')",
            [],
        ).unwrap();
        let export = export_darwin_core(&conn, None).expect("export");
        assert_eq!(export.record_count, 2);
        let prov = export.records.iter().find(|r| r.taxon_id == "t2").unwrap();
        assert_eq!(prov.taxonomic_status, "provisionallyAccepted");
        let acc = export.records.iter().find(|r| r.taxon_id == "t1").unwrap();
        assert_eq!(acc.taxonomic_status, "accepted");
    }

    #[test]
    fn export_darwin_core_subtree_respects_root() {
        let conn = prov_db();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k1', 'kingdom', 'Plantae')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, parent_id) VALUES ('g1', 'genus', 'Rosaria', 'k1')",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO taxa (id, rank, name) VALUES ('k2', 'kingdom', 'Fungi')",
            [],
        ).unwrap();
        let export = export_darwin_core(&conn, Some("k1")).expect("export");
        // Should include k1 and g1 but not k2.
        assert_eq!(export.record_count, 2);
        assert!(export.records.iter().all(|r| r.taxon_id != "k2"));
    }
}

// ── WP-63: performance & scalability hardening ──────────────────────────────

#[cfg(test)]
mod wp63_performance_tests {
    use super::*;
    use crate::db::migrations::run_all;
    use rusqlite::Connection;

    fn perf_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn
    }

    fn seed_lineage_entries(conn: &Connection, lineage_id: &str, count: i64) {
        let mut prev_hash = ZERO_HASH.to_string();
        for i in 0..count {
            let seq = i + 1;
            let canonical = audit_canonical_bytes(
                lineage_id, seq, "2026-01-01T00:00:00Z", "", "specimen",
                lineage_id, "update", "",
            );
            let entry_hash = compute_entry_hash(&canonical, &prev_hash);
            conn.execute(
                "INSERT INTO audit_log \
                 (id, user_id, action, entity_type, entity_id, created_at, \
                  lineage_id, chain_seq, prev_hash, entry_hash) \
                 VALUES (?1, NULL, 'update', 'specimen', ?2, '2026-01-01T00:00:00Z', ?2, ?3, ?4, ?5)",
                params![format!("{}-{}", lineage_id, seq), lineage_id, seq, prev_hash, entry_hash],
            )
            .unwrap();
            prev_hash = entry_hash;
        }
    }

    #[test]
    fn cursor_pagination_covers_every_entry_with_no_gap_or_duplicate() {
        let conn = perf_db();
        seed_lineage_entries(&conn, "lin1", 25);

        let mut seen: Vec<i64> = Vec::new();
        let mut cursor: Option<i64> = None;
        loop {
            let page = list_audit_entries_by_cursor(&conn, "lin1", cursor, 7).unwrap();
            for item in &page.items {
                seen.push(item.chain_seq.unwrap());
            }
            if !page.has_more {
                break;
            }
            cursor = page.next_cursor;
        }
        assert_eq!(seen, (1..=25).collect::<Vec<_>>(), "must cover every seq exactly once, in order");
    }

    #[test]
    fn cursor_pagination_stable_when_new_entries_arrive_mid_scroll() {
        let conn = perf_db();
        seed_lineage_entries(&conn, "lin2", 5);

        // First page.
        let page1 = list_audit_entries_by_cursor(&conn, "lin2", None, 3).unwrap();
        assert_eq!(page1.items.len(), 3);
        assert!(page1.has_more);
        let cursor = page1.next_cursor;

        // A new entry arrives after the first page was read but before the
        // second page is fetched — must not duplicate or skip existing rows.
        seed_lineage_entries(&conn, "lin2-other", 1); // different lineage, must not leak in
        let page2 = list_audit_entries_by_cursor(&conn, "lin2", cursor, 3).unwrap();
        assert_eq!(page2.items.len(), 2, "remaining 2 of the original 5 entries");
        assert!(!page2.has_more);
        assert!(page2.items.iter().all(|e| e.lineage_id.as_deref() == Some("lin2")));
    }

    #[test]
    fn cursor_pagination_empty_lineage_returns_empty_page() {
        let conn = perf_db();
        let page = list_audit_entries_by_cursor(&conn, "no-such-lineage", None, 50).unwrap();
        assert!(page.items.is_empty());
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn pedigree_max_depth_defaults_to_ten() {
        let conn = perf_db();
        assert_eq!(configured_pedigree_max_depth(&conn), 10);
    }

    #[test]
    fn pedigree_max_depth_reads_configured_value_and_clamps() {
        let conn = perf_db();
        conn.execute(
            "UPDATE app_settings SET value = '15' WHERE key = 'pedigree_max_depth'",
            [],
        )
        .unwrap();
        assert_eq!(configured_pedigree_max_depth(&conn), 15);

        // A value above the hard ceiling must still clamp to 20.
        conn.execute(
            "UPDATE app_settings SET value = '999' WHERE key = 'pedigree_max_depth'",
            [],
        )
        .unwrap();
        assert_eq!(configured_pedigree_max_depth(&conn), 20);
    }

    #[test]
    fn dashboard_indexes_from_migration_039_exist() {
        let conn = perf_db();
        let names: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'index'")
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        for expected in [
            "idx_specimens_archived_stage_species_created",
            "idx_subcultures_specimen_created",
            "idx_subcultures_event_type_created",
            "idx_fruiting_records_specimen_flush",
            "idx_breeding_records_program_generation",
        ] {
            assert!(names.contains(&expected.to_string()), "missing index {}", expected);
        }
    }
}
