// WP-71: connection-level shared-taxonomy-registry lifecycle. Pure functions over
// a rusqlite `Connection` (no Tauri) so the full export → verify → preview →
// import/reconcile flow is unit-testable against an in-memory migrated database,
// exactly as the `passport::store` and `anchoring::store` helpers are. The thin
// `commands::registry` layer only adds session/role gating on top of these.
//
// Guarantee, held here and disclosed in the module docs: importing a registry is
// **additive and non-destructive** — it inserts records this lab does not yet
// have (accept) or a divergent local copy (fork), and otherwise keeps the local
// record (override). It never overwrites or deletes an existing local record. A
// strain is always inserted `unverified` — a foreign lab's confirmed status is
// never inherited.
use rusqlite::{params, Connection};
use serde::Serialize;

use super::{
    assemble_and_sign, parse_registry, verify_registry, IssuerIdentity, RegistryRecord,
    RegistryVerification, TaxonomyRegistry, RECORD_SPECIES, RECORD_STRAIN, RECORD_TAXON,
};
use crate::db::queries::log_audit;
use crate::passport::store::{get_lab_identity, read_lab_name};

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// A summary row for the registry register (issued + imported), without the full
/// JSON payload.
#[derive(Debug, Serialize)]
pub struct RegistryRecordRow {
    pub id: String,
    pub registry_id: String,
    pub direction: String,
    pub issuer_lab: String,
    pub issuer_public_key: String,
    pub content_hash: String,
    pub record_count: i64,
    pub taxon_count: i64,
    pub species_count: i64,
    pub strain_count: i64,
    pub verified: bool,
    pub created_at: String,
}

/// Per-record reconciliation status against the local database.
#[derive(Debug, Clone, Serialize)]
pub struct RecordPlan {
    pub source_key: String,
    pub record_type: String,
    pub name: String,
    pub origin_lab: String,
    /// `new` | `identical` | `conflict`.
    pub local_status: String,
    /// A short human note about the match (e.g. what differs, or a blocker).
    pub detail: String,
    /// The disposition preview suggests as the default.
    pub suggested_disposition: String,
}

/// The result of previewing an import: the verification verdict plus a per-record
/// reconciliation plan. No side effects.
#[derive(Debug, Serialize)]
pub struct RegistryImportPreview {
    pub verification: RegistryVerification,
    pub records: Vec<RecordPlan>,
}

/// One applied record after an import, recording the disposition and what was done.
#[derive(Debug, Clone, Serialize)]
pub struct AppliedRecord {
    pub source_key: String,
    pub record_type: String,
    pub local_status: String,
    pub disposition: String,
    pub action_taken: String,
    pub local_record_id: Option<String>,
}

/// Outcome of importing a registry: the verification verdict, the local audit-log
/// row that now commits the import into this lab's own chain, and the per-record
/// applied dispositions.
#[derive(Debug, Serialize)]
pub struct RegistryImportResult {
    pub imported: bool,
    pub registry_id: String,
    pub local_row_id: String,
    pub audit_entry_id: Option<String>,
    pub verification: RegistryVerification,
    pub applied: Vec<AppliedRecord>,
    pub inserted: i64,
    pub forked: i64,
    pub kept_local: i64,
    pub skipped: i64,
}

/// This lab's public issuer identity (name + Ed25519 public key) — the same WP-60
/// lab key the passport uses. Shared out-of-band so partner labs can verify the
/// registries this lab exports.
pub fn get_identity(conn: &Connection) -> Result<IssuerIdentity, String> {
    get_lab_identity(conn)
}

// ── Gathering local records into a registry ──────────────────────────────────

/// Split a `Genus species` scientific name into `(genus, species_name)`.
fn split_scientific(sci: &str) -> (String, String) {
    match sci.split_once(' ') {
        Some((g, s)) => (g.trim().to_string(), s.trim().to_string()),
        None => (sci.trim().to_string(), String::new()),
    }
}

