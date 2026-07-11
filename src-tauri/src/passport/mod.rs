// WP-70: Federated identity & inter-lab specimen transfer — the "specimen
// passport".
//
// A specimen passport is a signed, self-contained JSON document describing a
// specimen's identity and full provenance, which a **receiving lab can verify
// independently** — without any access to the originating lab's database — using
// only three embedded things:
//   1. the originating lab's public key (self-attested issuer identity),
//   2. the specimen's own audit-chain entries (canonical form + hashes, so every
//      entry hash and the chain linkage can be recomputed from scratch), and
//   3. an Ed25519 signature over the passport's content hash.
//
// When a receiving lab accepts a passport it is **imported into that lab's own
// audit chain** (a `passport_imported` entry that commits to the passport's
// content hash), so the receiver's tamper-evident record now attests "we
// received and verified this passport at time T". No central authority is
// involved — trust flows from the issuer's signature and the recomputable hash
// chain alone.
//
// Scope, disclosed honestly (matching the WP-66 "no broadcast" / WP-51 "no
// network transport" precedent): SteloPTC does not *transport* a passport over
// any network. Issuing produces a signed JSON file the operator moves through
// their own channel (secure file transfer, email, USB); importing reads such a
// file. The cryptographic guarantee — a receiver can verify a passport with only
// the issuer's public key — is independent of who carries the bytes, so the
// verifiable core ships now and the (credential-bearing, peer-discovery)
// transport stays out of the app, exactly like WP-66's broadcast step.
//
// This module is the pure, dependency-light core (no Tauri, no DB): the passport
// data model, its deterministic canonical serialization, content hashing,
// Ed25519 assembly/signing, and independent verification. The connection-level
// issue/import/list lifecycle lives in `passport::store`; the thin session/role
// gating lives in `commands::passport`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::compliance_export::signing;
use crate::db::queries::{build_merkle_root, compute_entry_hash};

pub mod store;

/// Wire-format identifier — distinguishes a SteloPTC specimen passport from any
/// other JSON a verifier might be handed.
pub const PASSPORT_FORMAT: &str = "steloptc.specimen-passport";
/// Passport format version. Bump only for a structurally different layout; the
/// canonical serialization must stay byte-stable within a version so existing
/// signatures keep verifying.
pub const PASSPORT_VERSION: &str = "1";

/// The self-attested identity of the issuing lab. There is no certificate
/// authority: the receiver decides out-of-band whether it trusts this public key
/// (e.g. the labs exchanged keys directly), exactly as with the WP-60 export
/// signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssuerIdentity {
    /// Human-readable lab name (informational; part of the signed content).
    pub lab_name: String,
    /// Base64 Ed25519 public key the signature verifies against.
    pub public_key: String,
}

/// The identity subset of the specimen carried in the passport. This is a
/// snapshot for interpretation by the receiver; the authoritative provenance is
/// the `provenance` chain below.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportSpecimen {
    /// Originating lab's specimen id (also the audit `lineage_id`).
    pub specimen_id: String,
    pub accession_number: String,
    pub scientific_name: Option<String>,
    pub strain_id: Option<String>,
    pub stage: Option<String>,
    pub generation: i32,
    pub origin_type: Option<String>,
    pub provenance_note: Option<String>,
    pub initiation_date: Option<String>,
}

/// One audit entry embedded in a passport's provenance, in the exact shape a
/// verifier needs to recompute its hash and chain linkage. Mirrors WP-21's
/// `ProofEntry` minus the Merkle path (the passport verifies the whole set at
/// once against the optional anchor).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportAuditEntry {
    pub chain_seq: i64,
    /// Pipe-separated canonical form:
    /// `lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details`
    pub canonical: String,
    pub prev_hash: String,
    pub entry_hash: String,
}

