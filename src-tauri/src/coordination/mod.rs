// WP-72: Cross-lab breeding program coordination — federated, signed selection-log
// exchange.
//
// Two labs running separate copies of the *same* breeding program (WP-47) each
// accumulate their own selection records. WP-72 lets them merge those records
// periodically without centralizing either database, using the same trust
// mechanics as the WP-70 specimen passport and WP-71 taxonomy registry.
//
// A **breeding-coordination bundle** is a signed, self-contained JSON document
// carrying one program's identity plus its selection records, which any partner
// lab can **verify independently** (no access to the issuing lab's database) using
// only three embedded things:
//   1. the issuing lab's public key (self-attested issuer identity),
//   2. every selection record's canonical form + `record_hash`, so each record's
//      integrity is recomputable from scratch, and
//   3. an Ed25519 signature over the bundle's content hash.
//
// A receiving lab merges the bundle into its own copy of the program: the program
// is matched by its **cross-lab-stable natural key** (its name — never a local
// UUID), each incoming selection record is reconciled against the local program's
// records by a name-based key, and the whole merge is folded into the receiver's
// own tamper-evident audit chain (a `breeding_merge_imported` entry that commits to
// the bundle's content hash). No central authority: trust flows from the issuer's
// signature and the recomputable per-record hashes alone.
//
// Two design boundaries, disclosed honestly (matching the WP-66 "no broadcast" /
// WP-70 / WP-71 precedent):
//
//   * **No network transport.** Exporting a bundle produces a signed JSON file the
//     operator moves through their own channel; importing reads such a file. The
//     cryptographic guarantee is independent of who carries the bytes.
//
//   * **Two dispositions, not three.** WP-71 offered accept/override/fork because a
//     taxonomy record has a *local counterpart* that a receiver might keep or fork.
//     A selection record is an entry in an append-only selection *log*, so merging
//     is a set union: a record is either not-yet-present (accept it) or already
//     present (skip it). There is no local counterpart to "override", and "forking"
//     a log entry is meaningless — so this bundle exposes **accept / skip** only.
//     Importing is additive and non-destructive: it inserts selection records the
//     receiver does not yet have and never overwrites or deletes a local record, and
//     it never overwrites the local program's metadata (an absent program is created
//     as a shell; an existing one is left as-is and merged into).
//
// A selection record references its strain by the strain's **scientific name +
// code** (the same cross-lab-stable identity WP-71 uses), never a local strain
// UUID. On import the receiver resolves that to a local strain; if the strain is
// not present locally the record is skipped with a clear message (import it via the
// taxonomy registry first) — the breeding_records → strains foreign key cannot be
// satisfied otherwise.
//
// This module is the pure, dependency-light core (no Tauri, no DB): the bundle data
// model, its deterministic canonical serialization, per-record + content hashing,
// Ed25519 assembly/signing, and independent verification. The connection-level
// export/preview/import/merge lifecycle lives in `coordination::store`; the thin
// session/role gating lives in `commands::coordination`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compliance_export::signing;
// The self-attested issuer identity (lab name + Ed25519 public key) is the same
// concept as WP-70's passport issuer and WP-71's registry issuer — reuse it.
pub use crate::passport::IssuerIdentity;

pub mod store;

/// Wire-format identifier — distinguishes a SteloPTC breeding-coordination bundle
/// from any other JSON a verifier might be handed.
pub const BUNDLE_FORMAT: &str = "steloptc.breeding-coordination";
/// Bundle format version. Bump only for a structurally different layout; the
/// canonical serialization must stay byte-stable within a version so existing
/// signatures keep verifying.
pub const BUNDLE_VERSION: &str = "1";

/// The program header a bundle carries. Identified across labs by `name` (its
/// natural key); the descriptive fields are context the receiver uses only when
/// creating the program shell for the first time (never to overwrite an existing
/// local program).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleProgram {
    /// Cross-lab-stable natural key — the program name.
    pub name: String,
    pub goal: Option<String>,
    pub target_traits: Option<String>,
    pub start_date: Option<String>,
    pub notes: Option<String>,
    /// The lab that authored this program header.
    pub origin_lab: String,
}