/// Gather this lab's shareable taxonomy — genus-and-above `taxa`, all `species`,
/// and all non-archived `strains` — as unsigned registry records (record hashes
/// are filled by `assemble_and_sign`). Never exports a strain's genomic
/// fingerprint (it may be access-restricted, and a foreign lab must re-confirm
/// identity anyway); `status` is carried for information only.
pub fn gather_records(conn: &Connection, lab_name: &str) -> Result<Vec<RegistryRecord>, String> {
    let mut records: Vec<RegistryRecord> = Vec::new();

    // Taxa (kingdom..genus). parent_name via a self-join for context.
    let mut stmt = conn
        .prepare(
            "SELECT t.rank, t.name, p.name \
             FROM taxa t LEFT JOIN taxa p ON t.parent_id = p.id \
             ORDER BY t.rank, t.name",
        )
        .map_err(|e| e.to_string())?;
    let taxa = stmt
        .query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    for (rank, name, parent_name) in taxa {
        records.push(RegistryRecord {
            record_type: RECORD_TAXON.to_string(),
            source_key: format!("taxon|{}|{}", rank, name),
            name,
            rank: Some(rank),
            parent_name,
            scientific_name: None,
            code: None,
            strain_type: None,
            status: None,
            note: None,
            origin_lab: lab_name.to_string(),
            record_hash: String::new(),
        });
    }

    // Species.
    let mut stmt = conn
        .prepare("SELECT genus, species_name, species_code, common_name FROM species ORDER BY genus, species_name")
        .map_err(|e| e.to_string())?;
    let species = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    for (genus, species_name, code, common_name) in species {
        let sci = format!("{} {}", genus, species_name);
        records.push(RegistryRecord {
            record_type: RECORD_SPECIES.to_string(),
            source_key: format!("species|{}", sci),
            name: sci.clone(),
            rank: None,
            parent_name: Some(genus),
            scientific_name: Some(sci),
            code: Some(code),
            strain_type: None,
            status: None,
            note: common_name,
            origin_lab: lab_name.to_string(),
            record_hash: String::new(),
        });
    }

    // Strains (non-archived), joined to their species for the scientific name.
    let mut stmt = conn
        .prepare(
            "SELECT sp.genus, sp.species_name, s.code, s.name, s.strain_type, s.status \
             FROM strains s JOIN species sp ON s.species_id = sp.id \
             WHERE s.is_archived = 0 \
             ORDER BY sp.genus, sp.species_name, s.code",
        )
        .map_err(|e| e.to_string())?;
    let strains = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>();
    for (genus, species_name, code, name, strain_type, status) in strains {
        let sci = format!("{} {}", genus, species_name);
        records.push(RegistryRecord {
            record_type: RECORD_STRAIN.to_string(),
            source_key: format!("strain|{}|{}", sci, code),
            name,
            rank: None,
            parent_name: None,
            scientific_name: Some(sci),
            code: Some(code),
            strain_type: Some(strain_type),
            status: Some(status),
            note: None,
            origin_lab: lab_name.to_string(),
            record_hash: String::new(),
        });
    }

    Ok(records)
}

/// Export a signed taxonomy registry for this lab, record it (direction
/// `issued`), and return the full document.
pub fn export_registry(conn: &Connection, created_by: Option<&str>) -> Result<TaxonomyRegistry, String> {
    let (public_key, private_key) = crate::compliance_export::load_or_create_lab_signing_key(conn)?;
    let lab_name = read_lab_name(conn);
    let issuer = IssuerIdentity { lab_name: lab_name.clone(), public_key };
    let records = gather_records(conn, &lab_name)?;

    let registry = assemble_and_sign(
        uuid::Uuid::new_v4().to_string(),
        now_iso(),
        issuer.clone(),
        records,
        &private_key,
    )?;

    let (taxa, species, strains) = kind_counts(&registry);
    let registry_json = serde_json::to_string_pretty(&registry).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO taxonomy_registries \
         (id, registry_id, direction, issuer_lab, issuer_public_key, content_hash, record_count, \
          taxon_count, species_count, strain_count, verified, audit_entry, registry_json, created_by, created_at) \
         VALUES (?1, ?2, 'issued', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, NULL, ?10, ?11, ?12)",
        params![
            uuid::Uuid::new_v4().to_string(),
            registry.registry_id,
            issuer.lab_name,
            issuer.public_key,
            registry.content_hash,
            registry.records.len() as i64,
            taxa,
            species,
            strains,
            registry_json,
            created_by,
            registry.issued_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(registry)
}

fn kind_counts(reg: &TaxonomyRegistry) -> (i64, i64, i64) {
    let mut taxa = 0;
    let mut species = 0;
    let mut strains = 0;
    for r in &reg.records {
        match r.record_type.as_str() {
            RECORD_TAXON => taxa += 1,
            RECORD_SPECIES => species += 1,
            RECORD_STRAIN => strains += 1,
            _ => {}
        }
    }
    (taxa, species, strains)
}

/// Verify a registry JSON with no side effects — pure verification, no import.
pub fn verify_registry_json(json: &str) -> Result<RegistryVerification, String> {
    let registry = parse_registry(json)?;
    Ok(verify_registry(&registry))
}

// ── Local reconciliation lookups ─────────────────────────────────────────────

fn taxon_local_id(conn: &Connection, rank: &str, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT id FROM taxa WHERE rank = ?1 AND name = ?2 AND local_override = 0 LIMIT 1",
        params![rank, name],
        |r| r.get(0),
    )
    .ok()
}

struct LocalSpecies {
    id: String,
    code: String,
    common_name: Option<String>,
}

