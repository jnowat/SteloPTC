// WP-71: Shared taxonomy registry — federated, signed reference-data exchange.
//
// A taxonomy registry is a signed, self-contained JSON document carrying a lab's
// shared taxonomy — its genus-and-above `taxa`, its `species`, and its `strains`
// — which any other lab can **verify independently** (no access to the issuing
// lab's database) using only three embedded things:
//   1. the issuing lab's public key (self-attested issuer identity),
//   2. every record's canonical form + `record_hash`, so each record's integrity
//      is recomputable from scratch, and
//   3. an Ed25519 signature over the registry's content hash.
//
// A receiving lab decides, **per record**, one of three dispositions — accept
// (adopt into its own reference tables), override (keep its local version), or
// fork (add a divergent local copy) — and the whole reconciliation is folded into
// the receiver's own tamper-evident audit chain (a `registry_imported` entry that
// commits to the registry's content hash). No central authority: trust flows from
// the issuer's signature and the recomputable per-record hashes alone.
//
// Scope, disclosed honestly (matching the WP-66 "no broadcast" / WP-70 "no
// network transport" precedent): SteloPTC does not run or poll a *subscription
// server*. Exporting a registry produces a signed JSON file the operator moves
// through their own channel; importing reads such a file. The cryptographic
// guarantee is independent of who carries the bytes, so the verifiable core
// ships now and the (credential-bearing, peer-discovery) transport stays out of
// the app. A second boundary, also disclosed: importing is **additive and
// non-destructive** — it never overwrites or deletes an existing local record; it
// inserts records the receiver does not yet have and logs the operator's
// disposition for the rest. And a strain is always imported as `unverified` (a
// foreign lab's `confirmed_genomic` claim is never inherited — the receiver must
// re-confirm locally), matching the Trust-Layer rule that strain confirmation is
// not transferable across labs.
//
// This module is the pure, dependency-light core (no Tauri, no DB): the registry
// data model, its deterministic canonical serialization, per-record + content
// hashing, Ed25519 assembly/signing, and independent verification. The
// connection-level export/import/reconcile lifecycle lives in `registry::store`;
// the thin session/role gating lives in `commands::registry`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compliance_export::signing;
// The self-attested issuer identity (lab name + Ed25519 public key) is the same
// concept as WP-70's passport issuer — reuse it rather than defining a twin.
pub use crate::passport::IssuerIdentity;

pub mod store;

/// Wire-format identifier — distinguishes a SteloPTC taxonomy registry from any
/// other JSON a verifier might be handed.
pub const REGISTRY_FORMAT: &str = "steloptc.taxonomy-registry";
/// Registry format version. Bump only for a structurally different layout; the
/// canonical serialization must stay byte-stable within a version so existing
/// signatures keep verifying.
pub const REGISTRY_VERSION: &str = "1";

/// The three record kinds a registry carries.
pub const RECORD_TAXON: &str = "taxon";
pub const RECORD_SPECIES: &str = "species";
pub const RECORD_STRAIN: &str = "strain";

/// One shared taxonomy record. A flat, type-tagged shape (rather than a nested
/// enum) keeps the canonical serialization and the standalone verifier trivial.
/// `source_key` is a **name-based, cross-lab-stable natural key** — never a local
/// UUID or `taxon_path` (those are lab-local and would never match a peer). The
/// receiver reconciles incoming records against its own by this key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryRecord {
    /// `taxon` | `species` | `strain`.
    pub record_type: String,
    /// Name-based natural key, e.g. `taxon|genus|Citrus`,
    /// `species|Citrus sinensis`, or `strain|Citrus sinensis|VAL-EARLY`.
    pub source_key: String,
    /// Display name (taxon name, species scientific name, or strain name).
    pub name: String,
    /// Taxon rank (`kingdom`..`genus`) for `taxon` records; empty otherwise.
    pub rank: Option<String>,
    /// Parent taxon's name, for a `taxon` record with a parent (context only —
    /// reconciliation keys off `source_key`).
    pub parent_name: Option<String>,
    /// Scientific name (`Genus species`) for `species`/`strain` records.
    pub scientific_name: Option<String>,
    /// `species_code` for a species; strain `code` for a strain.
    pub code: Option<String>,
    /// Strain type (`wildtype`, …) for a `strain` record.
    pub strain_type: Option<String>,
    /// Strain status **as authored by the origin lab** — carried for information
    /// only. A receiver never adopts it: imported strains are always local
    /// `unverified` (see the module note).
    pub status: Option<String>,
    /// Free-text note (species common name, strain provenance).
    pub note: Option<String>,
    /// The lab that authored this record. For a first-hop export this equals the
    /// issuer; it is preserved so a re-exported record keeps its provenance.
    pub origin_lab: String,
    /// SHA-256 (hex) over this record's canonical form (every field above except
    /// `record_hash`). Lets a verifier catch a single tampered record.
    pub record_hash: String,
}

