# Shared Taxonomy Registry (WP-71)

*Federated, signed exchange of taxonomy reference data between independent labs.*

A **taxonomy registry** is a signed, self-contained JSON document carrying a lab's shared
taxonomy — its genus-and-above `taxa`, its `species`, and its `strains`. Any other lab can
**verify it independently** (with no access to the issuing lab's database) and then reconcile it
into its own reference tables, deciding **per record** whether to accept, override, or fork.

This is the Phase G companion to the [specimen passport](specimen-passport.md): the passport moves
*one specimen's provenance*; the registry moves *shared reference data*. Both are signed with the
same lab Ed25519 key, both are verifiable with only the issuer's public key, and both ship the
**verifiable core without a network transport** — the same disclosed boundary as WP-66's on-chain
broadcast.

---

## What a registry proves

A verifier, holding only the issuer's public key, can confirm:

1. **Authorship** — the registry was signed by the lab that holds the private key for the embedded
   public key. Nobody else could have produced the signature.
2. **Integrity** — not one record, and no field of any record, was altered after signing. Each
   record carries its own `record_hash`, and the registry's `content_hash` commits to every
   record's hash.

It does **not** assert that the taxonomy is *correct* — only that it is *exactly what the issuing
lab published, unaltered*. Whether to trust the content is the receiver's decision, expressed
through the per-record disposition.

---

## Document shape

```jsonc
{
  "format": "steloptc.taxonomy-registry",
  "version": "1",
  "registry_id": "<uuid>",
  "issued_at": "2026-07-11T00:00:00.000Z",
  "issuer": {
    "lab_name": "Origin Lab",
    "public_key": "<base64 Ed25519 public key>"
  },
  "records": [
    {
      "record_type": "taxon",
      "source_key": "taxon|genus|Citrus",
      "name": "Citrus",
      "rank": "genus",
      "parent_name": null,
      "scientific_name": null,
      "code": null,
      "strain_type": null,
      "status": null,
      "note": null,
      "origin_lab": "Origin Lab",
      "record_hash": "<sha256 hex over this record's canonical form>"
    },
    {
      "record_type": "species",
      "source_key": "species|Citrus sinensis",
      "name": "Citrus sinensis",
      "scientific_name": "Citrus sinensis",
      "parent_name": "Citrus",
      "code": "CIT-SIN",
      "note": "Sweet orange",
      "origin_lab": "Origin Lab",
      "record_hash": "…"
    },
    {
      "record_type": "strain",
      "source_key": "strain|Citrus sinensis|VAL-EARLY",
      "name": "Citrus sinensis VAL-EARLY",
      "scientific_name": "Citrus sinensis",
      "code": "VAL-EARLY",
      "strain_type": "wildtype",
      "status": "confirmed_genomic",
      "origin_lab": "Origin Lab",
      "record_hash": "…"
    }
  ],
  "content_hash": "<sha256 hex over the canonical content>",
  "signature": "<base64 Ed25519 signature over content_hash>"
}
```

### The `source_key` is a name-based, cross-lab natural key

Records are matched between labs by `source_key` — a **name-based** key, never a local UUID or
`taxon_path` (those are lab-local and would never match a peer):

| Record kind | `source_key` form                       | Example                            |
|-------------|-----------------------------------------|------------------------------------|
| taxon       | `taxon\|<rank>\|<name>`                  | `taxon\|genus\|Citrus`             |
| species     | `species\|<Genus species>`              | `species\|Citrus sinensis`         |
| strain      | `strain\|<Genus species>\|<code>`       | `strain\|Citrus sinensis\|VAL-EARLY` |

Records are sorted by `source_key` before signing, so a re-export of unchanged data is
byte-identical.

### The strain `status` is informational only

A strain record carries the origin lab's `status` (e.g. `confirmed_genomic`) **for information
only**. It is never inherited: an imported strain is always created locally as `unverified`, with
the origin lab and its claimed status recorded in `confirmation_basis`. Strain confirmation is not
transferable across labs — the receiving lab must re-confirm identity itself. The genomic
fingerprint is **never** exported.

---

## Canonical forms & hashing

Both hashes use the same control-character canonical form as the rest of the Trust Layer: each
field is `label` `0x1f` `value` `0x1e`. Those bytes never appear in hex hashes, ISO timestamps, or
realistic taxonomy text, so the encoding is unambiguous and length-independent.

**Per-record hash** (`compute_record_hash`) commits to every field of a record *except*
`record_hash`, in this fixed order: `record_type`, `source_key`, `name`, `rank`, `parent_name`,
`scientific_name`, `code`, `strain_type`, `status`, `note`, `origin_lab`. Missing optional fields
serialize as the empty string.

**Content hash** (`compute_content_hash`) commits to `format`, `version`, `registry_id`,
`issued_at`, `issuer.lab_name`, `issuer.public_key`, `records.count`, and then for each record (in
stored order) `record.source_key` and `record.record_hash`. So editing any record's fields breaks
its `record_hash`; editing a `record_hash` (or any header field) breaks the `content_hash`; and the
signature covers the `content_hash`.

The reference implementation is `src-tauri/src/registry/mod.rs`.

---

## Verification checks

`verify_registry` runs, in order:

1. **format / version** — the ones this verifier understands.
2. **content_hash** — recomputing it from the fields reproduces the stored value.
3. **issuer_signature** — the Ed25519 signature over `content_hash` verifies against
   `issuer.public_key`.
4. **records** — every record's `record_hash` recomputes from its canonical form, and no two
   records share a `source_key` (a duplicate would make reconciliation ambiguous).

`verified` is true only when every check passes.

---

## Importing: accept / override / fork

Import is **additive and non-destructive** — it never overwrites or deletes an existing local
record. For each incoming record, the receiver first sees a reconciliation *plan* (local status
`new`, `identical`, or `conflict`) and then chooses a disposition:

| Disposition  | `new` local status                      | `identical` / `conflict`                     |
|--------------|-----------------------------------------|----------------------------------------------|
| **accept**   | insert the record (strain → `unverified`) | keep the local record (nothing overwritten) |
| **override** | insert nothing; keep local              | keep the local record                        |
| **fork**     | insert a divergent local copy (marked)  | insert a divergent local copy (marked)       |

- A **species** whose code collides with a different local species gets a suffixed code
  (`CIT-SIN-2`). A **forked** record gets a `-FORK` code and, for taxa/strains, a `(fork · <lab>)`
  name marker and `local_override = 1`.
- A **strain** can only be inserted when its species already exists locally — accept the species
  first (species records sort before strain records, so accepting both in one import works). If the
  species is absent, the strain is **skipped** and the reason is recorded.

The whole import is folded into the receiver's own tamper-evident audit chain (a
`registry_imported` entry committing to the registry's `content_hash`), and every per-record
decision is recorded in `registry_record_dispositions`.

---

## Standalone verifier (~40 lines, no SteloPTC)

Anyone can verify a registry with only Python and PyNaCl — no SteloPTC install, no database, no
network. This reproduces the exact canonical forms above.

```python
import base64, hashlib, json, sys
from nacl.signing import VerifyKey

def sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()

def _field(buf: bytearray, label: str, value):
    buf.extend(label.encode()); buf.append(0x1f)
    buf.extend((value or "").encode()); buf.append(0x1e)

def canonical_record(r: dict) -> bytes:
    buf = bytearray()
    for label in ("record_type", "source_key", "name", "rank", "parent_name",
                  "scientific_name", "code", "strain_type", "status", "note", "origin_lab"):
        _field(buf, label, r.get(label))
    return bytes(buf)

def canonical_content(reg: dict) -> bytes:
    buf = bytearray()
    _field(buf, "format", reg["format"])
    _field(buf, "version", reg["version"])
    _field(buf, "registry_id", reg["registry_id"])
    _field(buf, "issued_at", reg["issued_at"])
    _field(buf, "issuer.lab_name", reg["issuer"]["lab_name"])
    _field(buf, "issuer.public_key", reg["issuer"]["public_key"])
    _field(buf, "records.count", str(len(reg["records"])))
    for r in reg["records"]:
        _field(buf, "record.source_key", r["source_key"])
        _field(buf, "record.record_hash", r["record_hash"])
    return bytes(buf)

def verify(registry_json: str) -> bool:
    reg = json.loads(registry_json)
    assert reg["format"] == "steloptc.taxonomy-registry", "wrong format"
    assert reg["version"] == "1", "unsupported version"

    # 1. content hash
    assert sha256_hex(canonical_content(reg)) == reg["content_hash"], "content hash mismatch"

    # 2. issuer signature over the content hash
    vk = VerifyKey(base64.b64decode(reg["issuer"]["public_key"]))
    vk.verify(reg["content_hash"].encode(), base64.b64decode(reg["signature"]))  # raises on failure

    # 3. per-record hashes + unique keys
    seen = set()
    for r in reg["records"]:
        assert r["source_key"] not in seen, f"duplicate key {r['source_key']}"
        seen.add(r["source_key"])
        assert sha256_hex(canonical_record(r)) == r["record_hash"], f"tampered record {r['source_key']}"

    print(f"OK — {len(reg['records'])} records signed by {reg['issuer']['lab_name']}")
    return True

if __name__ == "__main__":
    verify(open(sys.argv[1]).read())
```

Run it with `python verify_registry.py taxonomy-registry-xxxxxxxx.json`. A non-zero exit (an
`AssertionError` or a PyNaCl `BadSignatureError`) means the registry was altered or was not signed
by the claimed lab.

---

## Scope, disclosed honestly

Matching the WP-66 ("no broadcast") and WP-70 ("no network transport") precedents:

- **No subscription server.** SteloPTC does not run or poll a taxonomy *server*. Exporting downloads
  a signed JSON file the operator moves through their own channel; importing reads one. The
  cryptographic guarantee is independent of who carries the bytes, so the verifiable core ships now
  and the credential-bearing, peer-discovery transport stays out of the app.
- **Import is additive.** It never overwrites or deletes a local record — it inserts what you don't
  have (accept), a divergent copy (fork), or nothing (override), and logs every decision.
- **Strain confirmation is not transferable.** An imported strain is always `unverified`; a foreign
  lab's `confirmed_genomic` claim is recorded as context but never inherited.

---

## Where it lives

| Layer            | Location                                                              |
|------------------|----------------------------------------------------------------------|
| Pure core        | `src-tauri/src/registry/mod.rs` (model, canonical forms, sign/verify) |
| DB lifecycle     | `src-tauri/src/registry/store.rs` (export / preview / import)         |
| Command gating   | `src-tauri/src/commands/registry.rs`                                  |
| Schema           | migration 050 — `taxonomy_registries` + `registry_record_dispositions` |
| UI               | Audit Log → **Shared Taxonomy Registry** panel                       |