fn species_local(conn: &Connection, genus: &str, species_name: &str) -> Option<LocalSpecies> {
    conn.query_row(
        "SELECT id, species_code, common_name FROM species WHERE genus = ?1 AND species_name = ?2 LIMIT 1",
        params![genus, species_name],
        |r| {
            Ok(LocalSpecies {
                id: r.get(0)?,
                code: r.get(1)?,
                common_name: r.get(2)?,
            })
        },
    )
    .ok()
}

fn strain_local_id(conn: &Connection, species_id: &str, code: &str) -> Option<String> {
    conn.query_row(
        "SELECT id FROM strains WHERE species_id = ?1 AND code = ?2 LIMIT 1",
        params![species_id, code],
        |r| r.get(0),
    )
    .ok()
}

/// Compute the local reconciliation status for one incoming record.
fn plan_for(conn: &Connection, r: &RegistryRecord) -> RecordPlan {
    let (local_status, detail) = match r.record_type.as_str() {
        RECORD_TAXON => {
            let rank = r.rank.as_deref().unwrap_or("");
            if taxon_local_id(conn, rank, &r.name).is_some() {
                ("identical".to_string(), format!("A local {} '{}' already exists.", rank, r.name))
            } else {
                ("new".to_string(), format!("No local {} '{}'.", rank, r.name))
            }
        }
        RECORD_SPECIES => {
            let (genus, species_name) = split_scientific(r.scientific_name.as_deref().unwrap_or(""));
            match species_local(conn, &genus, &species_name) {
                None => ("new".to_string(), format!("No local species '{}'.", r.name)),
                Some(local) => {
                    let code_match = Some(&local.code) == r.code.as_ref();
                    let note_match = local.common_name == r.note;
                    if code_match && note_match {
                        ("identical".to_string(), "Matches a local species.".to_string())
                    } else {
                        (
                            "conflict".to_string(),
                            format!(
                                "Local species '{}' differs (local code {}, incoming {}).",
                                r.name,
                                local.code,
                                r.code.as_deref().unwrap_or("—")
                            ),
                        )
                    }
                }
            }
        }
        RECORD_STRAIN => {
            let (genus, species_name) = split_scientific(r.scientific_name.as_deref().unwrap_or(""));
            let code = r.code.as_deref().unwrap_or("");
            match species_local(conn, &genus, &species_name) {
                None => (
                    "new".to_string(),
                    format!("Species '{} {}' is not present locally — accept it first.", genus, species_name),
                ),
                Some(local_sp) => {
                    if strain_local_id(conn, &local_sp.id, code).is_some() {
                        ("identical".to_string(), format!("A local strain '{}' already exists.", code))
                    } else {
                        ("new".to_string(), format!("No local strain '{}' under {}.", code, r.name))
                    }
                }
            }
        }
        other => ("new".to_string(), format!("Unknown record type '{}'.", other)),
    };

    let suggested = match local_status.as_str() {
        "conflict" => "override",
        _ => "accept",
    }
    .to_string();

    RecordPlan {
        source_key: r.source_key.clone(),
        record_type: r.record_type.clone(),
        name: r.name.clone(),
        origin_lab: r.origin_lab.clone(),
        local_status,
        detail,
        suggested_disposition: suggested,
    }
}

/// Preview an import: verify the registry and compute a per-record reconciliation
/// plan against the local database. No side effects.
pub fn preview_import(conn: &Connection, json: &str) -> Result<RegistryImportPreview, String> {
    let registry = parse_registry(json)?;
    let verification = verify_registry(&registry);
    let records = if verification.verified {
        registry.records.iter().map(|r| plan_for(conn, r)).collect()
    } else {
        Vec::new()
    };
    Ok(RegistryImportPreview { verification, records })
}

// ── Applying an import ───────────────────────────────────────────────────────

/// A caller's per-record disposition decision. Records omitted use the previewed
/// default (new/identical → accept, conflict → override).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RecordDecision {
    pub source_key: String,
    /// `accept` | `override` | `fork`.
    pub disposition: String,
}

fn code_is_taken(conn: &Connection, code: &str) -> bool {
    conn.query_row("SELECT 1 FROM species WHERE species_code = ?1 LIMIT 1", params![code], |_| Ok(()))
        .is_ok()
}