/// The full signed registry document — the JSON that travels between labs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyRegistry {
    pub format: String,
    pub version: String,
    pub registry_id: String,
    pub issued_at: String,
    pub issuer: IssuerIdentity,
    /// Ordered deterministically by `source_key` ascending (stable across
    /// exports of the same data, so re-exports are byte-identical).
    pub records: Vec<RegistryRecord>,
    /// SHA-256 (hex) over the canonical content of everything above.
    pub content_hash: String,
    /// Base64 Ed25519 signature over `content_hash`, by `issuer.public_key`.
    pub signature: String,
}

/// One named verification check, so the UI can show a per-check ✓/✗ list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// The result of independently verifying a registry. `verified` is true only when
/// every check passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryVerification {
    pub verified: bool,
    pub registry_id: String,
    pub issuer_lab: String,
    pub issuer_public_key: String,
    pub record_count: i64,
    pub taxon_count: i64,
    pub species_count: i64,
    pub strain_count: i64,
    pub checks: Vec<RegistryCheck>,
    pub message: String,
}

/// Append one labelled field to the canonical buffer using control-char
/// delimiters (`0x1f` unit separator between label and value, `0x1e` record
/// separator after) that never appear in hex hashes, ISO timestamps, or realistic
/// identity/taxonomy text. Length-independent and unambiguous. Mirrors the WP-70
/// passport canonical form exactly.
fn push_field(buf: &mut Vec<u8>, label: &str, value: &str) {
    buf.extend_from_slice(label.as_bytes());
    buf.push(0x1f);
    buf.extend_from_slice(value.as_bytes());
    buf.push(0x1e);
}

/// Deterministic canonical byte serialization of a single record — every field
/// except `record_hash`. Any change to any committed field changes these bytes
/// and therefore the record hash. Field order is fixed; append new fields at the
/// end only within a version.
pub fn canonical_record_bytes(r: &RegistryRecord) -> Vec<u8> {
    let mut buf = Vec::new();
    push_field(&mut buf, "record_type", &r.record_type);
    push_field(&mut buf, "source_key", &r.source_key);
    push_field(&mut buf, "name", &r.name);
    push_field(&mut buf, "rank", r.rank.as_deref().unwrap_or(""));
    push_field(&mut buf, "parent_name", r.parent_name.as_deref().unwrap_or(""));
    push_field(&mut buf, "scientific_name", r.scientific_name.as_deref().unwrap_or(""));
    push_field(&mut buf, "code", r.code.as_deref().unwrap_or(""));
    push_field(&mut buf, "strain_type", r.strain_type.as_deref().unwrap_or(""));
    push_field(&mut buf, "status", r.status.as_deref().unwrap_or(""));
    push_field(&mut buf, "note", r.note.as_deref().unwrap_or(""));
    push_field(&mut buf, "origin_lab", &r.origin_lab);
    buf
}

/// SHA-256 (lowercase hex) of a record's canonical bytes.
pub fn compute_record_hash(r: &RegistryRecord) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_record_bytes(r));
    format!("{:x}", hasher.finalize())
}

/// Deterministic canonical byte serialization of a registry's content — every
/// field except `content_hash` and `signature`, including each record's committed
/// `record_hash`. Records are hashed in their stored order (the store sorts by
/// `source_key` before signing, so a re-export of unchanged data is byte-stable).
pub fn canonical_content_bytes(reg: &TaxonomyRegistry) -> Vec<u8> {
    let mut buf = Vec::new();
    push_field(&mut buf, "format", &reg.format);
    push_field(&mut buf, "version", &reg.version);
    push_field(&mut buf, "registry_id", &reg.registry_id);
    push_field(&mut buf, "issued_at", &reg.issued_at);
    push_field(&mut buf, "issuer.lab_name", &reg.issuer.lab_name);
    push_field(&mut buf, "issuer.public_key", &reg.issuer.public_key);
    push_field(&mut buf, "records.count", &reg.records.len().to_string());
    for r in &reg.records {
        push_field(&mut buf, "record.source_key", &r.source_key);
        push_field(&mut buf, "record.record_hash", &r.record_hash);
    }
    buf
}

