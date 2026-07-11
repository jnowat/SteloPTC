# SteloPTC Specimen Passport

_Specification for the federated specimen passport introduced in WP-70 (Phase G — multi-institutional & federated networks, v1.45.0)._

A **specimen passport** is a signed, self-contained JSON document describing a specimen's
identity and full provenance, which a **receiving lab can verify independently** — without any
access to the originating lab's database — and then **import into its own audit chain**.

---

## 1. Why a passport?

The Trust Layer (WP-18 hash chain, WP-20 Merkle checkpoints, WP-66 on-chain anchoring, WP-67
signed events) makes a specimen's history tamper-evident and non-repudiable **within one lab's
installation**. But tissue-culture material physically moves between labs — a germplasm
repository ships an accession to a commercial propagator, one research group hands a strain to
another. The receiving lab wants to know: *is this the provenance the sender claims, and can I
prove it without trusting the sender's database?*

The specimen passport answers that. It carries three things a receiver can check with nothing
but the file and the sender's public key:

1. **Issuer identity** — the originating lab's name and Ed25519 public key (self-attested).
2. **Provenance** — the specimen's own audit-chain entries in canonical form, so every entry
   hash and the chain linkage can be **recomputed from scratch**.
3. **A signature** — an Ed25519 signature over the passport's content hash.

No central authority is involved. Trust flows from the issuer's signature (which the receiver
decides out-of-band whether to trust, e.g. the labs exchanged keys directly) and from the
recomputable hash chain.

---

## 2. Honest scope — what ships, and what doesn't

SteloPTC **does not transport a passport over any network.** Issuing produces a signed JSON
file the operator moves through their own channel (secure file transfer, email, USB); importing
reads such a file. This is the same "foundation now, credential-bearing transport later"
boundary the roadmap draws for WP-66 (no automatic on-chain broadcast) and WP-51 (no LAN
transport).

The cryptographic guarantee — *a receiver can verify a passport with only the issuer's public
key* — is **independent of who carries the bytes**, so the verifiable core ships now and the
peer-discovery/networking layer stays out of the app.

| Step | Who does it | How |
|---|---|---|
| Build & sign a passport for a local specimen | SteloPTC | `issue_specimen_passport` → downloads JSON |
| Move the file between labs | The operator | Their own channel (out of scope) |
| Verify a received passport (no import) | SteloPTC or any third party | `verify_specimen_passport` / the standalone recipe in §6 |
| Import a verified passport into the receiving lab's audit chain | SteloPTC | `import_specimen_passport` |

---

## 3. Document format

```jsonc
{
  "format": "steloptc.specimen-passport",
  "version": "1",
  "passport_id": "<uuid>",
  "issued_at": "2026-07-11T00:00:05.000Z",
  "issuer": {
    "lab_name": "Green Thumb Labs",
    "public_key": "<base64 Ed25519 public key>"
  },
  "specimen": {
    "specimen_id": "<originating lab's specimen id — also the audit lineage_id>",
    "accession_number": "2026-07-11-CIT-SIN-001",
    "scientific_name": "Citrus sinensis",
    "strain_id": null,
    "stage": "shoot_meristem",
    "generation": 2,
    "origin_type": null,
    "provenance_note": "USDA germplasm collection",
    "initiation_date": "2026-01-01"
  },
  "provenance": [
    {
      "chain_seq": 1,
      "canonical": "spec-1|1|2026-01-01T…|user1|specimen|spec-1|create|created",
      "prev_hash": "0000…0000",
      "entry_hash": "<sha256 hex>"
    }
    // … one object per hashed audit-log entry, ascending chain_seq
  ],
  "merkle_anchor": {                // optional; present only when a checkpoint seals exactly these entries
    "checkpoint_id": "<uuid>",
    "merkle_root": "<sha256 hex>",
    "anchored_txid": "<dogecoin txid or null>"
  },
  "content_hash": "<sha256 hex over the canonical content>",
  "signature": "<base64 Ed25519 signature over content_hash>"
}
```

The `canonical` string of each provenance entry is exactly the WP-18 audit canonical form:

```
lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
```

with NULL optional fields serialized as empty string (see `docs/merkle-proofs.md`).