/// One selection record (a WP-47 `breeding_record`) in transit. Flat and
/// type-free to keep the canonical serialization and the standalone verifier
/// trivial. The strain is referenced by scientific name + code — never a local
/// UUID — so the receiver can resolve it against its own strains.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionRecord {
    /// Name-based natural key, e.g.
    /// `sel|Fragrance F1|Citrus sinensis VAL-EARLY|g2|2026-05-01|alice|a1b2c3d4`.
    /// The trailing token is a short content digest so two genuinely distinct
    /// selections of the same strain/generation/date/selector do not collide,
    /// while byte-identical selections from two labs share a key and merge to one.
    pub source_key: String,
    /// The strain's scientific name (`Genus species`), for cross-lab resolution.
    pub strain_scientific_name: String,
    /// The strain's code, for cross-lab resolution.
    pub strain_code: String,
    pub generation_number: i32,
    pub selection_notes: Option<String>,
    pub fitness_score: Option<f64>,
    pub selection_date: Option<String>,
    pub selected_by: Option<String>,
    pub notes: Option<String>,
    /// The lab that authored this selection record.
    pub origin_lab: String,
    /// SHA-256 (hex) over this record's canonical form (every field above except
    /// `record_hash`). Lets a verifier catch a single tampered record.
    pub record_hash: String,
}

/// The full signed coordination bundle — the JSON that travels between labs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationBundle {
    pub format: String,
    pub version: String,
    pub bundle_id: String,
    pub issued_at: String,
    pub issuer: IssuerIdentity,
    pub program: BundleProgram,
    /// Ordered deterministically by `source_key` ascending (stable across exports
    /// of the same data, so re-exports are byte-identical).
    pub records: Vec<SelectionRecord>,
    /// SHA-256 (hex) over the canonical content of everything above.
    pub content_hash: String,
    /// Base64 Ed25519 signature over `content_hash`, by `issuer.public_key`.
    pub signature: String,
}

/// One named verification check, so the UI can show a per-check ✓/✗ list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// The result of independently verifying a bundle. `verified` is true only when
/// every check passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleVerification {
    pub verified: bool,
    pub bundle_id: String,
    pub issuer_lab: String,
    pub issuer_public_key: String,
    pub program_name: String,
    pub record_count: i64,
    pub checks: Vec<BundleCheck>,
    pub message: String,
}

/// Append one labelled field to the canonical buffer using control-char delimiters
/// (`0x1f` unit separator between label and value, `0x1e` record separator after)
/// that never appear in hex hashes, ISO timestamps, or realistic identity/breeding
/// text. Length-independent and unambiguous. Mirrors the WP-70/WP-71 canonical form
/// exactly.
fn push_field(buf: &mut Vec<u8>, label: &str, value: &str) {
    buf.extend_from_slice(label.as_bytes());
    buf.push(0x1f);
    buf.extend_from_slice(value.as_bytes());
    buf.push(0x1e);
}

/// Canonical byte form of an `Option<f64>` — empty for `None`, else the shortest
/// round-tripping decimal (Rust's default `f64` Display). Deterministic across
/// platforms for the finite values a fitness score ever holds.
fn fnum(v: Option<f64>) -> String {
    match v {
        Some(n) => n.to_string(),
        None => String::new(),
    }
}

/// Deterministic canonical byte serialization of a single selection record — every
/// field except `record_hash`. Any change to any committed field changes these
/// bytes and therefore the record hash. Field order is fixed; append new fields at
/// the end only within a version.
pub fn canonical_record_bytes(r: &SelectionRecord) -> Vec<u8> {
    let mut buf = Vec::new();
    push_field(&mut buf, "source_key", &r.source_key);
    push_field(&mut buf, "strain_scientific_name", &r.strain_scientific_name);
    push_field(&mut buf, "strain_code", &r.strain_code);
    push_field(&mut buf, "generation_number", &r.generation_number.to_string());
    push_field(&mut buf, "selection_notes", r.selection_notes.as_deref().unwrap_or(""));
    push_field(&mut buf, "fitness_score", &fnum(r.fitness_score));
    push_field(&mut buf, "selection_date", r.selection_date.as_deref().unwrap_or(""));
    push_field(&mut buf, "selected_by", r.selected_by.as_deref().unwrap_or(""));
    push_field(&mut buf, "notes", r.notes.as_deref().unwrap_or(""));
    push_field(&mut buf, "origin_lab", &r.origin_lab);
    buf
}