/// SHA-256 (lowercase hex) of the canonical content bytes.
pub fn compute_content_hash(reg: &TaxonomyRegistry) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_content_bytes(reg));
    format!("{:x}", hasher.finalize())
}

/// Assemble and sign a registry from already-gathered records (pure; no DB).
/// Sorts records by `source_key`, fills each record's `record_hash`, then fills
/// the registry `content_hash` and `signature`, so the returned document is
/// complete and independently verifiable. `private_key_b64` must correspond to
/// `issuer.public_key`.
pub fn assemble_and_sign(
    registry_id: String,
    issued_at: String,
    issuer: IssuerIdentity,
    mut records: Vec<RegistryRecord>,
    private_key_b64: &str,
) -> Result<TaxonomyRegistry, String> {
    // Deterministic order → byte-stable re-exports.
    records.sort_by(|a, b| a.source_key.cmp(&b.source_key));
    for r in &mut records {
        r.record_hash = compute_record_hash(r);
    }
    let mut reg = TaxonomyRegistry {
        format: REGISTRY_FORMAT.to_string(),
        version: REGISTRY_VERSION.to_string(),
        registry_id,
        issued_at,
        issuer,
        records,
        content_hash: String::new(),
        signature: String::new(),
    };
    reg.content_hash = compute_content_hash(&reg);
    reg.signature = signing::sign(private_key_b64, reg.content_hash.as_bytes())?;
    Ok(reg)
}

fn count_kind(reg: &TaxonomyRegistry, kind: &str) -> i64 {
    reg.records.iter().filter(|r| r.record_type == kind).count() as i64
}

fn fail(checks: Vec<RegistryCheck>, reg: &TaxonomyRegistry, message: String) -> RegistryVerification {
    RegistryVerification {
        verified: false,
        registry_id: reg.registry_id.clone(),
        issuer_lab: reg.issuer.lab_name.clone(),
        issuer_public_key: reg.issuer.public_key.clone(),
        record_count: reg.records.len() as i64,
        taxon_count: count_kind(reg, RECORD_TAXON),
        species_count: count_kind(reg, RECORD_SPECIES),
        strain_count: count_kind(reg, RECORD_STRAIN),
        checks,
        message,
    }
}