---

## 4. Content hash & signature

The **content hash** commits to every field except `content_hash` and `signature` itself. It is
`SHA-256` over a deterministic byte serialization built by concatenating, in a fixed order, each
field as `label 0x1F value 0x1E` (unit- and record-separator control bytes that never appear in
the hex hashes, ISO timestamps, or realistic identity text). The exact field order is in
`src-tauri/src/passport/mod.rs::canonical_content_bytes`. Provenance entries are appended after a
`provenance.count` field, each as `entry.chain_seq`, `entry.canonical`, `entry.prev_hash`,
`entry.entry_hash`; the anchor is appended as `anchor.present` = `0`/`1` and, when present, its
three fields.

The **signature** is a detached Ed25519 signature over the ASCII bytes of `content_hash`, made
with the lab-wide signing key (the same key WP-60 uses for regulatory export attestation — one
lab identity, reused). Because the content hash already commits to the whole document, a valid
signature over it authenticates every field.

---

## 5. Verification

`verify_specimen_passport` (and the pure `passport::verify_passport`) runs five checks, in order,
and returns a per-check ✓/✗ list plus an overall verdict:

1. **format & version** — the document is a `steloptc.specimen-passport` this verifier understands.
2. **content_hash** — recomputing it from the fields reproduces the stored value (nothing was
   edited after signing).
3. **issuer_signature** — the Ed25519 signature over `content_hash` verifies against
   `issuer.public_key`.
4. **provenance_chain** — every entry's `entry_hash` recomputes as
   `SHA-256(canonical ‖ prev_hash)`, entries are in ascending `chain_seq`, and each links to the
   previous entry's hash. (The first entry's `prev_hash` is the chain anchor — `ZERO_HASH` for a
   root lineage, or a parent lineage's last hash for a split/forked specimen — and is accepted as
   given, exactly as `verify_audit_lineage` does, since the parent chain is not carried.)
5. **merkle_anchor** _(only if present)_ — the Merkle root rebuilt from the entry hashes equals
   the anchored checkpoint root. If that root was itself anchored on-chain (WP-66), the `txid`
   lets a verifier cross-check it against a public block explorer using the on-chain-anchoring
   recipe.

A passport is `verified` only when every applicable check passes.

---

## 6. Standalone verifier (no SteloPTC required)

A partner lab can verify a passport with ~40 lines of Python and the issuer's public key —
proving the guarantee is genuinely independent of the SteloPTC application. This mirrors the
standalone recipes in `docs/merkle-proofs.md` and `docs/on-chain-anchoring.md`.

```python
import base64, hashlib, json
# pip install pynacl
from nacl.signing import VerifyKey
from nacl.exceptions import BadSignatureError

def sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()

def canonical_content(p: dict) -> bytes:
    buf = bytearray()
    def field(label: str, value: str):
        buf.extend(label.encode()); buf.append(0x1f)
        buf.extend((value or "").encode()); buf.append(0x1e)
    field("format", p["format"]); field("version", p["version"])
    field("passport_id", p["passport_id"]); field("issued_at", p["issued_at"])
    field("issuer.lab_name", p["issuer"]["lab_name"])
    field("issuer.public_key", p["issuer"]["public_key"])
    s = p["specimen"]
    field("specimen.specimen_id", s["specimen_id"])
    field("specimen.accession_number", s["accession_number"])
    field("specimen.scientific_name", s.get("scientific_name") or "")
    field("specimen.strain_id", s.get("strain_id") or "")
    field("specimen.stage", s.get("stage") or "")
    field("specimen.generation", str(s["generation"]))
    field("specimen.origin_type", s.get("origin_type") or "")
    field("specimen.provenance_note", s.get("provenance_note") or "")
    field("specimen.initiation_date", s.get("initiation_date") or "")
    field("provenance.count", str(len(p["provenance"])))
    for e in p["provenance"]:
        field("entry.chain_seq", str(e["chain_seq"]))
        field("entry.canonical", e["canonical"])
        field("entry.prev_hash", e["prev_hash"])
        field("entry.entry_hash", e["entry_hash"])
    a = p.get("merkle_anchor")
    if a:
        field("anchor.present", "1")
        field("anchor.checkpoint_id", a["checkpoint_id"])
        field("anchor.merkle_root", a["merkle_root"])
        field("anchor.anchored_txid", a.get("anchored_txid") or "")
    else:
        field("anchor.present", "0")
    return bytes(buf)

def verify(passport_json: str) -> bool:
    p = json.loads(passport_json)
    assert p["format"] == "steloptc.specimen-passport" and p["version"] == "1"
    # 1. content hash
    assert sha256_hex(canonical_content(p)) == p["content_hash"], "content hash mismatch"
    # 2. issuer signature over the content hash
    vk = VerifyKey(base64.b64decode(p["issuer"]["public_key"]))
    try:
        vk.verify(p["content_hash"].encode(), base64.b64decode(p["signature"]))
    except BadSignatureError:
        raise AssertionError("invalid issuer signature")
    # 3. provenance chain
    prev = None
    for e in p["provenance"]:
        assert sha256_hex(e["canonical"].encode() + e["prev_hash"].encode()) == e["entry_hash"]
        if prev is not None:
            assert e["prev_hash"] == prev, "broken chain linkage"
        prev = e["entry_hash"]
    print("Passport OK — signed by", p["issuer"]["lab_name"],
          "· subject", p["specimen"]["accession_number"])
    return True
```