/// SHA-256 (lowercase hex) of a record's canonical bytes.
pub fn compute_record_hash(r: &SelectionRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_record_bytes(r));
    format!("{:x}", hasher.finalize())
}

/// A short (8 hex char) content digest used to disambiguate a selection record's
/// natural key. Derived from the fields that distinguish two selections sharing the
/// same strain/generation/date/selector — so identical content from two labs
/// yields the same token (and merges to one), while differing content diverges.
pub fn content_discriminator(
    selection_notes: Option<&str>,
    fitness_score: Option<f64>,
    notes: Option<&str>,
) -> String {
    let mut buf = Vec::new();
    push_field(&mut buf, "selection_notes", selection_notes.unwrap_or(""));
    push_field(&mut buf, "fitness_score", &fnum(fitness_score));
    push_field(&mut buf, "notes", notes.unwrap_or(""));
    let mut hasher = Sha256::new();
    hasher.update(&buf);
    format!("{:x}", hasher.finalize())[..8].to_string()
}

/// Build a selection record's cross-lab-stable natural key from its identifying
/// fields. Deterministic and free of any local id.
pub fn build_source_key(
    program_name: &str,
    strain_scientific_name: &str,
    strain_code: &str,
    generation_number: i32,
    selection_date: Option<&str>,
    selected_by: Option<&str>,
    discriminator: &str,
) -> String {
    format!(
        "sel|{}|{} {}|g{}|{}|{}|{}",
        program_name,
        strain_scientific_name,
        strain_code,
        generation_number,
        selection_date.unwrap_or(""),
        selected_by.unwrap_or(""),
        discriminator,
    )
}

/// Deterministic canonical byte serialization of a bundle's content — every field
/// except `content_hash` and `signature`, including the program header and each
/// record's committed `record_hash`. Records are hashed in their stored order (the
/// store sorts by `source_key` before signing, so a re-export of unchanged data is
/// byte-stable).
pub fn canonical_content_bytes(b: &CoordinationBundle) -> Vec<u8> {
    let mut buf = Vec::new();
    push_field(&mut buf, "format", &b.format);
    push_field(&mut buf, "version", &b.version);
    push_field(&mut buf, "bundle_id", &b.bundle_id);
    push_field(&mut buf, "issued_at", &b.issued_at);
    push_field(&mut buf, "issuer.lab_name", &b.issuer.lab_name);
    push_field(&mut buf, "issuer.public_key", &b.issuer.public_key);
    push_field(&mut buf, "program.name", &b.program.name);
    push_field(&mut buf, "program.goal", b.program.goal.as_deref().unwrap_or(""));
    push_field(&mut buf, "program.target_traits", b.program.target_traits.as_deref().unwrap_or(""));
    push_field(&mut buf, "program.start_date", b.program.start_date.as_deref().unwrap_or(""));
    push_field(&mut buf, "program.notes", b.program.notes.as_deref().unwrap_or(""));
    push_field(&mut buf, "program.origin_lab", &b.program.origin_lab);
    push_field(&mut buf, "records.count", &b.records.len().to_string());
    for r in &b.records {
        push_field(&mut buf, "record.source_key", &r.source_key);
        push_field(&mut buf, "record.record_hash", &r.record_hash);
    }
    buf
}

/// SHA-256 (lowercase hex) of the canonical content bytes.
pub fn compute_content_hash(b: &CoordinationBundle) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_content_bytes(b));
    format!("{:x}", hasher.finalize())
}