/// Independently verify a registry — no DB, no trust in the issuer's database.
///
/// Checks, in order:
///   1. Format & version are the ones this verifier understands.
///   2. `content_hash` recomputes from the canonical content (no field, and no
///      record's `record_hash`, was edited after signing).
///   3. The Ed25519 signature over `content_hash` verifies against the embedded
///      issuer public key.
///   4. Every record's `record_hash` recomputes from its canonical form, and no
///      two records share a `source_key` (a duplicate key would make
///      reconciliation ambiguous).
pub fn verify_registry(reg: &TaxonomyRegistry) -> RegistryVerification {
    let mut checks: Vec<RegistryCheck> = Vec::new();

    // 1. Format & version.
    if reg.format != REGISTRY_FORMAT {
        return fail(
            vec![RegistryCheck {
                name: "format".to_string(),
                ok: false,
                detail: format!("Unrecognized format '{}' (expected '{}').", reg.format, REGISTRY_FORMAT),
            }],
            reg,
            "Not a SteloPTC taxonomy registry.".to_string(),
        );
    }
    if reg.version != REGISTRY_VERSION {
        return fail(
            vec![RegistryCheck {
                name: "version".to_string(),
                ok: false,
                detail: format!("Unsupported registry version '{}' (expected '{}').", reg.version, REGISTRY_VERSION),
            }],
            reg,
            format!("Unsupported registry version '{}'.", reg.version),
        );
    }
    checks.push(RegistryCheck {
        name: "format".to_string(),
        ok: true,
        detail: format!("{} v{}", REGISTRY_FORMAT, REGISTRY_VERSION),
    });

    // 2. Content hash.
    let recomputed = compute_content_hash(reg);
    if recomputed != reg.content_hash {
        checks.push(RegistryCheck {
            name: "content_hash".to_string(),
            ok: false,
            detail: "The content hash does not match the registry's fields — it was altered after signing.".to_string(),
        });
        return fail(checks, reg, "Content hash mismatch — the registry was tampered with.".to_string());
    }
    checks.push(RegistryCheck {
        name: "content_hash".to_string(),
        ok: true,
        detail: "Recomputed content hash matches.".to_string(),
    });

    // 3. Issuer signature.
    match signing::verify(&reg.issuer.public_key, reg.content_hash.as_bytes(), &reg.signature) {
        Ok(true) => checks.push(RegistryCheck {
            name: "issuer_signature".to_string(),
            ok: true,
            detail: format!("Signed by {}'s key.", reg.issuer.lab_name),
        }),
        Ok(false) => {
            checks.push(RegistryCheck {
                name: "issuer_signature".to_string(),
                ok: false,
                detail: "The signature does not verify against the issuer's public key.".to_string(),
            });
            return fail(checks, reg, "Invalid issuer signature.".to_string());
        }
        Err(e) => {
            checks.push(RegistryCheck {
                name: "issuer_signature".to_string(),
                ok: false,
                detail: format!("Malformed key or signature: {}", e),
            });
            return fail(checks, reg, "Malformed issuer key or signature.".to_string());
        }
    }

    // 4. Per-record hash integrity + unique source keys.
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for r in &reg.records {
        if !seen.insert(r.source_key.as_str()) {
            checks.push(RegistryCheck {
                name: "records".to_string(),
                ok: false,
                detail: format!("Duplicate record key '{}' — reconciliation would be ambiguous.", r.source_key),
            });
            return fail(checks, reg, format!("Duplicate record key '{}'.", r.source_key));
        }
        let computed = compute_record_hash(r);
        if computed != r.record_hash {
            checks.push(RegistryCheck {
                name: "records".to_string(),
                ok: false,
                detail: format!("Tampered record '{}' — recomputed hash does not match.", r.source_key),
            });
            return fail(checks, reg, format!("Tampered record '{}'.", r.source_key));
        }
    }
    checks.push(RegistryCheck {
        name: "records".to_string(),
        ok: true,
        detail: format!(
            "{} record{} intact ({} taxa, {} species, {} strains).",
            reg.records.len(),
            if reg.records.len() == 1 { "" } else { "s" },
            count_kind(reg, RECORD_TAXON),
            count_kind(reg, RECORD_SPECIES),
            count_kind(reg, RECORD_STRAIN),
        ),
    });

    RegistryVerification {
        verified: true,
        registry_id: reg.registry_id.clone(),
        issuer_lab: reg.issuer.lab_name.clone(),
        issuer_public_key: reg.issuer.public_key.clone(),
        record_count: reg.records.len() as i64,
        taxon_count: count_kind(reg, RECORD_TAXON),
        species_count: count_kind(reg, RECORD_SPECIES),
        strain_count: count_kind(reg, RECORD_STRAIN),
        checks,
        message: format!(
            "Registry verified — signed by {} and all {} record{} intact.",
            reg.issuer.lab_name,
            reg.records.len(),
            if reg.records.len() == 1 { "" } else { "s" },
        ),
    }
}

