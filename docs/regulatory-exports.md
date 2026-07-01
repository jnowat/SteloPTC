# Regulatory Compliance Exports (WP-60)

SteloPTC can package its existing cryptographic guarantees (the hash-chained audit log from the Trust Layer, WP-18–21) into three regulator-recognizable formats. All three are **read-only** exports — nothing in the live database is modified by generating one, and none of them submit anything to any external authority. Available from **Compliance → Regulatory Export**, supervisor/admin only.

## 1. FDA 21 CFR Part 11 attestation bundle

**What it's for:** demonstrating that SteloPTC's electronic records meet the "trustworthy, tamper-evident" bar 21 CFR Part 11 requires, for labs subject to FDA inspection (typically cell-culture / biomanufacturing labs).

**Contents of the generated `.zip`:**

| File | Contents |
|---|---|
| `part11_cover.json` | Lab name, system version, export date range, total audit-entry count, and a plain-language attestation statement. |
| `part11_audit_trail.json` | Every `audit_log` entry in the selected date range, in canonical form, with `chain_seq`/`prev_hash`/`entry_hash` for each. |
| `part11_verification.json` | The result of independently re-verifying every hash-chained entry in the range (`{ verified, total_entries_checked, first_break }`). |
| `part11_user_activity.json` | Every user, their role, and how many actions they performed in the range. |
| `*.sig` (one per document above) | A detached Ed25519 signature over that exact file's bytes. |
| `signing_public_key.b64` | The lab's Ed25519 public key, base64-encoded. |

**What satisfies which requirement:**

- *Append-only audit log with tamper evidence* → the SHA-256 hash chain (`chain_seq`/`prev_hash`/`entry_hash`) established in WP-18, independently re-verified for this export's date range and reported in `part11_verification.json`.
- *Access controls / accountability* → `part11_user_activity.json`, backed by the existing role-based access control system (Admin/Supervisor/Tech/Guest) and per-action user attribution in every audit entry.
- *Record authenticity* → the Ed25519 signature over each document, verifiable against the bundled public key without trusting SteloPTC itself at verification time (see **Independent verification** below).

**Why Ed25519 instead of RSA-4096:** the original design sketch called for RSA-4096. Ed25519 gives the same signing/verification guarantee an inspector needs — verify a signature against a bundled public key — with a dramatically smaller, widely-audited pure-Rust dependency and no PEM/ASN.1 certificate machinery. This is a deliberate substitution: SteloPTC's signature is a *self-attestation* (the lab vouching for its own export), not a certificate issued by a trusted third party, so no certificate-authority chain was ever needed.

## 2. USDA APHIS PPQ Form 526 pre-fill

**What it's for:** plant tissue culture labs applying for or renewing a Permit to Move Live Plant Pests, Noxious Weeds, or Soil.

**What it contains:** a JSON document with fields auto-populated from live specimen/species records — scientific name, provenance, source plant, any existing permit number/expiry already on file — plus all `quarantine`-type compliance records for the selected specimens.

**What it does not do:** submit anything to APHIS. SteloPTC produces a ready-to-review package; the authorized scientist named in the export is expected to review, complete any fields SteloPTC can't auto-populate (containment facility details, intended use narrative), and submit through APHIS's own channels.

## 3. CITES Species Provenance Dossier

**What it's for:** demonstrating legal chain of custody for CITES Appendix I/II/III-listed species on export or inspection.

**What it contains:**

- Species identification via the existing WP-49 Darwin Core export (scientific name, taxonomic status, taxon hierarchy).
- The CITES Appendix designation **as confirmed by the user** — SteloPTC does not maintain a live CITES species database, so this is never auto-determined; the wizard requires an explicit selection and displays a reminder to confirm against an official reference (the CITES Secretariat's appendices, updated periodically).
- Full chain of custody: every parent/child specimen relationship reachable from the selected root specimen, with dates, locations, and the responsible user for each transfer.
- Every propagation (subculture) record for the specimen, in chronological order.
- An audit-chain verification summary (same `verify_audit_range` used by the Part 11 export, run over the specimen's full history).

## Independent verification

An inspector does not need SteloPTC installed to check a Part 11 or CITES export's signatures. Given a document (e.g. `part11_audit_trail.json`), its detached signature (`part11_audit_trail.json.sig`), and the bundled `signing_public_key.b64`:

```python
import base64
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
from cryptography.exceptions import InvalidSignature

def verify_document(document_bytes: bytes, signature_b64: str, public_key_b64: str) -> bool:
    public_key = Ed25519PublicKey.from_public_bytes(base64.b64decode(public_key_b64))
    signature = base64.b64decode(signature_b64)
    try:
        public_key.verify(signature, document_bytes)
        return True
    except InvalidSignature:
        return False
```

(Requires only the widely-used `cryptography` package: `pip install cryptography`.)

For the underlying audit-chain hash verification itself (not just the signature over the export), see the standalone Python verifier already documented in [`docs/merkle-proofs.md`](merkle-proofs.md) §8 — the canonical entry format and hash construction are identical; the Part 11/CITES exports use the same `chain_seq`/`prev_hash`/`entry_hash` fields.

## Role gating

Every export command (`export_fda_part11_bundle`, `export_usda_permit`, `export_cites_dossier`) requires supervisor or admin role. `get_signing_public_key` (which lazily generates the lab's Ed25519 keypair on first call if one doesn't exist yet) is also supervisor/admin only.