/// Assemble and sign a bundle from an already-gathered program header + records
/// (pure; no DB). Sorts records by `source_key`, fills each record's `record_hash`,
/// then fills the bundle `content_hash` and `signature`, so the returned document
/// is complete and independently verifiable. `private_key_b64` must correspond to
/// `issuer.public_key`.
pub fn assemble_and_sign(
    bundle_id: String,
    issued_at: String,
    issuer: IssuerIdentity,
    program: BundleProgram,
    mut records: Vec<SelectionRecord>,
    private_key_b64: &str,
) -> Result<CoordinationBundle, String> {
    // Deterministic order → byte-stable re-exports.
    records.sort_by(|a, b| a.source_key.cmp(&b.source_key));
    for r in &mut records {
        r.record_hash = compute_record_hash(r);
    }
    let mut bundle = CoordinationBundle {
        format: BUNDLE_FORMAT.to_string(),
        version: BUNDLE_VERSION.to_string(),
        bundle_id,
        issued_at,
        issuer,
        program,
        records,
        content_hash: String::new(),
        signature: String::new(),
    };
    bundle.content_hash = compute_content_hash(&bundle);
    bundle.signature = signing::sign(private_key_b64, bundle.content_hash.as_bytes())?;
    Ok(bundle)
}

fn fail(checks: Vec<BundleCheck>, b: &CoordinationBundle, message: String) -> BundleVerification {
    BundleVerification {
        verified: false,
        bundle_id: b.bundle_id.clone(),
        issuer_lab: b.issuer.lab_name.clone(),
        issuer_public_key: b.issuer.public_key.clone(),
        program_name: b.program.name.clone(),
        record_count: b.records.len() as i64,
        checks,
        message,
    }
}

/// Independently verify a bundle — no DB, no trust in the issuer's database.
///
/// Checks, in order:
///   1. Format & version are the ones this verifier understands.
///   2. `content_hash` recomputes from the canonical content (no field, and no
///      record's `record_hash`, was edited after signing).
///   3. The Ed25519 signature over `content_hash` verifies against the embedded
///      issuer public key.
///   4. Every record's `record_hash` recomputes from its canonical form, and no
///      two records share a `source_key` (a duplicate key would make merge
///      reconciliation ambiguous).
pub fn verify_bundle(b: &CoordinationBundle) -> BundleVerification {
    let mut checks: Vec<BundleCheck> = Vec::new();

    // 1. Format & version.
    if b.format != BUNDLE_FORMAT {
        return fail(
            vec![BundleCheck {
                name: "format".to_string(),
                ok: false,
                detail: format!("Unrecognized format '{}' (expected '{}').", b.format, BUNDLE_FORMAT),
            }],
            b,
            "Not a SteloPTC breeding-coordination bundle.".to_string(),
        );
    }
    if b.version != BUNDLE_VERSION {
        return fail(
            vec![BundleCheck {
                name: "version".to_string(),
                ok: false,
                detail: format!("Unsupported bundle version '{}' (expected '{}').", b.version, BUNDLE_VERSION),
            }],
            b,
            format!("Unsupported bundle version '{}'.", b.version),
        );
    }
    checks.push(BundleCheck {
        name: "format".to_string(),
        ok: true,
        detail: format!("{} v{}", BUNDLE_FORMAT, BUNDLE_VERSION),
    });

    // 2. Content hash.
    let recomputed = compute_content_hash(b);
    if recomputed != b.content_hash {
        checks.push(BundleCheck {
            name: "content_hash".to_string(),
            ok: false,
            detail: "The content hash does not match the bundle's fields — it was altered after signing.".to_string(),
        });
        return fail(checks, b, "Content hash mismatch — the bundle was tampered with.".to_string());
    }
    checks.push(BundleCheck {
        name: "content_hash".to_string(),
        ok: true,
        detail: "Recomputed content hash matches.".to_string(),
    });

    // 3. Issuer signature.
    match signing::verify(&b.issuer.public_key, b.content_hash.as_bytes(), &b.signature) {
        Ok(true) => checks.push(BundleCheck {
            name: "issuer_signature".to_string(),
            ok: true,
            detail: format!("Signed by {}'s key.", b.issuer.lab_name),
        }),
        Ok(false) => {
            checks.push(BundleCheck {
                name: "issuer_signature".to_string(),
                ok: false,
                detail: "The signature does not verify against the issuer's public key.".to_string(),
            });
            return fail(checks, b, "Invalid issuer signature.".to_string());
        }
        Err(e) => {
            checks.push(BundleCheck {
                name: "issuer_signature".to_string(),
                ok: false,
                detail: format!("Malformed key or signature: {}", e),
            });
            return fail(checks, b, "Malformed issuer key or signature.".to_string());
        }
    }

    // 4. Per-record hash integrity + unique source keys.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for r in &b.records {
        if !seen.insert(r.source_key.as_str()) {
            checks.push(BundleCheck {
                name: "records".to_string(),
                ok: false,
                detail: format!("Duplicate record key '{}' — merge reconciliation would be ambiguous.", r.source_key),
            });
            return fail(checks, b, format!("Duplicate record key '{}'.", r.source_key));
        }
        let computed = compute_record_hash(r);
        if computed != r.record_hash {
            checks.push(BundleCheck {
                name: "records".to_string(),
                ok: false,
                detail: format!("Tampered record '{}' — recomputed hash does not match.", r.source_key),
            });
            return fail(checks, b, format!("Tampered record '{}'.", r.source_key));
        }
    }
    checks.push(BundleCheck {
        name: "records".to_string(),
        ok: true,
        detail: format!(
            "{} selection record{} intact.",
            b.records.len(),
            if b.records.len() == 1 { "" } else { "s" },
        ),
    });

    BundleVerification {
        verified: true,
        bundle_id: b.bundle_id.clone(),
        issuer_lab: b.issuer.lab_name.clone(),
        issuer_public_key: b.issuer.public_key.clone(),
        program_name: b.program.name.clone(),
        record_count: b.records.len() as i64,
        checks,
        message: format!(
            "Bundle verified — program '{}' signed by {}, all {} record{} intact.",
            b.program.name,
            b.issuer.lab_name,
            b.records.len(),
            if b.records.len() == 1 { "" } else { "s" },
        ),
    }
}