/// Optional cross-reference to a Merkle checkpoint (WP-20) that seals the exact
/// entries in this passport, plus any on-chain anchor txid (WP-66). Present only
/// when a checkpoint covers precisely the exported entry set, so rebuilding the
/// root from the passport's entry hashes always reproduces `merkle_root`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportMerkleAnchor {
    pub checkpoint_id: String,
    pub merkle_root: String,
    /// The Dogecoin (or other chain) txid, if the checkpoint root was anchored
    /// on-chain via WP-66. Informational — a verifier confirms it against a block
    /// explorer using the WP-66 recipe; the passport itself only carries it.
    pub anchored_txid: Option<String>,
}

/// The full signed passport document — the JSON that travels between labs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecimenPassport {
    pub format: String,
    pub version: String,
    pub passport_id: String,
    pub issued_at: String,
    pub issuer: IssuerIdentity,
    pub specimen: PassportSpecimen,
    /// Ordered by `chain_seq` ascending.
    pub provenance: Vec<PassportAuditEntry>,
    pub merkle_anchor: Option<PassportMerkleAnchor>,
    /// SHA-256 (hex) over the canonical content of everything above.
    pub content_hash: String,
    /// Base64 Ed25519 signature over `content_hash`, by `issuer.public_key`.
    pub signature: String,
}

/// One named verification check, so the UI can show a per-check ✓/✗ list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// The result of independently verifying a passport. `verified` is true only when
/// every check passes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassportVerification {
    pub verified: bool,
    pub passport_id: String,
    pub issuer_lab: String,
    pub issuer_public_key: String,
    pub subject_accession: String,
    pub subject_scientific_name: Option<String>,
    pub entry_count: i64,
    pub checks: Vec<PassportCheck>,
    pub message: String,
}

/// Append one labelled field to the canonical buffer using control-char
/// delimiters (`0x1f` unit separator between label and value, `0x1e` record
/// separator after) that never appear in the hex hashes, ISO timestamps, or
/// realistic identity text this commits to. Length-independent and unambiguous.
fn push_field(buf: &mut Vec<u8>, label: &str, value: &str) {
    buf.extend_from_slice(label.as_bytes());
    buf.push(0x1f);
    buf.extend_from_slice(value.as_bytes());
    buf.push(0x1e);
}

/// Deterministic canonical byte serialization of a passport's content — every
/// field except `content_hash` and `signature`. Any change to any committed
/// field changes these bytes and therefore the content hash and signature. Field
/// order is fixed; append new fields at the end only within a version.
pub fn canonical_content_bytes(p: &SpecimenPassport) -> Vec<u8> {
    let mut buf = Vec::new();
    push_field(&mut buf, "format", &p.format);
    push_field(&mut buf, "version", &p.version);
    push_field(&mut buf, "passport_id", &p.passport_id);
    push_field(&mut buf, "issued_at", &p.issued_at);
    push_field(&mut buf, "issuer.lab_name", &p.issuer.lab_name);
    push_field(&mut buf, "issuer.public_key", &p.issuer.public_key);

    push_field(&mut buf, "specimen.specimen_id", &p.specimen.specimen_id);
    push_field(&mut buf, "specimen.accession_number", &p.specimen.accession_number);
    push_field(&mut buf, "specimen.scientific_name", p.specimen.scientific_name.as_deref().unwrap_or(""));
    push_field(&mut buf, "specimen.strain_id", p.specimen.strain_id.as_deref().unwrap_or(""));
    push_field(&mut buf, "specimen.stage", p.specimen.stage.as_deref().unwrap_or(""));
    push_field(&mut buf, "specimen.generation", &p.specimen.generation.to_string());
    push_field(&mut buf, "specimen.origin_type", p.specimen.origin_type.as_deref().unwrap_or(""));
    push_field(&mut buf, "specimen.provenance_note", p.specimen.provenance_note.as_deref().unwrap_or(""));
    push_field(&mut buf, "specimen.initiation_date", p.specimen.initiation_date.as_deref().unwrap_or(""));

    push_field(&mut buf, "provenance.count", &p.provenance.len().to_string());
    for e in &p.provenance {
        push_field(&mut buf, "entry.chain_seq", &e.chain_seq.to_string());
        push_field(&mut buf, "entry.canonical", &e.canonical);
        push_field(&mut buf, "entry.prev_hash", &e.prev_hash);
        push_field(&mut buf, "entry.entry_hash", &e.entry_hash);
    }

    match &p.merkle_anchor {
        Some(a) => {
            push_field(&mut buf, "anchor.present", "1");
            push_field(&mut buf, "anchor.checkpoint_id", &a.checkpoint_id);
            push_field(&mut buf, "anchor.merkle_root", &a.merkle_root);
            push_field(&mut buf, "anchor.anchored_txid", a.anchored_txid.as_deref().unwrap_or(""));
        }
        None => push_field(&mut buf, "anchor.present", "0"),
    }
    buf
}