/// Return `base`, or `base-2`, `base-3`… — the first species code not already used.
fn unique_species_code(conn: &Connection, base: &str) -> String {
    if !code_is_taken(conn, base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{}-{}", base, n);
        if !code_is_taken(conn, &candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn strain_code_taken(conn: &Connection, species_id: &str, code: &str) -> bool {
    strain_local_id(conn, species_id, code).is_some()
}

fn unique_strain_code(conn: &Connection, species_id: &str, base: &str) -> String {
    if !strain_code_taken(conn, species_id, base) {
        return base.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{}-{}", base, n);
        if !strain_code_taken(conn, species_id, &candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Insert a taxon reference record. `local_override` distinguishes an accepted
/// shared taxon (0) from a forked, lab-curated divergent copy (1).
fn insert_taxon(conn: &Connection, rank: &str, name: &str, parent_name: Option<&str>, local_override: i64) -> Result<String, String> {
    // Best-effort parent resolution by name (parents are usually a higher rank).
    let parent_id: Option<String> = parent_name.and_then(|pn| {
        conn.query_row(
            "SELECT id FROM taxa WHERE name = ?1 ORDER BY local_override ASC LIMIT 1",
            params![pn],
            |r| r.get(0),
        )
        .ok()
    });
    let id = uuid::Uuid::new_v4().to_string();
    // taxon_path is a JSON array of local ids (matching backfill_genus_taxa). A
    // child appends its id to the parent's path; a root taxon is just [id].
    let path = match &parent_id {
        Some(pid) => {
            let parent_path: Option<String> = conn
                .query_row("SELECT taxon_path FROM taxa WHERE id = ?1", params![pid], |r| r.get(0))
                .ok()
                .flatten();
            match parent_path {
                Some(pp) if pp.ends_with(']') && pp.len() > 2 => {
                    format!("{},\"{}\"]", &pp[..pp.len() - 1], id)
                }
                _ => format!("[\"{}\"]", id),
            }
        }
        None => format!("[\"{}\"]", id),
    };
    conn.execute(
        "INSERT INTO taxa (id, rank, name, parent_id, local_override, taxon_path) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, rank, name, parent_id, local_override, path],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

fn insert_species(conn: &Connection, genus: &str, species_name: &str, code: &str, common_name: Option<&str>) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let final_code = unique_species_code(conn, code);
    conn.execute(
        "INSERT INTO species (id, genus, species_name, common_name, species_code) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, genus, species_name, common_name, final_code],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Insert a strain reference record. Always `unverified` — a foreign lab's
/// confirmed status is never inherited (the receiver must re-confirm). The origin
/// lab and its claimed status are preserved in `confirmation_basis` for context.
fn insert_strain(
    conn: &Connection,
    species_id: &str,
    name: &str,
    code: &str,
    strain_type: &str,
    origin_lab: &str,
    claimed_status: &str,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let final_code = unique_strain_code(conn, species_id, code);
    let basis = format!(
        "Imported from taxonomy registry issued by {} (origin claimed '{}'); re-verify locally.",
        origin_lab, claimed_status
    );
    conn.execute(
        "INSERT INTO strains (id, species_id, name, code, strain_type, status, is_hybrid, confirmation_basis) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'unverified', 0, ?6)",
        params![id, species_id, name, final_code, strain_type, basis],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Apply one record with a chosen disposition, returning the applied outcome.
fn apply_record(
    conn: &Connection,
    r: &RegistryRecord,
    plan: &RecordPlan,
    disposition: &str,
) -> Result<AppliedRecord, String> {
    let mut local_record_id: Option<String> = None;
    let action_taken: String;

    match disposition {
        "override" => {
            action_taken = "Kept the local record; declined the incoming version.".to_string();
        }
        "accept" => {
            if plan.local_status == "new" {
                match r.record_type.as_str() {
                    RECORD_TAXON => {
                        let id = insert_taxon(conn, r.rank.as_deref().unwrap_or("genus"), &r.name, r.parent_name.as_deref(), 0)?;
                        local_record_id = Some(id);
                        action_taken = "Inserted a new shared taxon.".to_string();
                    }
                    RECORD_SPECIES => {
                        let (genus, species_name) = split_scientific(r.scientific_name.as_deref().unwrap_or(""));
                        let id = insert_species(conn, &genus, &species_name, r.code.as_deref().unwrap_or(""), r.note.as_deref())?;
                        local_record_id = Some(id);
                        action_taken = "Inserted a new species.".to_string();
                    }
                    RECORD_STRAIN => {
                        let (genus, species_name) = split_scientific(r.scientific_name.as_deref().unwrap_or(""));
                        match species_local(conn, &genus, &species_name) {
                            Some(sp) => {
                                let id = insert_strain(
                                    conn,
                                    &sp.id,
                                    &r.name,
                                    r.code.as_deref().unwrap_or(""),
                                    r.strain_type.as_deref().unwrap_or("wildtype"),
                                    &r.origin_lab,
                                    r.status.as_deref().unwrap_or("unverified"),
                                )?;
                                local_record_id = Some(id);
                                action_taken = "Inserted a new strain (unverified).".to_string();
                            }
                            None => {
                                action_taken = format!(
                                    "Skipped — species '{} {}' is not present locally; accept it first.",
                                    genus, species_name
                                );
                            }
                        }
                    }
                    other => {
                        action_taken = format!("Skipped — unknown record type '{}'.", other);
                    }
                }
            } else {
                // identical / conflict: additive-only, so accept keeps local.
                action_taken = "Already present locally; kept the local record.".to_string();
            }
        }
        "fork" => {
            match r.record_type.as_str() {
                RECORD_TAXON => {
                    let forked_name = format!("{} (fork · {})", r.name, r.origin_lab);
                    let id = insert_taxon(conn, r.rank.as_deref().unwrap_or("genus"), &forked_name, r.parent_name.as_deref(), 1)?;
                    local_record_id = Some(id);
                    action_taken = "Forked a divergent local taxon.".to_string();
                }
                RECORD_SPECIES => {
                    let (genus, species_name) = split_scientific(r.scientific_name.as_deref().unwrap_or(""));
                    let base_code = format!("{}-FORK", r.code.as_deref().unwrap_or("SP"));
                    let id = insert_species(conn, &genus, &species_name, &base_code, r.note.as_deref())?;
                    local_record_id = Some(id);
                    action_taken = "Forked a divergent local species.".to_string();
                }
                RECORD_STRAIN => {
                    let (genus, species_name) = split_scientific(r.scientific_name.as_deref().unwrap_or(""));
                    match species_local(conn, &genus, &species_name) {
                        Some(sp) => {
                            let base_code = format!("{}-FORK", r.code.as_deref().unwrap_or("STR"));
                            let forked_name = format!("{} (fork · {})", r.name, r.origin_lab);
                            let id = insert_strain(
                                conn,
                                &sp.id,
                                &forked_name,
                                &base_code,
                                r.strain_type.as_deref().unwrap_or("wildtype"),
                                &r.origin_lab,
                                r.status.as_deref().unwrap_or("unverified"),
                            )?;
                            local_record_id = Some(id);
                            action_taken = "Forked a divergent local strain (unverified).".to_string();
                        }
                        None => {
                            action_taken = format!(
                                "Skipped — species '{} {}' is not present locally; accept it first.",
                                genus, species_name
                            );
                        }
                    }
                }
                other => {
                    action_taken = format!("Skipped — unknown record type '{}'.", other);
                }
            }
        }
        other => {
            return Err(format!("Unknown disposition '{}'.", other));
        }
    }

    Ok(AppliedRecord {
        source_key: r.source_key.clone(),
        record_type: r.record_type.clone(),
        local_status: plan.local_status.clone(),
        disposition: disposition.to_string(),
        action_taken,
        local_record_id,
    })
}

/// Import a received registry: verify it, refuse an invalid or duplicate one, fold
/// it into this lab's own audit chain (a `registry_imported` entry committing to
/// the content hash), then apply each record's disposition additively and record
/// the reconciliation. Records not named in `decisions` use the previewed default.
pub fn import_registry(
    conn: &Connection,
    json: &str,
    decisions: &[RecordDecision],
    imported_by: Option<&str>,
) -> Result<RegistryImportResult, String> {
    let registry = parse_registry(json)?;
    let verification = verify_registry(&registry);
    if !verification.verified {
        return Err(format!("Refusing to import an unverifiable registry: {}", verification.message));
    }

    // Do not import the same registry twice.
    let already: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM taxonomy_registries WHERE direction = 'imported' AND registry_id = ?1",
            params![registry.registry_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if already > 0 {
        return Err(format!("Registry '{}' has already been imported.", registry.registry_id));
    }

    // Fold the import into this lab's own tamper-evident audit chain.
    let details = format!(
        "Imported taxonomy registry from {} ({} records; content {}).",
        registry.issuer.lab_name,
        registry.records.len(),
        &registry.content_hash[..registry.content_hash.len().min(16)]
    );
    log_audit(
        conn,
        imported_by,
        "import",
        "taxonomy_registry",
        Some(&registry.registry_id),
        None,
        Some(&registry.content_hash),
        Some(&details),
    )
    .map_err(|e| e.to_string())?;

    let audit_entry_id: Option<String> = conn
        .query_row(
            "SELECT id FROM audit_log WHERE entity_type = 'taxonomy_registry' AND entity_id = ?1 AND action = 'import' \
             ORDER BY created_at DESC LIMIT 1",
            params![registry.registry_id],
            |r| r.get(0),
        )
        .ok();

    let (taxa, species, strains) = kind_counts(&registry);
    let local_row_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO taxonomy_registries \
         (id, registry_id, direction, issuer_lab, issuer_public_key, content_hash, record_count, \
          taxon_count, species_count, strain_count, verified, audit_entry, registry_json, created_by, created_at) \
         VALUES (?1, ?2, 'imported', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12, ?13)",
        params![
            local_row_id,
            registry.registry_id,
            registry.issuer.lab_name,
            registry.issuer.public_key,
            registry.content_hash,
            registry.records.len() as i64,
            taxa,
            species,
            strains,
            audit_entry_id,
            json,
            imported_by,
            now_iso(),
        ],
    )
    .map_err(|e| e.to_string())?;

    // Apply each record. Species are applied before strains within the sorted
    // order (source_key `species|…` sorts before `strain|…`), so a strain whose
    // species was just accepted in this same batch can attach to it.
    let mut applied: Vec<AppliedRecord> = Vec::new();
    let (mut inserted, mut forked, mut kept_local, mut skipped) = (0i64, 0i64, 0i64, 0i64);
    for r in &registry.records {
        let plan = plan_for(conn, r);
        let disposition = decisions
            .iter()
            .find(|d| d.source_key == r.source_key)
            .map(|d| d.disposition.clone())
            .unwrap_or_else(|| plan.suggested_disposition.clone());

        let outcome = apply_record(conn, r, &plan, &disposition)?;
        match (outcome.disposition.as_str(), outcome.local_record_id.is_some()) {
            ("fork", true) => forked += 1,
            (_, true) => inserted += 1,
            ("accept", false) if outcome.action_taken.starts_with("Skipped") => skipped += 1,
            ("fork", false) => skipped += 1,
            _ => kept_local += 1,
        }

        conn.execute(
            "INSERT INTO registry_record_dispositions \
             (id, registry_row_id, source_key, record_type, local_status, disposition, action_taken, local_record_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                uuid::Uuid::new_v4().to_string(),
                local_row_id,
                outcome.source_key,
                outcome.record_type,
                outcome.local_status,
                outcome.disposition,
                outcome.action_taken,
                outcome.local_record_id,
            ],
        )
        .map_err(|e| e.to_string())?;
        applied.push(outcome);
    }

    Ok(RegistryImportResult {
        imported: true,
        registry_id: registry.registry_id,
        local_row_id,
        audit_entry_id,
        verification,
        applied,
        inserted,
        forked,
        kept_local,
        skipped,
    })
}

// ── Register queries ─────────────────────────────────────────────────────────

/// List registry register rows, newest first, optionally filtered by direction.
pub fn list_registries(conn: &Connection, direction: Option<&str>) -> Result<Vec<RegistryRecordRow>, String> {
    let cols = "id, registry_id, direction, issuer_lab, issuer_public_key, content_hash, record_count, \
                taxon_count, species_count, strain_count, verified, created_at";
    let map = |r: &rusqlite::Row| -> rusqlite::Result<RegistryRecordRow> {
        Ok(RegistryRecordRow {
            id: r.get(0)?,
            registry_id: r.get(1)?,
            direction: r.get(2)?,
            issuer_lab: r.get(3)?,
            issuer_public_key: r.get(4)?,
            content_hash: r.get(5)?,
            record_count: r.get(6)?,
            taxon_count: r.get(7)?,
            species_count: r.get(8)?,
            strain_count: r.get(9)?,
            verified: r.get::<_, i64>(10)? != 0,
            created_at: r.get(11)?,
        })
    };
    match direction {
        Some(dir) => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {} FROM taxonomy_registries WHERE direction = ?1 ORDER BY created_at DESC",
                    cols
                ))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![dir], map)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
        None => {
            let mut stmt = conn
                .prepare(&format!("SELECT {} FROM taxonomy_registries ORDER BY created_at DESC", cols))
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], map)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        }
    }
}

/// Fetch the full stored registry JSON for one register row (for re-export).
pub fn get_registry_json(conn: &Connection, row_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT registry_json FROM taxonomy_registries WHERE id = ?1",
        params![row_id],
        |r| r.get(0),
    )
    .map_err(|_| format!("Registry record '{}' not found.", row_id))
}

/// Fetch the recorded per-record dispositions for one imported registry row.
pub fn list_dispositions(conn: &Connection, registry_row_id: &str) -> Result<Vec<AppliedRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT source_key, record_type, local_status, disposition, action_taken, local_record_id \
             FROM registry_record_dispositions WHERE registry_row_id = ?1 ORDER BY source_key",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![registry_row_id], |r| {
            Ok(AppliedRecord {
                source_key: r.get(0)?,
                record_type: r.get(1)?,
                local_status: r.get(2)?,
                disposition: r.get(3)?,
                action_taken: r.get(4)?,
                local_record_id: r.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_all(&conn).unwrap();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) \
             VALUES ('u1', 'u1', 'x', 'User One', 'admin')",
            [],
        )
        .unwrap();
        conn
    }

    /// Seed a genus taxon, a species, and a strain on `conn`.
    fn seed_taxonomy(conn: &Connection, genus: &str, sp: &str, code: &str, strain_code: &str, strain_status: &str) {
        let tid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, local_override, taxon_path) VALUES (?1, 'genus', ?2, 0, ?3)",
            params![tid, genus, format!("[\"{}\"]", tid)],
        )
        .unwrap();
        let sid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, common_name, species_code) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sid, genus, sp, "Common", code],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, strain_type, status) VALUES (?1, ?2, ?3, ?4, 'wildtype', ?5)",
            params![uuid::Uuid::new_v4().to_string(), sid, format!("{} {} {}", genus, sp, strain_code), strain_code, strain_status],
        )
        .unwrap();
    }

    #[test]
    fn export_gathers_and_signs_all_kinds() {
        let conn = test_db();
        seed_taxonomy(&conn, "Citrus", "sinensis", "CIT-SIN", "VAL", "confirmed_genomic");
        let reg = export_registry(&conn, Some("u1")).unwrap();
        let v = verify_registry(&reg);
        assert!(v.verified, "{}", v.message);
        assert_eq!(v.taxon_count, 1);
        assert_eq!(v.species_count, 1);
        assert_eq!(v.strain_count, 1);
        // Recorded as issued.
        assert_eq!(list_registries(&conn, Some("issued")).unwrap().len(), 1);
    }

    #[test]
    fn export_never_leaks_genomic_fingerprint() {
        let conn = test_db();
        seed_taxonomy(&conn, "Citrus", "sinensis", "CIT-SIN", "VAL", "confirmed_genomic");
        conn.execute("UPDATE strains SET genomic_fingerprint = 'SECRET-FP' WHERE code = 'VAL'", []).unwrap();
        let reg = export_registry(&conn, Some("u1")).unwrap();
        let json = serde_json::to_string(&reg).unwrap();
        assert!(!json.contains("SECRET-FP"), "genomic fingerprint must never be exported");
    }

    #[test]
    fn preview_marks_new_records_against_empty_receiver() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "confirmed_genomic");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        let preview = preview_import(&receiver, &json).unwrap();
        assert!(preview.verification.verified);
        assert!(preview.records.iter().all(|p| p.local_status == "new"));
    }

    #[test]
    fn import_accepts_new_records_into_receiver() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "confirmed_genomic");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        let result = import_registry(&receiver, &json, &[], Some("u1")).unwrap();
        assert!(result.imported);
        assert_eq!(result.inserted, 3, "taxon + species + strain all inserted");
        // The taxon, species and strain now exist locally.
        let taxa: i64 = receiver.query_row("SELECT COUNT(*) FROM taxa WHERE name = 'Citrus'", [], |r| r.get(0)).unwrap();
        assert_eq!(taxa, 1);
        let species: i64 = receiver
            .query_row("SELECT COUNT(*) FROM species WHERE genus = 'Citrus' AND species_name = 'sinensis'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(species, 1);
        let strain: i64 = receiver.query_row("SELECT COUNT(*) FROM strains WHERE code = 'VAL'", [], |r| r.get(0)).unwrap();
        assert_eq!(strain, 1);
    }

    #[test]
    fn imported_strain_is_always_unverified() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "confirmed_genomic");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        import_registry(&receiver, &json, &[], Some("u1")).unwrap();
        let status: String = receiver.query_row("SELECT status FROM strains WHERE code = 'VAL'", [], |r| r.get(0)).unwrap();
        assert_eq!(status, "unverified", "a foreign confirmed_genomic claim must never be inherited");
        let basis: String = receiver.query_row("SELECT confirmation_basis FROM strains WHERE code = 'VAL'", [], |r| r.get(0)).unwrap();
        assert!(basis.contains("re-verify locally"));
    }

    #[test]
    fn import_folds_into_receiver_audit_chain() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        let before: i64 = receiver
            .query_row("SELECT COUNT(*) FROM audit_log WHERE entity_type = 'taxonomy_registry'", [], |r| r.get(0))
            .unwrap();
        let result = import_registry(&receiver, &json, &[], Some("u1")).unwrap();
        assert!(result.audit_entry_id.is_some());
        let after: i64 = receiver
            .query_row("SELECT COUNT(*) FROM audit_log WHERE entity_type = 'taxonomy_registry'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn duplicate_import_is_rejected() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        assert!(import_registry(&receiver, &json, &[], Some("u1")).is_ok());
        assert!(import_registry(&receiver, &json, &[], Some("u1")).is_err());
    }

    #[test]
    fn import_rejects_tampered_registry() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let mut reg = export_registry(&origin, Some("u1")).unwrap();
        reg.records[0].name = "Poncirus".to_string(); // breaks record + content hash
        let json = serde_json::to_string(&reg).unwrap();

        let receiver = test_db();
        assert!(import_registry(&receiver, &json, &[], Some("u1")).is_err());
        assert_eq!(list_registries(&receiver, Some("imported")).unwrap().len(), 0);
    }

    #[test]
    fn override_disposition_keeps_local_and_inserts_nothing() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let reg = export_registry(&origin, Some("u1")).unwrap();
        let json = serde_json::to_string(&reg).unwrap();

        let receiver = test_db();
        let decisions: Vec<RecordDecision> = reg
            .records
            .iter()
            .map(|r| RecordDecision { source_key: r.source_key.clone(), disposition: "override".to_string() })
            .collect();
        let result = import_registry(&receiver, &json, &decisions, Some("u1")).unwrap();
        assert_eq!(result.inserted, 0);
        assert_eq!(result.kept_local, 3);
        let taxa: i64 = receiver.query_row("SELECT COUNT(*) FROM taxa", [], |r| r.get(0)).unwrap();
        assert_eq!(taxa, 0, "override must not insert anything");
    }

    #[test]
    fn fork_disposition_inserts_divergent_copies() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let reg = export_registry(&origin, Some("u1")).unwrap();
        let json = serde_json::to_string(&reg).unwrap();

        // Receiver already has the same species so the strain fork can attach.
        let receiver = test_db();
        seed_taxonomy(&receiver, "Citrus", "sinensis", "CIT-SIN-LOCAL", "LOCAL", "unverified");
        let decisions: Vec<RecordDecision> = reg
            .records
            .iter()
            .map(|r| RecordDecision { source_key: r.source_key.clone(), disposition: "fork".to_string() })
            .collect();
        let result = import_registry(&receiver, &json, &decisions, Some("u1")).unwrap();
        assert_eq!(result.forked, 3);
        // A forked taxon carries the origin lab marker in its name and local_override=1.
        let forked_taxa: i64 = receiver
            .query_row("SELECT COUNT(*) FROM taxa WHERE local_override = 1 AND name LIKE '%(fork%'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(forked_taxa, 1);
    }

    #[test]
    fn strain_accept_skips_when_species_absent() {
        // Origin exports only a strain-bearing taxonomy; receiver overrides the
        // species (keeps nothing) but accepts the strain — which must skip.
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let reg = export_registry(&origin, Some("u1")).unwrap();
        let json = serde_json::to_string(&reg).unwrap();

        let receiver = test_db();
        let decisions: Vec<RecordDecision> = reg
            .records
            .iter()
            .map(|r| {
                let disp = if r.record_type == RECORD_STRAIN { "accept" } else { "override" };
                RecordDecision { source_key: r.source_key.clone(), disposition: disp.to_string() }
            })
            .collect();
        let result = import_registry(&receiver, &json, &decisions, Some("u1")).unwrap();
        assert_eq!(result.skipped, 1, "strain accept with no local species must skip");
        let strains: i64 = receiver.query_row("SELECT COUNT(*) FROM strains", [], |r| r.get(0)).unwrap();
        assert_eq!(strains, 0);
    }

    #[test]
    fn dispositions_are_recorded_for_an_import() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        let result = import_registry(&receiver, &json, &[], Some("u1")).unwrap();
        let recorded = list_dispositions(&receiver, &result.local_row_id).unwrap();
        assert_eq!(recorded.len(), 3);
        assert!(recorded.iter().all(|d| d.disposition == "accept"));
    }

    #[test]
    fn reimporting_after_accept_marks_records_identical() {
        let origin = test_db();
        seed_taxonomy(&origin, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        let json = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        import_registry(&receiver, &json, &[], Some("u1")).unwrap();
        // A fresh export of the same origin data is a different registry_id, so the
        // dup guard does not fire; previewing it now shows everything identical.
        let json2 = serde_json::to_string(&export_registry(&origin, Some("u1")).unwrap()).unwrap();
        let preview = preview_import(&receiver, &json2).unwrap();
        assert!(preview.records.iter().all(|p| p.local_status == "identical"), "already-accepted records read back as identical");
    }

    #[test]
    fn get_registry_json_returns_stored_document() {
        let conn = test_db();
        seed_taxonomy(&conn, "Citrus", "sinensis", "CIT-SIN", "VAL", "unverified");
        export_registry(&conn, Some("u1")).unwrap();
        let row = &list_registries(&conn, Some("issued")).unwrap()[0];
        let json = get_registry_json(&conn, &row.id).unwrap();
        let parsed = parse_registry(&json).unwrap();
        assert!(verify_registry(&parsed).verified);
    }

    #[test]
    fn get_identity_exposes_lab_name_and_key() {
        let conn = test_db();
        let id = get_identity(&conn).unwrap();
        assert!(!id.public_key.is_empty());
    }
}