/// Parse a bundle from JSON, returning a clear error on malformed input.
pub fn parse_bundle(json: &str) -> Result<CoordinationBundle, String> {
    serde_json::from_str(json).map_err(|e| format!("Invalid coordination bundle JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(code: &str, gen: i32, notes: &str, fitness: f64, date: &str, by: &str) -> SelectionRecord {
        let strain = "Citrus sinensis";
        let disc = content_discriminator(Some(notes), Some(fitness), None);
        let key = build_source_key("Fragrance F1", strain, code, gen, Some(date), Some(by), &disc);
        SelectionRecord {
            source_key: key,
            strain_scientific_name: strain.to_string(),
            strain_code: code.to_string(),
            generation_number: gen,
            selection_notes: Some(notes.to_string()),
            fitness_score: Some(fitness),
            selection_date: Some(date.to_string()),
            selected_by: Some(by.to_string()),
            notes: None,
            origin_lab: "Origin Lab".to_string(),
            record_hash: String::new(),
        }
    }

    fn sample_bundle() -> (CoordinationBundle, String) {
        let kp = signing::generate_keypair();
        let program = BundleProgram {
            name: "Fragrance F1".to_string(),
            goal: Some("Higher terpene yield".to_string()),
            target_traits: Some("aroma, vigor".to_string()),
            start_date: Some("2026-01-01".to_string()),
            notes: None,
            origin_lab: "Origin Lab".to_string(),
        };
        let records = vec![
            record("VAL-EARLY", 1, "vigorous", 8.5, "2026-03-01", "alice"),
            record("VAL-EARLY", 2, "best aroma", 9.1, "2026-05-01", "bob"),
        ];
        let bundle = assemble_and_sign(
            "bundle-1".to_string(),
            "2026-07-11T00:00:00.000Z".to_string(),
            IssuerIdentity { lab_name: "Origin Lab".to_string(), public_key: kp.public_key_b64.clone() },
            program,
            records,
            &kp.private_key_b64,
        )
        .unwrap();
        (bundle, kp.private_key_b64)
    }

    #[test]
    fn signed_bundle_round_trips_and_verifies() {
        let (b, _) = sample_bundle();
        let v = verify_bundle(&b);
        assert!(v.verified, "{}", v.message);
        assert_eq!(v.record_count, 2);
        assert_eq!(v.program_name, "Fragrance F1");
        assert!(v.checks.iter().all(|c| c.ok));
    }

    #[test]
    fn records_are_sorted_by_source_key_for_byte_stability() {
        let (b, _) = sample_bundle();
        let mut sorted = b.records.clone();
        sorted.sort_by(|a, c| a.source_key.cmp(&c.source_key));
        let keys: Vec<&str> = b.records.iter().map(|r| r.source_key.as_str()).collect();
        let want: Vec<&str> = sorted.iter().map(|r| r.source_key.as_str()).collect();
        assert_eq!(keys, want);
    }

    #[test]
    fn json_round_trips() {
        let (b, _) = sample_bundle();
        let json = serde_json::to_string_pretty(&b).unwrap();
        let parsed = parse_bundle(&json).unwrap();
        assert!(verify_bundle(&parsed).verified);
    }

    #[test]
    fn tampering_with_a_record_field_breaks_record_hash() {
        let (mut b, priv_key) = sample_bundle();
        b.records[0].selection_notes = Some("forged".to_string());
        b.content_hash = compute_content_hash(&b);
        b.signature = signing::sign(&priv_key, b.content_hash.as_bytes()).unwrap();
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "records" && !c.ok));
    }

    #[test]
    fn tampering_with_program_metadata_breaks_content_hash() {
        let (mut b, _) = sample_bundle();
        b.program.goal = Some("something else".to_string());
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "content_hash" && !c.ok));
    }

    #[test]
    fn tampering_with_a_record_hash_breaks_content_hash() {
        let (mut b, _) = sample_bundle();
        b.records[0].record_hash = "0".repeat(64);
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "content_hash" && !c.ok));
    }

    #[test]
    fn forged_issuer_signature_is_detected() {
        let (mut b, _) = sample_bundle();
        let attacker = signing::generate_keypair();
        b.signature = signing::sign(&attacker.private_key_b64, b.content_hash.as_bytes()).unwrap();
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "issuer_signature" && !c.ok));
    }

    #[test]
    fn duplicate_source_key_is_rejected() {
        let (mut b, priv_key) = sample_bundle();
        let mut dup = b.records[0].clone();
        dup.record_hash = compute_record_hash(&dup);
        b.records.push(dup);
        b.content_hash = compute_content_hash(&b);
        b.signature = signing::sign(&priv_key, b.content_hash.as_bytes()).unwrap();
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "records" && !c.ok));
    }

    #[test]
    fn wrong_format_is_rejected() {
        let (mut b, _) = sample_bundle();
        b.format = "something.else".to_string();
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "format" && !c.ok));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let (mut b, _) = sample_bundle();
        b.version = "99".to_string();
        let v = verify_bundle(&b);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "version" && !c.ok));
    }

    #[test]
    fn empty_program_bundle_verifies() {
        let kp = signing::generate_keypair();
        let program = BundleProgram {
            name: "Empty Program".to_string(),
            goal: None,
            target_traits: None,
            start_date: None,
            notes: None,
            origin_lab: "Empty Lab".to_string(),
        };
        let b = assemble_and_sign(
            "bundle-empty".to_string(),
            "2026-07-11T00:00:00.000Z".to_string(),
            IssuerIdentity { lab_name: "Empty Lab".to_string(), public_key: kp.public_key_b64.clone() },
            program,
            vec![],
            &kp.private_key_b64,
        )
        .unwrap();
        let v = verify_bundle(&b);
        assert!(v.verified);
        assert_eq!(v.record_count, 0);
    }

    #[test]
    fn identical_content_yields_identical_discriminator() {
        let a = content_discriminator(Some("note"), Some(9.0), None);
        let b = content_discriminator(Some("note"), Some(9.0), None);
        assert_eq!(a, b);
        let c = content_discriminator(Some("different"), Some(9.0), None);
        assert_ne!(a, c);
    }
}