/// SHA-256 (lowercase hex) of the canonical content bytes.
pub fn compute_content_hash(p: &SpecimenPassport) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_content_bytes(p));
    format!("{:x}", hasher.finalize())
}

/// Assemble and sign a passport from already-gathered data (pure; no DB). Fills
/// `content_hash` and `signature`, so the returned document is complete and
/// independently verifiable. `private_key_b64` must correspond to
/// `issuer.public_key`.
#[allow(clippy::too_many_arguments)]
pub fn assemble_and_sign(
    passport_id: String,
    issued_at: String,
    issuer: IssuerIdentity,
    specimen: PassportSpecimen,
    provenance: Vec<PassportAuditEntry>,
    merkle_anchor: Option<PassportMerkleAnchor>,
    private_key_b64: &str,
) -> Result<SpecimenPassport, String> {
    let mut passport = SpecimenPassport {
        format: PASSPORT_FORMAT.to_string(),
        version: PASSPORT_VERSION.to_string(),
        passport_id,
        issued_at,
        issuer,
        specimen,
        provenance,
        merkle_anchor,
        content_hash: String::new(),
        signature: String::new(),
    };
    passport.content_hash = compute_content_hash(&passport);
    passport.signature = signing::sign(private_key_b64, passport.content_hash.as_bytes())?;
    Ok(passport)
}

fn fail(checks: Vec<PassportCheck>, p: &SpecimenPassport, message: String) -> PassportVerification {
    PassportVerification {
        verified: false,
        passport_id: p.passport_id.clone(),
        issuer_lab: p.issuer.lab_name.clone(),
        issuer_public_key: p.issuer.public_key.clone(),
        subject_accession: p.specimen.accession_number.clone(),
        subject_scientific_name: p.specimen.scientific_name.clone(),
        entry_count: p.provenance.len() as i64,
        checks,
        message,
    }
}