The Merkle-root check (step 5) reuses the "duplicate-last" tree construction documented in
`docs/merkle-checkpoints.md`.

---

## 7. Importing

`import_specimen_passport` verifies the passport, **refuses an unverifiable or already-imported
one**, and then folds it into the receiving lab's own tamper-evident record: it writes a
`passport_imported` entry into `audit_log` (`entity_type = 'specimen_passport'`, `new_value` =
the passport's `content_hash`). That audit entry starts its own single-entry lineage and is
itself hash-chained, so the receiving lab can later prove *when it accepted the passport and what
content it committed to*. The register (`specimen_passports`, migration 049) records both
directions:

- `issued` — a passport this lab produced for one of its own specimens (kept for re-export).
- `imported` — a passport received, verified, and folded into this lab's audit chain.

`UNIQUE(direction, passport_id)` makes a repeated import a clear error, not a silent duplicate.

---

## 8. Data model (migration 049)

**`specimen_passports`**

| Column | Notes |
|---|---|
| `id` (PK) | local row uuid |
| `passport_id` | the passport document's own uuid |
| `direction` | `issued` \| `imported` (CHECK) |
| `specimen_id` | local specimen (issued only; NULL for imported foreign specimens) |
| `issuer_lab`, `issuer_public_key` | the issuing lab's identity |
| `subject_accession`, `subject_scientific_name` | specimen identity |
| `content_hash` | the signed content hash |
| `entry_count` | number of provenance entries |
| `verified` | 1 once verified (always 1 for stored rows) |
| `audit_entry` | imported: the local `audit_log` row that recorded the import |
| `passport_json` | the full signed document |
| `created_by`, `created_at` | |
| `UNIQUE(direction, passport_id)` | prevents duplicate imports |

---

## 9. Commands

| Command | Role | Purpose |
|---|---|---|
| `get_lab_identity` | any | This lab's issuer name + public key (share out-of-band). |
| `set_lab_name` | manage | Set the issuer name shown in issued passports. |
| `issue_specimen_passport` | write | Build, sign, record, and return a passport for a local specimen. |
| `verify_specimen_passport` | any | Verify a passport JSON with no side effects. |
| `import_specimen_passport` | write | Verify and import, writing the receiving-lab audit entry. |
| `list_specimen_passports` | any | The issued/imported register. |
| `get_specimen_passport_json` | any | Re-export a stored passport's JSON. |

The UI is the **Audit Log → Specimen Passports** panel; specimens also expose an **Issue
Passport** action on their detail page.

---

## 10. Relationship to Phase G

WP-70 is the first packet of Phase G (multi-institutional & federated networks). It provides the
**signed passport format and independent verification**; the reserved follow-ups (WP-71 shared
taxonomy registry, WP-72 cross-lab breeding coordination) build on the same self-attested-key,
recomputable-chain foundation. Nothing here forecloses a future networked transport — it simply
does not bundle one.
