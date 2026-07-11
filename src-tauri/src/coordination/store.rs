// WP-72: connection-level breeding-coordination lifecycle. Pure functions over a
// rusqlite `Connection` (no Tauri) so the full export → verify → preview →
// import/merge flow is unit-testable against an in-memory migrated database,
// exactly as the `passport::store` and `registry::store` helpers are. The thin
// `commands::coordination` layer only adds session/role gating on top of these.
//
// Guarantee, held here and disclosed in the module docs: importing a bundle is
// **additive and non-destructive** — it inserts selection records this lab's copy
// of the program does not yet have (accept) and otherwise keeps local state (skip).
// It never overwrites or deletes an existing local selection record, and it never
// overwrites the local program's metadata (an absent program is created as a shell;
// an existing one is matched by name and merged into).
use rusqlite::{params, Connection};
use serde::Serialize;

use super::{
    assemble_and_sign, build_source_key, content_discriminator, parse_bundle, verify_bundle,
    BundleProgram, BundleVerification, CoordinationBundle, IssuerIdentity, SelectionRecord,
};
use crate::db::queries::log_audit;
use crate::passport::store::{get_lab_identity, read_lab_name};

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// A summary row for the bundle register (issued + imported), without the full JSON.
#[derive(Debug, Serialize)]
pub struct BundleRow {
    pub id: String,
    pub bundle_id: String,
    pub direction: String,
    pub issuer_lab: String,
    pub issuer_public_key: String,
    pub program_name: String,
    pub content_hash: String,
    pub record_count: i64,
    pub verified: bool,
    pub created_at: String,
}

/// Per-record reconciliation status against the local program.
#[derive(Debug, Clone, Serialize)]
pub struct SelectionPlan {
    pub source_key: String,
    pub strain_scientific_name: String,
    pub strain_code: String,
    pub generation_number: i32,
    pub origin_lab: String,
    /// `new` | `identical` | `blocked` (strain not present locally).
    pub local_status: String,
    /// A short human note about the match (e.g. what blocks it).
    pub detail: String,
    /// The disposition preview suggests as the default.
    pub suggested_disposition: String,
}

/// The result of previewing an import: the verification verdict, a per-record merge
/// plan, and whether the local program already exists. No side effects.
#[derive(Debug, Serialize)]
pub struct BundleImportPreview {
    pub verification: BundleVerification,
    pub program_exists_locally: bool,
    pub records: Vec<SelectionPlan>,
}

/// One applied record after an import, recording the disposition and what was done.
#[derive(Debug, Clone, Serialize)]
pub struct AppliedSelection {
    pub source_key: String,
    pub local_status: String,
    pub disposition: String,
    pub action_taken: String,
    pub local_record_id: Option<String>,
}

/// Outcome of importing a bundle: the verification verdict, the local audit-log row
/// that now commits the merge into this lab's own chain, and the per-record applied
/// dispositions.
#[derive(Debug, Serialize)]
pub struct BundleImportResult {
    pub imported: bool,
    pub bundle_id: String,
    pub local_row_id: String,
    pub program_id: String,
    pub program_created: bool,
    pub audit_entry_id: Option<String>,
    pub verification: BundleVerification,
    pub applied: Vec<AppliedSelection>,
    pub inserted: i64,
    pub kept_local: i64,
    pub skipped: i64,
}

/// This lab's public issuer identity (name + Ed25519 public key) — the same WP-60
/// lab key the passport and registry use.
pub fn get_identity(conn: &Connection) -> Result<IssuerIdentity, String> {
    get_lab_identity(conn)
}

// ── Gathering a program's selection records into a bundle ─────────────────────

/// Split a `Genus species` scientific name into `(genus, species_name)`.
fn split_scientific(sci: &str) -> (String, String) {
    match sci.split_once(' ') {
        Some((g, s)) => (g.trim().to_string(), s.trim().to_string()),
        None => (sci.trim().to_string(), String::new()),
    }
}

struct ProgramRow {
    name: String,
    goal: Option<String>,
    target_traits: Option<String>,
    start_date: Option<String>,
    notes: Option<String>,
}