/// Independently verify a passport — no DB, no trust in the issuer's database.
///
/// Checks, in order:
///   1. Format & version are the ones this verifier understands.
///   2. `content_hash` recomputes from the canonical content (no field was edited
///      after signing).
///   3. The Ed25519 signature over `content_hash` verifies against the embedded
///      issuer public key (the holder of the issuer's private key produced it).
///   4. The provenance hash chain is internally consistent: each entry's stored
///      `entry_hash` recomputes from its canonical form + `prev_hash`, entries are
///      in ascending `chain_seq`, and each links to the previous entry's hash.
///   5. If a Merkle anchor is present, the root rebuilt from the entry hashes
///      equals the anchor's `merkle_root`.
///
/// Note (matching `verify_audit_lineage`): the first provenance entry's
/// `prev_hash` is the chain's anchor (ZERO_HASH for a root lineage, or a parent
/// lineage's last hash for a forked/split specimen). It cannot be verified
/// without the parent chain, so it is accepted as given — every *subsequent* link
/// and every entry hash within the passport is fully checked.
pub fn verify_passport(p: &SpecimenPassport) -> PassportVerification {
    let mut checks: Vec<PassportCheck> = Vec::new();

    // 1. Format & version.
    if p.format != PASSPORT_FORMAT {
        return fail(
            vec![PassportCheck {
                name: "format".to_string(),
                ok: false,
                detail: format!("Unrecognized format '{}' (expected '{}').", p.format, PASSPORT_FORMAT),
            }],
            p,
            "Not a SteloPTC specimen passport.".to_string(),
        );
    }
    if p.version != PASSPORT_VERSION {
        return fail(
            vec![PassportCheck {
                name: "version".to_string(),
                ok: false,
                detail: format!("Unsupported passport version '{}' (expected '{}').", p.version, PASSPORT_VERSION),
            }],
            p,
            format!("Unsupported passport version '{}'.", p.version),
        );
    }
    checks.push(PassportCheck {
        name: "format".to_string(),
        ok: true,
        detail: format!("{} v{}", PASSPORT_FORMAT, PASSPORT_VERSION),
    });

    // 2. Content hash.
    let recomputed = compute_content_hash(p);
    if recomputed != p.content_hash {
        checks.push(PassportCheck {
            name: "content_hash".to_string(),
            ok: false,
            detail: "The content hash does not match the passport's fields — it was altered after signing.".to_string(),
        });
        return fail(checks, p, "Content hash mismatch — the passport was tampered with.".to_string());
    }
    checks.push(PassportCheck {
        name: "content_hash".to_string(),
        ok: true,
        detail: "Recomputed content hash matches.".to_string(),
    });

    // 3. Issuer signature.
    match signing::verify(&p.issuer.public_key, p.content_hash.as_bytes(), &p.signature) {
        Ok(true) => checks.push(PassportCheck {
            name: "issuer_signature".to_string(),
            ok: true,
            detail: format!("Signed by {}'s key.", p.issuer.lab_name),
        }),
        Ok(false) => {
            checks.push(PassportCheck {
                name: "issuer_signature".to_string(),
                ok: false,
                detail: "The signature does not verify against the issuer's public key.".to_string(),
            });
            return fail(checks, p, "Invalid issuer signature.".to_string());
        }
        Err(e) => {
            checks.push(PassportCheck {
                name: "issuer_signature".to_string(),
                ok: false,
                detail: format!("Malformed key or signature: {}", e),
            });
            return fail(checks, p, "Malformed issuer key or signature.".to_string());
        }
    }

    // 4. Provenance hash chain.
    let mut expected_prev: Option<&str> = None;
    let mut prev_seq: Option<i64> = None;
    for e in &p.provenance {
        // Ascending, gapless-tolerant ordering (a lineage seq may skip via forks,
        // but must strictly increase).
        if let Some(ps) = prev_seq {
            if e.chain_seq <= ps {
                checks.push(PassportCheck {
                    name: "provenance_chain".to_string(),
                    ok: false,
                    detail: format!("Entries out of order at seq {} (must be ascending).", e.chain_seq),
                });
                return fail(checks, p, format!("Provenance entries out of order at seq {}.", e.chain_seq));
            }
        }
        // Linkage (skipped for the first entry, whose prev_hash is the un-carried anchor).
        if let Some(prev_hash) = expected_prev {
            if e.prev_hash != prev_hash {
                checks.push(PassportCheck {
                    name: "provenance_chain".to_string(),
                    ok: false,
                    detail: format!("Broken chain linkage at seq {} — prev_hash does not match the preceding entry.", e.chain_seq),
                });
                return fail(checks, p, format!("Provenance chain broken at seq {}.", e.chain_seq));
            }
        }
        // Content hash of the entry.
        let computed = compute_entry_hash(e.canonical.as_bytes(), &e.prev_hash);
        if computed != e.entry_hash {
            checks.push(PassportCheck {
                name: "provenance_chain".to_string(),
                ok: false,
                detail: format!("Tampered provenance entry at seq {} — recomputed hash does not match.", e.chain_seq),
            });
            return fail(checks, p, format!("Tampered provenance entry at seq {}.", e.chain_seq));
        }
        expected_prev = Some(&e.entry_hash);
        prev_seq = Some(e.chain_seq);
    }
    checks.push(PassportCheck {
        name: "provenance_chain".to_string(),
        ok: true,
        detail: format!(
            "{} provenance {} verified.",
            p.provenance.len(),
            if p.provenance.len() == 1 { "entry" } else { "entries" }
        ),
    });

    // 5. Optional Merkle anchor.
    if let Some(anchor) = &p.merkle_anchor {
        let leaves: Vec<String> = p.provenance.iter().map(|e| e.entry_hash.clone()).collect();
        let root = build_merkle_root(&leaves);
        if root != anchor.merkle_root {
            checks.push(PassportCheck {
                name: "merkle_anchor".to_string(),
                ok: false,
                detail: "The Merkle root rebuilt from the provenance does not match the anchored checkpoint root.".to_string(),
            });
            return fail(checks, p, "Merkle anchor mismatch.".to_string());
        }
        checks.push(PassportCheck {
            name: "merkle_anchor".to_string(),
            ok: true,
            detail: match &anchor.anchored_txid {
                Some(txid) => format!("Root matches checkpoint; anchored on-chain (txid {}).", txid),
                None => "Root matches the sealed checkpoint.".to_string(),
            },
        });
    }

    PassportVerification {
        verified: true,
        passport_id: p.passport_id.clone(),
        issuer_lab: p.issuer.lab_name.clone(),
        issuer_public_key: p.issuer.public_key.clone(),
        subject_accession: p.specimen.accession_number.clone(),
        subject_scientific_name: p.specimen.scientific_name.clone(),
        entry_count: p.provenance.len() as i64,
        checks,
        message: format!(
            "Passport verified — signed by {} and all {} provenance {} intact.",
            p.issuer.lab_name,
            p.provenance.len(),
            if p.provenance.len() == 1 { "entry" } else { "entries" }
        ),
    }
}