/// Parse a registry from JSON, returning a clear error on malformed input.
pub fn parse_registry(json: &str) -> Result<TaxonomyRegistry, String> {
    serde_json::from_str(json).map_err(|e| format!("Invalid registry JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxon(name: &str, rank: &str, origin: &str) -> RegistryRecord {
        RegistryRecord {
            record_type: RECORD_TAXON.to_string(),
            source_key: format!("taxon|{}|{}", rank, name),
            name: name.to_string(),
            rank: Some(rank.to_string()),
            parent_name: None,
            scientific_name: None,
            code: None,
            strain_type: None,
            status: None,
            note: None,
            origin_lab: origin.to_string(),
            record_hash: String::new(),
        }
    }

    fn species(genus: &str, sp: &str, code: &str, origin: &str) -> RegistryRecord {
        let sci = format!("{} {}", genus, sp);
        RegistryRecord {
            record_type: RECORD_SPECIES.to_string(),
            source_key: format!("species|{}", sci),
            name: sci.clone(),
            rank: None,
            parent_name: Some(genus.to_string()),
            scientific_name: Some(sci),
            code: Some(code.to_string()),
            strain_type: None,
            status: None,
            note: Some("An orange".to_string()),
            origin_lab: origin.to_string(),
            record_hash: String::new(),
        }
    }

    fn strain(genus: &str, sp: &str, code: &str, status: &str, origin: &str) -> RegistryRecord {
        let sci = format!("{} {}", genus, sp);
        RegistryRecord {
            record_type: RECORD_STRAIN.to_string(),
            source_key: format!("strain|{}|{}", sci, code),
            name: format!("{} {}", sci, code),
            rank: None,
            parent_name: None,
            scientific_name: Some(sci),
            code: Some(code.to_string()),
            strain_type: Some("wildtype".to_string()),
            status: Some(status.to_string()),
            note: None,
            origin_lab: origin.to_string(),
            record_hash: String::new(),
        }
    }

    fn sample_registry() -> (TaxonomyRegistry, String) {
        let kp = signing::generate_keypair();
        let records = vec![
            taxon("Citrus", "genus", "Origin Lab"),
            species("Citrus", "sinensis", "CIT-SIN", "Origin Lab"),
            strain("Citrus", "sinensis", "VAL-EARLY", "confirmed_genomic", "Origin Lab"),
        ];
        let reg = assemble_and_sign(
            "reg-1".to_string(),
            "2026-07-11T00:00:00.000Z".to_string(),
            IssuerIdentity { lab_name: "Origin Lab".to_string(), public_key: kp.public_key_b64.clone() },
            records,
            &kp.private_key_b64,
        )
        .unwrap();
        (reg, kp.private_key_b64)
    }

    #[test]
    fn signed_registry_round_trips_and_verifies() {
        let (reg, _) = sample_registry();
        let v = verify_registry(&reg);
        assert!(v.verified, "{}", v.message);
        assert_eq!(v.record_count, 3);
        assert_eq!(v.taxon_count, 1);
        assert_eq!(v.species_count, 1);
        assert_eq!(v.strain_count, 1);
        assert!(v.checks.iter().all(|c| c.ok));
    }

    #[test]
    fn records_are_sorted_by_source_key_for_byte_stability() {
        let (reg, _) = sample_registry();
        let mut sorted = reg.records.clone();
        sorted.sort_by(|a, b| a.source_key.cmp(&b.source_key));
        let keys: Vec<&str> = reg.records.iter().map(|r| r.source_key.as_str()).collect();
        let want: Vec<&str> = sorted.iter().map(|r| r.source_key.as_str()).collect();
        assert_eq!(keys, want);
    }

    #[test]
    fn json_round_trips() {
        let (reg, _) = sample_registry();
        let json = serde_json::to_string_pretty(&reg).unwrap();
        let parsed = parse_registry(&json).unwrap();
        assert!(verify_registry(&parsed).verified);
    }

    #[test]
    fn tampering_with_a_record_field_breaks_record_hash() {
        // Edit a record's descriptive field but leave record_hash — caught by the
        // per-record hash check even before the content hash. Re-sign the outer
        // content so only the inner record check can catch it.
        let (mut reg, priv_key) = sample_registry();
        reg.records[0].name = "Poncirus".to_string();
        reg.content_hash = compute_content_hash(&reg);
        reg.signature = signing::sign(&priv_key, reg.content_hash.as_bytes()).unwrap();
        let v = verify_registry(&reg);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "records" && !c.ok));
    }

    #[test]
    fn tampering_with_a_record_hash_breaks_content_hash() {
        let (mut reg, _) = sample_registry();
        reg.records[0].record_hash = "0".repeat(64);
        let v = verify_registry(&reg);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "content_hash" && !c.ok));
    }

    #[test]
    fn forged_issuer_signature_is_detected() {
        let (mut reg, _) = sample_registry();
        let attacker = signing::generate_keypair();
        reg.signature = signing::sign(&attacker.private_key_b64, reg.content_hash.as_bytes()).unwrap();
        let v = verify_registry(&reg);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "issuer_signature" && !c.ok));
    }

    #[test]
    fn duplicate_source_key_is_rejected() {
        let (mut reg, priv_key) = sample_registry();
        // Duplicate the first record, re-hash it, and re-sign the outer content so
        // only the uniqueness check can catch the collision.
        let mut dup = reg.records[0].clone();
        dup.record_hash = compute_record_hash(&dup);
        reg.records.push(dup);
        reg.content_hash = compute_content_hash(&reg);
        reg.signature = signing::sign(&priv_key, reg.content_hash.as_bytes()).unwrap();
        let v = verify_registry(&reg);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "records" && !c.ok));
    }

    #[test]
    fn wrong_format_is_rejected() {
        let (mut reg, _) = sample_registry();
        reg.format = "something.else".to_string();
        let v = verify_registry(&reg);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "format" && !c.ok));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let (mut reg, _) = sample_registry();
        reg.version = "99".to_string();
        let v = verify_registry(&reg);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "version" && !c.ok));
    }

    #[test]
    fn empty_registry_verifies() {
        let kp = signing::generate_keypair();
        let reg = assemble_and_sign(
            "reg-empty".to_string(),
            "2026-07-11T00:00:00.000Z".to_string(),
            IssuerIdentity { lab_name: "Empty Lab".to_string(), public_key: kp.public_key_b64.clone() },
            vec![],
            &kp.private_key_b64,
        )
        .unwrap();
        let v = verify_registry(&reg);
        assert!(v.verified);
        assert_eq!(v.record_count, 0);
    }
}