fn load_program(conn: &Connection, program_id: &str) -> Result<ProgramRow, String> {
    conn.query_row(
        "SELECT name, goal, target_traits, start_date, notes FROM breeding_programs WHERE id = ?1",
        params![program_id],
        |r| {
            Ok(ProgramRow {
                name: r.get(0)?,
                goal: r.get(1)?,
                target_traits: r.get(2)?,
                start_date: r.get(3)?,
                notes: r.get(4)?,
            })
        },
    )
    .map_err(|_| format!("Breeding program '{}' not found.", program_id))
}

/// Gather one program's selection records as unsigned bundle records (record hashes
/// are filled by `assemble_and_sign`). Each record references its strain by
/// scientific name + code so a partner lab can resolve it. A record's `origin_lab`
/// is the lab that authored it — a locally-authored record carries this lab's name;
/// a previously-imported record keeps its foreign origin (so a re-export preserves
/// provenance).
pub fn gather_records(conn: &Connection, program_id: &str, program_name: &str, lab_name: &str) -> Result<Vec<SelectionRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT sp.genus, sp.species_name, s.code, br.generation_number, br.selection_notes, \
                    br.fitness_score, br.selection_date, br.selected_by, br.notes, br.origin_lab \
             FROM breeding_records br \
             JOIN strains s ON br.strain_id = s.id \
             JOIN species sp ON s.species_id = sp.id \
             WHERE br.program_id = ?1 \
             ORDER BY br.generation_number, br.created_at",
        )
        .map_err(|e| e.to_string())?;
    struct Row {
        genus: String,
        species_name: String,
        code: String,
        generation_number: i32,
        selection_notes: Option<String>,
        fitness_score: Option<f64>,
        selection_date: Option<String>,
        selected_by: Option<String>,
        notes: Option<String>,
        origin_lab: Option<String>,
    }
    let rows: Vec<Row> = stmt
        .query_map(params![program_id], |r| {
            Ok(Row {
                genus: r.get(0)?,
                species_name: r.get(1)?,
                code: r.get(2)?,
                generation_number: r.get(3)?,
                selection_notes: r.get(4)?,
                fitness_score: r.get(5)?,
                selection_date: r.get(6)?,
                selected_by: r.get(7)?,
                notes: r.get(8)?,
                origin_lab: r.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Dedup by natural key: two byte-identical selection events are the same event
    // and must not produce a duplicate key (which verification would reject).
    let mut records: Vec<SelectionRecord> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in rows {
        let sci = format!("{} {}", row.genus, row.species_name);
        let disc = content_discriminator(row.selection_notes.as_deref(), row.fitness_score, row.notes.as_deref());
        let source_key = build_source_key(
            program_name,
            &sci,
            &row.code,
            row.generation_number,
            row.selection_date.as_deref(),
            row.selected_by.as_deref(),
            &disc,
        );
        if !seen.insert(source_key.clone()) {
            continue;
        }
        let origin = row.origin_lab.unwrap_or_else(|| lab_name.to_string());
        records.push(SelectionRecord {
            source_key,
            strain_scientific_name: sci,
            strain_code: row.code,
            generation_number: row.generation_number,
            selection_notes: row.selection_notes,
            fitness_score: row.fitness_score,
            selection_date: row.selection_date,
            selected_by: row.selected_by,
            notes: row.notes,
            origin_lab: origin,
            record_hash: String::new(),
        });
    }
    Ok(records)
}

/// Export a signed coordination bundle for one program, record it (direction
/// `issued`), and return the full document.
pub fn export_bundle(conn: &Connection, program_id: &str, created_by: Option<&str>) -> Result<CoordinationBundle, String> {
    let (public_key, private_key) = crate::compliance_export::load_or_create_lab_signing_key(conn)?;
    let lab_name = read_lab_name(conn);
    let issuer = IssuerIdentity { lab_name: lab_name.clone(), public_key };
    let prog = load_program(conn, program_id)?;
    let records = gather_records(conn, program_id, &prog.name, &lab_name)?;
    let program = BundleProgram {
        name: prog.name.clone(),
        goal: prog.goal,
        target_traits: prog.target_traits,
        start_date: prog.start_date,
        notes: prog.notes,
        origin_lab: lab_name.clone(),
    };

    let bundle = assemble_and_sign(
        uuid::Uuid::new_v4().to_string(),
        now_iso(),
        issuer.clone(),
        program,
        records,
        &private_key,
    )?;

    let bundle_json = serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO breeding_bundles \
         (id, bundle_id, direction, issuer_lab, issuer_public_key, program_name, content_hash, \
          record_count, verified, audit_entry, bundle_json, created_by, created_at) \
         VALUES (?1, ?2, 'issued', ?3, ?4, ?5, ?6, ?7, 1, NULL, ?8, ?9, ?10)",
        params![
            uuid::Uuid::new_v4().to_string(),
            bundle.bundle_id,
            issuer.lab_name,
            issuer.public_key,
            bundle.program.name,
            bundle.content_hash,
            bundle.records.len() as i64,
            bundle_json,
            created_by,
            bundle.issued_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(bundle)
}

/// Verify a bundle JSON with no side effects — pure verification, no import.
pub fn verify_bundle_json(json: &str) -> Result<BundleVerification, String> {
    let bundle = parse_bundle(json)?;
    Ok(verify_bundle(&bundle))
}

// ── Local reconciliation lookups ─────────────────────────────────────────────

/// Find the local program id for a program name (the cross-lab natural key).
fn local_program_id(conn: &Connection, name: &str) -> Option<String> {
    conn.query_row(
        "SELECT id FROM breeding_programs WHERE name = ?1 LIMIT 1",
        params![name],
        |r| r.get(0),
    )
    .ok()
}

/// Resolve a strain by its cross-lab identity (scientific name + code) to a local
/// strain id.
fn strain_local_id(conn: &Connection, scientific_name: &str, code: &str) -> Option<String> {
    let (genus, species_name) = split_scientific(scientific_name);
    conn.query_row(
        "SELECT s.id FROM strains s JOIN species sp ON s.species_id = sp.id \
         WHERE sp.genus = ?1 AND sp.species_name = ?2 AND s.code = ?3 LIMIT 1",
        params![genus, species_name, code],
        |r| r.get(0),
    )
    .ok()
}

/// Recompute the set of natural keys already present in the local copy of a program
/// — so an incoming record can be classified `new` vs `identical` without storing
/// `source_key` locally.
fn local_source_keys(conn: &Connection, program_id: &str, program_name: &str) -> Result<std::collections::HashSet<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT sp.genus, sp.species_name, s.code, br.generation_number, br.selection_notes, \
                    br.fitness_score, br.selection_date, br.selected_by, br.notes \
             FROM breeding_records br \
             JOIN strains s ON br.strain_id = s.id \
             JOIN species sp ON s.species_id = sp.id \
             WHERE br.program_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let keys = stmt
        .query_map(params![program_id], |r| {
            let genus: String = r.get(0)?;
            let species_name: String = r.get(1)?;
            let code: String = r.get(2)?;
            let generation_number: i32 = r.get(3)?;
            let selection_notes: Option<String> = r.get(4)?;
            let fitness_score: Option<f64> = r.get(5)?;
            let selection_date: Option<String> = r.get(6)?;
            let selected_by: Option<String> = r.get(7)?;
            let notes: Option<String> = r.get(8)?;
            let sci = format!("{} {}", genus, species_name);
            let disc = content_discriminator(selection_notes.as_deref(), fitness_score, notes.as_deref());
            Ok(build_source_key(
                program_name,
                &sci,
                &code,
                generation_number,
                selection_date.as_deref(),
                selected_by.as_deref(),
                &disc,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(keys)
}

/// Compute the local reconciliation status for one incoming selection record.
fn plan_for(
    conn: &Connection,
    r: &SelectionRecord,
    local_program: Option<&str>,
    local_keys: &std::collections::HashSet<String>,
) -> SelectionPlan {
    let (local_status, detail, suggested) = if strain_local_id(conn, &r.strain_scientific_name, &r.strain_code).is_none() {
        (
            "blocked".to_string(),
            format!(
                "Strain '{} {}' is not present locally — import it via the taxonomy registry first.",
                r.strain_scientific_name, r.strain_code
            ),
            "skip",
        )
    } else if local_program.is_some() && local_keys.contains(&r.source_key) {
        (
            "identical".to_string(),
            "This selection record is already present in the local program.".to_string(),
            "skip",
        )
    } else {
        (
            "new".to_string(),
            format!(
                "New selection of '{} {}' at generation {}.",
                r.strain_scientific_name, r.strain_code, r.generation_number
            ),
            "accept",
        )
    };

    SelectionPlan {
        source_key: r.source_key.clone(),
        strain_scientific_name: r.strain_scientific_name.clone(),
        strain_code: r.strain_code.clone(),
        generation_number: r.generation_number,
        origin_lab: r.origin_lab.clone(),
        local_status,
        detail,
        suggested_disposition: suggested.to_string(),
    }
}

/// Preview an import: verify the bundle and compute a per-record merge plan against
/// the local copy of the program. No side effects.
pub fn preview_import(conn: &Connection, json: &str) -> Result<BundleImportPreview, String> {
    let bundle = parse_bundle(json)?;
    let verification = verify_bundle(&bundle);
    let local_program = local_program_id(conn, &bundle.program.name);
    let program_exists_locally = local_program.is_some();
    let records = if verification.verified {
        let local_keys = match &local_program {
            Some(pid) => local_source_keys(conn, pid, &bundle.program.name)?,
            None => std::collections::HashSet::new(),
        };
        bundle
            .records
            .iter()
            .map(|r| plan_for(conn, r, local_program.as_deref(), &local_keys))
            .collect()
    } else {
        Vec::new()
    };
    Ok(BundleImportPreview { verification, program_exists_locally, records })
}

// ── Applying an import ───────────────────────────────────────────────────────

/// A caller's per-record disposition decision. Records omitted use the previewed
/// default (`new` → accept, `identical`/`blocked` → skip).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SelectionDecision {
    pub source_key: String,
    /// `accept` | `skip`.
    pub disposition: String,
}

/// Create a local shell of the program (name + carried metadata) so imported
/// selection records have somewhere to attach. Called only when no local program of
/// this name exists — never overwrites an existing one.
fn create_program_shell(conn: &Connection, program: &BundleProgram, created_by: Option<&str>) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let notes = format!(
        "Coordinated copy created from a bundle issued by {}.{}",
        program.origin_lab,
        program.notes.as_deref().map(|n| format!(" {}", n)).unwrap_or_default()
    );
    conn.execute(
        "INSERT INTO breeding_programs (id, name, goal, start_date, target_traits, notes, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![id, program.name, program.goal, program.start_date, program.target_traits, notes, created_by],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Insert an imported selection record into the local program. The record's
/// `origin_lab` is preserved so its provenance is visible and a re-export keeps it.
fn insert_selection(conn: &Connection, program_id: &str, strain_id: &str, r: &SelectionRecord) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO breeding_records \
         (id, program_id, strain_id, generation_number, selection_notes, fitness_score, \
          selection_date, selected_by, notes, origin_lab) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            id,
            program_id,
            strain_id,
            r.generation_number,
            r.selection_notes,
            r.fitness_score,
            r.selection_date,
            r.selected_by,
            r.notes,
            r.origin_lab,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

/// Import a received bundle: verify it, refuse an invalid or duplicate one, ensure
/// the local copy of the program (create a shell if absent — never overwrite),
/// fold the merge into this lab's own audit chain (a `breeding_merge_imported`
/// entry committing to the content hash), then apply each record's disposition
/// additively and record the reconciliation. Records not named in `decisions` use
/// the previewed default.
pub fn import_bundle(
    conn: &Connection,
    json: &str,
    decisions: &[SelectionDecision],
    imported_by: Option<&str>,
) -> Result<BundleImportResult, String> {
    let bundle = parse_bundle(json)?;
    let verification = verify_bundle(&bundle);
    if !verification.verified {
        return Err(format!("Refusing to import an unverifiable bundle: {}", verification.message));
    }

    // Do not import the same bundle twice.
    let already: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM breeding_bundles WHERE direction = 'imported' AND bundle_id = ?1",
            params![bundle.bundle_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if already > 0 {
        return Err(format!("Bundle '{}' has already been imported.", bundle.bundle_id));
    }

    // Ensure the local copy of the program (additive — create only if absent).
    let (program_id, program_created) = match local_program_id(conn, &bundle.program.name) {
        Some(pid) => (pid, false),
        None => (create_program_shell(conn, &bundle.program, imported_by)?, true),
    };

    // Fold the merge into this lab's own tamper-evident audit chain.
    let details = format!(
        "Merged breeding-coordination bundle for '{}' from {} ({} records; content {}).",
        bundle.program.name,
        bundle.issuer.lab_name,
        bundle.records.len(),
        &bundle.content_hash[..bundle.content_hash.len().min(16)]
    );
    log_audit(
        conn,
        imported_by,
        "import",
        "breeding_coordination",
        Some(&bundle.bundle_id),
        None,
        Some(&bundle.content_hash),
        Some(&details),
    )
    .map_err(|e| e.to_string())?;

    let audit_entry_id: Option<String> = conn
        .query_row(
            "SELECT id FROM audit_log WHERE entity_type = 'breeding_coordination' AND entity_id = ?1 AND action = 'import' \
             ORDER BY created_at DESC LIMIT 1",
            params![bundle.bundle_id],
            |r| r.get(0),
        )
        .ok();

    let local_row_id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO breeding_bundles \
         (id, bundle_id, direction, issuer_lab, issuer_public_key, program_name, content_hash, \
          record_count, verified, audit_entry, bundle_json, created_by, created_at) \
         VALUES (?1, ?2, 'imported', ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9, ?10, ?11)",
        params![
            local_row_id,
            bundle.bundle_id,
            bundle.issuer.lab_name,
            bundle.issuer.public_key,
            bundle.program.name,
            bundle.content_hash,
            bundle.records.len() as i64,
            audit_entry_id,
            json,
            imported_by,
            now_iso(),
        ],
    )
    .map_err(|e| e.to_string())?;

    // Apply each record.
    let local_keys = local_source_keys(conn, &program_id, &bundle.program.name)?;
    let mut applied: Vec<AppliedSelection> = Vec::new();
    let (mut inserted, mut kept_local, mut skipped) = (0i64, 0i64, 0i64);
    for r in &bundle.records {
        let plan = plan_for(conn, r, Some(program_id.as_str()), &local_keys);
        let disposition = decisions
            .iter()
            .find(|d| d.source_key == r.source_key)
            .map(|d| d.disposition.clone())
            .unwrap_or_else(|| plan.suggested_disposition.clone());

        let mut local_record_id: Option<String> = None;
        let action_taken: String;
        match disposition.as_str() {
            "accept" => {
                if plan.local_status == "new" {
                    match strain_local_id(conn, &r.strain_scientific_name, &r.strain_code) {
                        Some(sid) => {
                            let id = insert_selection(conn, &program_id, &sid, r)?;
                            local_record_id = Some(id);
                            action_taken = "Merged in a new selection record.".to_string();
                            inserted += 1;
                        }
                        None => {
                            action_taken = format!(
                                "Skipped — strain '{} {}' is not present locally.",
                                r.strain_scientific_name, r.strain_code
                            );
                            skipped += 1;
                        }
                    }
                } else if plan.local_status == "blocked" {
                    action_taken = format!(
                        "Skipped — strain '{} {}' is not present locally.",
                        r.strain_scientific_name, r.strain_code
                    );
                    skipped += 1;
                } else {
                    action_taken = "Already present locally; kept the local record.".to_string();
                    kept_local += 1;
                }
            }
            "skip" => {
                action_taken = "Skipped by operator choice.".to_string();
                skipped += 1;
            }
            other => {
                return Err(format!("Unknown disposition '{}'.", other));
            }
        }

        conn.execute(
            "INSERT INTO breeding_bundle_dispositions \
             (id, bundle_row_id, source_key, local_status, disposition, action_taken, local_record_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                uuid::Uuid::new_v4().to_string(),
                local_row_id,
                r.source_key,
                plan.local_status,
                disposition,
                action_taken,
                local_record_id,
            ],
        )
        .map_err(|e| e.to_string())?;

        applied.push(AppliedSelection {
            source_key: r.source_key.clone(),
            local_status: plan.local_status,
            disposition,
            action_taken,
            local_record_id,
        });
    }

    Ok(BundleImportResult {
        imported: true,
        bundle_id: bundle.bundle_id,
        local_row_id,
        program_id,
        program_created,
        audit_entry_id,
        verification,
        applied,
        inserted,
        kept_local,
        skipped,
    })
}

// ── Register queries ─────────────────────────────────────────────────────────

/// List bundle register rows, newest first, optionally filtered by direction.
pub fn list_bundles(conn: &Connection, direction: Option<&str>) -> Result<Vec<BundleRow>, String> {
    let cols = "id, bundle_id, direction, issuer_lab, issuer_public_key, program_name, content_hash, \
                record_count, verified, created_at";
    let map = |r: &rusqlite::Row| -> rusqlite::Result<BundleRow> {
        Ok(BundleRow {
            id: r.get(0)?,
            bundle_id: r.get(1)?,
            direction: r.get(2)?,
            issuer_lab: r.get(3)?,
            issuer_public_key: r.get(4)?,
            program_name: r.get(5)?,
            content_hash: r.get(6)?,
            record_count: r.get(7)?,
            verified: r.get::<_, i64>(8)? != 0,
            created_at: r.get(9)?,
        })
    };
    match direction {
        Some(dir) => {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT {} FROM breeding_bundles WHERE direction = ?1 ORDER BY created_at DESC",
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
                .prepare(&format!("SELECT {} FROM breeding_bundles ORDER BY created_at DESC", cols))
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

/// Fetch the full stored bundle JSON for one register row (for re-export).
pub fn get_bundle_json(conn: &Connection, row_id: &str) -> Result<String, String> {
    conn.query_row(
        "SELECT bundle_json FROM breeding_bundles WHERE id = ?1",
        params![row_id],
        |r| r.get(0),
    )
    .map_err(|_| format!("Bundle record '{}' not found.", row_id))
}

/// Fetch the recorded per-record dispositions for one imported bundle row.
pub fn list_dispositions(conn: &Connection, bundle_row_id: &str) -> Result<Vec<AppliedSelection>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT source_key, local_status, disposition, action_taken, local_record_id \
             FROM breeding_bundle_dispositions WHERE bundle_row_id = ?1 ORDER BY source_key",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![bundle_row_id], |r| {
            Ok(AppliedSelection {
                source_key: r.get(0)?,
                local_status: r.get(1)?,
                disposition: r.get(2)?,
                action_taken: r.get(3)?,
                local_record_id: r.get(4)?,
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

    /// Seed a genus/species/strain and return the strain id.
    fn seed_strain(conn: &Connection, genus: &str, sp: &str, code: &str) -> String {
        let tid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO taxa (id, rank, name, local_override, taxon_path) VALUES (?1, 'genus', ?2, 0, ?3)",
            params![tid, genus, format!("[\"{}\"]", tid)],
        )
        .ok();
        let sid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO species (id, genus, species_name, common_name, species_code) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sid, genus, sp, "Common", format!("{}-{}", genus, sp)],
        )
        .unwrap();
        let strain_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO strains (id, species_id, name, code, strain_type, status) VALUES (?1, ?2, ?3, ?4, 'wildtype', 'unverified')",
            params![strain_id, sid, format!("{} {} {}", genus, sp, code), code],
        )
        .unwrap();
        strain_id
    }

    /// Seed a breeding program with one selection record, returning the program id.
    fn seed_program(conn: &Connection, name: &str, strain_id: &str, gen: i32, by: &str) -> String {
        let pid = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO breeding_programs (id, name, goal, target_traits) VALUES (?1, ?2, 'Goal', 'traits')",
            params![pid, name],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO breeding_records (id, program_id, strain_id, generation_number, selection_notes, fitness_score, selection_date, selected_by) \
             VALUES (?1, ?2, ?3, ?4, 'good', 8.0, '2026-05-01', ?5)",
            params![uuid::Uuid::new_v4().to_string(), pid, strain_id, gen, by],
        )
        .unwrap();
        pid
    }

    #[test]
    fn export_gathers_and_signs_a_program() {
        let conn = test_db();
        let sid = seed_strain(&conn, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&conn, "Fragrance F1", &sid, 1, "alice");
        let bundle = export_bundle(&conn, &pid, Some("u1")).unwrap();
        let v = verify_bundle(&bundle);
        assert!(v.verified, "{}", v.message);
        assert_eq!(v.record_count, 1);
        assert_eq!(v.program_name, "Fragrance F1");
        assert_eq!(list_bundles(&conn, Some("issued")).unwrap().len(), 1);
    }

    #[test]
    fn export_unknown_program_errs() {
        let conn = test_db();
        assert!(export_bundle(&conn, "nope", Some("u1")).is_err());
    }

    #[test]
    fn preview_marks_records_new_when_program_absent_but_strain_present() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        // Receiver has the strain (shared via registry) but not the program.
        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        let preview = preview_import(&receiver, &json).unwrap();
        assert!(preview.verification.verified);
        assert!(!preview.program_exists_locally);
        assert!(preview.records.iter().all(|p| p.local_status == "new"));
    }

    #[test]
    fn preview_blocks_records_when_strain_absent() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db(); // no strain
        let preview = preview_import(&receiver, &json).unwrap();
        assert!(preview.records.iter().all(|p| p.local_status == "blocked"));
    }

    #[test]
    fn import_creates_program_and_merges_records() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        let result = import_bundle(&receiver, &json, &[], Some("u1")).unwrap();
        assert!(result.imported);
        assert!(result.program_created);
        assert_eq!(result.inserted, 1);
        let programs: i64 = receiver.query_row("SELECT COUNT(*) FROM breeding_programs WHERE name = 'Fragrance F1'", [], |r| r.get(0)).unwrap();
        assert_eq!(programs, 1);
        let recs: i64 = receiver.query_row("SELECT COUNT(*) FROM breeding_records", [], |r| r.get(0)).unwrap();
        assert_eq!(recs, 1);
    }

    #[test]
    fn imported_record_preserves_origin_lab() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        // Origin lab name defaults to the unnamed default; set it so we can assert.
        crate::passport::store::set_lab_name(&origin, "Origin Lab").unwrap();
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        import_bundle(&receiver, &json, &[], Some("u1")).unwrap();
        let origin_lab: String = receiver.query_row("SELECT origin_lab FROM breeding_records LIMIT 1", [], |r| r.get(0)).unwrap();
        assert_eq!(origin_lab, "Origin Lab");
    }

    #[test]
    fn import_blocks_records_when_strain_absent() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db(); // no strain
        let result = import_bundle(&receiver, &json, &[], Some("u1")).unwrap();
        assert_eq!(result.skipped, 1);
        assert_eq!(result.inserted, 0);
        let recs: i64 = receiver.query_row("SELECT COUNT(*) FROM breeding_records", [], |r| r.get(0)).unwrap();
        assert_eq!(recs, 0);
    }

    #[test]
    fn import_folds_into_receiver_audit_chain() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        let before: i64 = receiver.query_row("SELECT COUNT(*) FROM audit_log WHERE entity_type = 'breeding_coordination'", [], |r| r.get(0)).unwrap();
        let result = import_bundle(&receiver, &json, &[], Some("u1")).unwrap();
        assert!(result.audit_entry_id.is_some());
        let after: i64 = receiver.query_row("SELECT COUNT(*) FROM audit_log WHERE entity_type = 'breeding_coordination'", [], |r| r.get(0)).unwrap();
        assert_eq!(after, before + 1);
    }

    #[test]
    fn duplicate_import_is_rejected() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        assert!(import_bundle(&receiver, &json, &[], Some("u1")).is_ok());
        assert!(import_bundle(&receiver, &json, &[], Some("u1")).is_err());
    }

    #[test]
    fn import_rejects_tampered_bundle() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let mut bundle = export_bundle(&origin, &pid, Some("u1")).unwrap();
        bundle.records[0].selection_notes = Some("forged".to_string());
        let json = serde_json::to_string(&bundle).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        assert!(import_bundle(&receiver, &json, &[], Some("u1")).is_err());
        assert_eq!(list_bundles(&receiver, Some("imported")).unwrap().len(), 0);
    }

    #[test]
    fn skip_disposition_inserts_nothing() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let bundle = export_bundle(&origin, &pid, Some("u1")).unwrap();
        let json = serde_json::to_string(&bundle).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        let decisions: Vec<SelectionDecision> = bundle
            .records
            .iter()
            .map(|r| SelectionDecision { source_key: r.source_key.clone(), disposition: "skip".to_string() })
            .collect();
        let result = import_bundle(&receiver, &json, &decisions, Some("u1")).unwrap();
        assert_eq!(result.inserted, 0);
        assert_eq!(result.skipped, 1);
        let recs: i64 = receiver.query_row("SELECT COUNT(*) FROM breeding_records", [], |r| r.get(0)).unwrap();
        assert_eq!(recs, 0);
    }

    #[test]
    fn reimporting_after_merge_marks_records_identical() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        import_bundle(&receiver, &json, &[], Some("u1")).unwrap();
        // A fresh export of the same origin data is a different bundle_id, so the
        // dup guard does not fire; previewing it now shows the record identical.
        let json2 = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();
        let preview = preview_import(&receiver, &json2).unwrap();
        assert!(preview.program_exists_locally);
        assert!(preview.records.iter().all(|p| p.local_status == "identical"));
    }

    #[test]
    fn merge_is_a_union_of_two_labs_records() {
        // Lab A and lab B run the same program with different selections. B imports
        // A's bundle → B ends up with the union.
        let lab_a = test_db();
        let sid_a = seed_strain(&lab_a, "Citrus", "sinensis", "VAL");
        let pid_a = seed_program(&lab_a, "Fragrance F1", &sid_a, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&lab_a, &pid_a, Some("u1")).unwrap()).unwrap();

        let lab_b = test_db();
        let sid_b = seed_strain(&lab_b, "Citrus", "sinensis", "VAL");
        // B has its own generation-2 selection of the same strain.
        seed_program(&lab_b, "Fragrance F1", &sid_b, 2, "bob");
        let result = import_bundle(&lab_b, &json, &[], Some("u1")).unwrap();
        assert!(!result.program_created, "B already had the program");
        assert_eq!(result.inserted, 1, "A's gen-1 record merges in");
        let recs: i64 = lab_b.query_row("SELECT COUNT(*) FROM breeding_records", [], |r| r.get(0)).unwrap();
        assert_eq!(recs, 2, "union of B's gen-2 and A's gen-1");
    }

    #[test]
    fn dispositions_are_recorded_for_an_import() {
        let origin = test_db();
        let sid = seed_strain(&origin, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&origin, "Fragrance F1", &sid, 1, "alice");
        let json = serde_json::to_string(&export_bundle(&origin, &pid, Some("u1")).unwrap()).unwrap();

        let receiver = test_db();
        seed_strain(&receiver, "Citrus", "sinensis", "VAL");
        let result = import_bundle(&receiver, &json, &[], Some("u1")).unwrap();
        let recorded = list_dispositions(&receiver, &result.local_row_id).unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].disposition, "accept");
    }

    #[test]
    fn get_bundle_json_returns_stored_document() {
        let conn = test_db();
        let sid = seed_strain(&conn, "Citrus", "sinensis", "VAL");
        let pid = seed_program(&conn, "Fragrance F1", &sid, 1, "alice");
        export_bundle(&conn, &pid, Some("u1")).unwrap();
        let row = &list_bundles(&conn, Some("issued")).unwrap()[0];
        let json = get_bundle_json(&conn, &row.id).unwrap();
        let parsed = parse_bundle(&json).unwrap();
        assert!(verify_bundle(&parsed).verified);
    }

    #[test]
    fn get_identity_exposes_key() {
        let conn = test_db();
        let id = get_identity(&conn).unwrap();
        assert!(!id.public_key.is_empty());
    }
}