/// Parse a passport from JSON, returning a clear error on malformed input.
pub fn parse_passport(json: &str) -> Result<SpecimenPassport, String> {
    serde_json::from_str(json).map_err(|e| format!("Invalid passport JSON: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::queries::ZERO_HASH;

    /// Build a small, valid provenance chain of `n` entries anchored at ZERO_HASH,
    /// mirroring how the real audit chain is hashed.
    fn sample_chain(lineage: &str, n: i64) -> Vec<PassportAuditEntry> {
        let mut entries = Vec::new();
        let mut prev = ZERO_HASH.to_string();
        for seq in 0..n {
            let canonical = format!(
                "{}|{}|2026-07-11T00:00:0{}.000Z|user1|specimen|{}|{}|",
                lineage, seq, seq, lineage, if seq == 0 { "create" } else { "passage" }
            );
            let entry_hash = compute_entry_hash(canonical.as_bytes(), &prev);
            entries.push(PassportAuditEntry {
                chain_seq: seq,
                canonical,
                prev_hash: prev.clone(),
                entry_hash: entry_hash.clone(),
            });
            prev = entry_hash;
        }
        entries
    }

    fn sample_passport(with_anchor: bool) -> (SpecimenPassport, String) {
        let kp = signing::generate_keypair();
        let provenance = sample_chain("spec-1", 3);
        let anchor = if with_anchor {
            let leaves: Vec<String> = provenance.iter().map(|e| e.entry_hash.clone()).collect();
            Some(PassportMerkleAnchor {
                checkpoint_id: "cp-1".to_string(),
                merkle_root: build_merkle_root(&leaves),
                anchored_txid: Some("abc123".to_string()),
            })
        } else {
            None
        };
        let passport = assemble_and_sign(
            "passport-1".to_string(),
            "2026-07-11T00:00:05.000Z".to_string(),
            IssuerIdentity { lab_name: "Origin Lab".to_string(), public_key: kp.public_key_b64.clone() },
            PassportSpecimen {
                specimen_id: "spec-1".to_string(),
                accession_number: "2026-07-11-CIT-SIN-001".to_string(),
                scientific_name: Some("Citrus sinensis".to_string()),
                strain_id: None,
                stage: Some("shoot_meristem".to_string()),
                generation: 2,
                origin_type: None,
                provenance_note: Some("USDA germplasm".to_string()),
                initiation_date: Some("2026-01-01".to_string()),
            },
            provenance,
            anchor,
            &kp.private_key_b64,
        )
        .unwrap();
        (passport, kp.private_key_b64)
    }

    #[test]
    fn signed_passport_round_trips_and_verifies() {
        let (passport, _) = sample_passport(false);
        let v = verify_passport(&passport);
        assert!(v.verified, "{}", v.message);
        assert_eq!(v.entry_count, 3);
        assert!(v.checks.iter().all(|c| c.ok));
    }

    #[test]
    fn passport_with_matching_merkle_anchor_verifies() {
        let (passport, _) = sample_passport(true);
        let v = verify_passport(&passport);
        assert!(v.verified, "{}", v.message);
        assert!(v.checks.iter().any(|c| c.name == "merkle_anchor" && c.ok));
    }

    #[test]
    fn json_round_trips() {
        let (passport, _) = sample_passport(true);
        let json = serde_json::to_string_pretty(&passport).unwrap();
        let parsed = parse_passport(&json).unwrap();
        assert!(verify_passport(&parsed).verified);
    }

    #[test]
    fn tampering_with_specimen_identity_breaks_content_hash() {
        let (mut passport, _) = sample_passport(false);
        passport.specimen.accession_number = "FORGED-999".to_string();
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "content_hash" && !c.ok));
    }

    #[test]
    fn tampering_with_a_field_then_rehashing_still_fails_signature() {
        // A forger who edits a field AND recomputes content_hash still cannot
        // produce a valid signature without the issuer's private key.
        let (mut passport, _) = sample_passport(false);
        passport.specimen.scientific_name = Some("Forged species".to_string());
        passport.content_hash = compute_content_hash(&passport);
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "issuer_signature" && !c.ok));
    }

    #[test]
    fn forged_issuer_signature_is_detected() {
        let (mut passport, _) = sample_passport(false);
        let attacker = signing::generate_keypair();
        // Attacker re-signs the real content hash with their own key but cannot
        // change issuer.public_key without also breaking the content hash.
        passport.signature = signing::sign(&attacker.private_key_b64, passport.content_hash.as_bytes()).unwrap();
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "issuer_signature" && !c.ok));
    }

    #[test]
    fn tampered_provenance_entry_is_detected() {
        let (mut passport, priv_key) = sample_passport(false);
        // Edit a provenance entry's canonical form, then re-sign the outer content
        // so only the inner chain check can catch it.
        passport.provenance[1].canonical = passport.provenance[1].canonical.replace("passage", "delete");
        passport.content_hash = compute_content_hash(&passport);
        passport.signature = signing::sign(&priv_key, passport.content_hash.as_bytes()).unwrap();
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "provenance_chain" && !c.ok));
    }

    #[test]
    fn broken_merkle_anchor_is_detected() {
        let (mut passport, priv_key) = sample_passport(true);
        passport.merkle_anchor.as_mut().unwrap().merkle_root = "0".repeat(64);
        passport.content_hash = compute_content_hash(&passport);
        passport.signature = signing::sign(&priv_key, passport.content_hash.as_bytes()).unwrap();
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "merkle_anchor" && !c.ok));
    }

    #[test]
    fn wrong_format_is_rejected() {
        let (mut passport, _) = sample_passport(false);
        passport.format = "something.else".to_string();
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "format" && !c.ok));
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let (mut passport, _) = sample_passport(false);
        passport.version = "99".to_string();
        let v = verify_passport(&passport);
        assert!(!v.verified);
        assert!(v.checks.iter().any(|c| c.name == "version" && !c.ok));
    }
}
