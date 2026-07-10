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

---

# Regulatory Submission Pipeline (WP-68)

_Added in v1.44.0. Builds directly on the export bundles above._

The **submission pipeline** turns a one-off export into a monitored lifecycle: it evaluates whether a submission's preconditions are met against live compliance state, generates and signs the bundle when it's ready, and tracks the submission through to a recorded external reference.

## Honest scope

SteloPTC produces a **ready-to-submit, signed package** — it does **not** electronically submit to a government web portal. Automated portal submission needs authenticated portal credentials, per-agency form APIs, and legal authorization that vary by jurisdiction; that is out of scope (and out of a specimen tracker). This is the same boundary WP-60 already draws ("SteloPTC does not submit this form to APHIS directly"). The operator downloads the signed package, submits it through the official channel, and records the returned reference number in the pipeline.

## Lifecycle

Each submission is a row in `regulatory_submissions` (migration 048) with a status:

| Status | Meaning |
|---|---|
| `ready` | Last readiness evaluation passed every check |
| `blocked` | One or more readiness checks failed (see the stored `readiness` JSON) |
| `generated` | The signed package was produced (`package_path` + top-level `package_signature`) |
| `submitted` | The operator submitted it externally and recorded a `submission_reference` |
| `acknowledged` | Reserved for a future authority-confirmation step |

## Readiness checks

`evaluate_submission_readiness(kind, scope)` runs kind-specific, read-only checks:

- **Part 11** — a valid `from`/`to` date range; the range contains audit entries; the audit hash chain verifies over the range (reusing `verify_audit_range`); at least one user exists.
- **USDA** — the lab profile is plant tissue culture; at least one specimen in scope; every specimen exists and has a scientific name; **no specimen has an expired permit** (a blocking compliance issue).
- **CITES** — the root specimen exists; a CITES Appendix is confirmed; the audit hash chain verifies (chain-of-custody integrity).

A submission is `ready` only when every check passes.

## Generation & signing

`generate_submission_package(submission_id)` re-checks readiness, assembles the WP-60 documents for the kind, signs each document, zips them with the public key (the same `sign_and_zip` path the exports use), adds a **top-level detached Ed25519 signature over the exact `.zip` artifact** (stored as `package_signature`), writes the package under `compliance_exports/submissions/`, and advances the submission to `generated`.

## Automated monitoring

`run_submission_monitor()` (and the same `monitor()` call wired into the background scheduler on each tick) re-evaluates every non-terminal submission against current compliance state and **auto-generates** the package for any that is now `ready` and flagged `auto_generate`. This is what makes the pipeline "monitor compliance state and, when conditions are met, generate and sign" without manual intervention.

## Role gating

Every submission command (`evaluate_submission_readiness`, `create_submission`, `reevaluate_submission`, `generate_submission_package`, `mark_submission_submitted`, `list_submissions`, `run_submission_monitor`) requires supervisor or admin role. The UI lives in **Compliance → Submission Pipeline**.
