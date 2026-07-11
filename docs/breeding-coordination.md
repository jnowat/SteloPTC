# Cross-Lab Breeding Program Coordination (WP-72)

*Federated, signed merging of breeding-program selection records between independent labs.*

Two labs that collaborate on the **same breeding program** each run their own SteloPTC
installation and accumulate their own selection records (WP-47 `breeding_records`). A
**coordination bundle** is a signed, self-contained JSON document carrying one program's identity
plus its selection records. Any partner lab can **verify it independently** (with no access to the
issuing lab's database) and then **merge** it into its own copy of the program, deciding **per
record** whether to accept or skip.

This is the final Phase G packet, and the companion to the [specimen passport](specimen-passport.md)
and the [taxonomy registry](taxonomy-registry.md): the passport moves *one specimen's provenance*,
the registry moves *shared reference data*, and this bundle moves *a program's selection log*. All
three are signed with the same lab Ed25519 key, all are verifiable with only the issuer's public
key, and all ship the **verifiable core without a network transport** — the same disclosed boundary
as WP-66's on-chain broadcast.

---

## What a bundle proves

A verifier, holding only the issuer's public key, can confirm:

1. **Authorship** — the bundle was signed by the lab that holds the private key for the embedded
   public key. Nobody else could have produced the signature.
2. **Integrity** — not one selection record, and no field of any record (nor the program header),
   was altered after signing. Each record carries its own `record_hash`, and the bundle's
   `content_hash` commits to the program header and every record's hash.

It does **not** assert that the selections are *good breeding decisions* — only that they are
*exactly what the issuing lab recorded, unaltered*. Whether to merge each record is the receiver's
decision, expressed through the per-record disposition.

---

## Document shape

```jsonc
{
  "format": "steloptc.breeding-coordination",
  "version": "1",
  "bundle_id": "<uuid>",
  "issued_at": "2026-07-11T00:00:00.000Z",
  "issuer": {
    "lab_name": "Origin Lab",
    "public_key": "<base64 Ed25519 public key>"
  },
  "program": {
    "name": "Fragrance F1",
    "goal": "Higher terpene yield",
    "target_traits": "aroma, vigor",
    "start_date": "2026-01-01",
    "notes": null,
    "origin_lab": "Origin Lab"
  },
  "records": [
    {
      "source_key": "sel|Fragrance F1|Citrus sinensis VAL-EARLY|g1|2026-03-01|alice|1a2b3c4d",
      "strain_scientific_name": "Citrus sinensis",
      "strain_code": "VAL-EARLY",
      "generation_number": 1,
      "selection_notes": "vigorous",
      "fitness_score": 8.5,
      "selection_date": "2026-03-01",
      "selected_by": "alice",
      "notes": null,
      "origin_lab": "Origin Lab",
      "record_hash": "<sha256 hex over this record's canonical form>"
    }
  ],
  "content_hash": "<sha256 hex over the canonical content>",
  "signature": "<base64 Ed25519 signature over content_hash>"
}
```

### The program is matched by name; the strain by scientific name + code

The bundle carries **one** program, identified across labs by its **name** — the cross-lab-stable
natural key (never a local UUID). Each selection record references its strain by
`strain_scientific_name` + `strain_code` (the same cross-lab identity the taxonomy registry uses),
so a receiver can resolve it against its own strains without sharing any local id.

### The `source_key` is a name-based, cross-lab natural key

Selection records are matched between labs by `source_key`:

```
sel|<program>|<Genus species> <code>|g<generation>|<selection_date>|<selected_by>|<content8>
```

The trailing `content8` is the first 8 hex characters of a SHA-256 over the record's
`selection_notes`, `fitness_score`, and `notes`. It disambiguates two genuinely distinct selections
that share the same strain / generation / date / selector, while byte-identical selections from two
labs produce the *same* key and merge to one. Records are sorted by `source_key` before signing, so
a re-export of unchanged data is byte-identical.

### `origin_lab` preserves provenance across hops

Every record carries the lab that authored it. A locally-authored record carries this lab's name; a
record previously merged in from a partner keeps *that* partner's name, so re-exporting a merged
program does not reattribute another lab's selections. Locally, a merged record is stored with its
`origin_lab` in `breeding_records.origin_lab` (`NULL` means locally authored).

---

## Canonical forms & hashing

Both hashes use the same control-character canonical form as the rest of the Trust Layer: each
field is `label` `0x1f` `value` `0x1e`. Those bytes never appear in hex hashes, ISO timestamps, or
realistic breeding text, so the encoding is unambiguous and length-independent. A `None` optional
serializes as the empty string; a `fitness_score` serializes as its shortest round-tripping decimal
(and empty when absent).

**Per-record hash** (`compute_record_hash`) commits to every field of a record *except*
`record_hash`, in this fixed order: `source_key`, `strain_scientific_name`, `strain_code`,
`generation_number`, `selection_notes`, `fitness_score`, `selection_date`, `selected_by`, `notes`,
`origin_lab`.

**Content hash** (`compute_content_hash`) commits to `format`, `version`, `bundle_id`, `issued_at`,
`issuer.lab_name`, `issuer.public_key`, the program header (`program.name`, `program.goal`,
`program.target_traits`, `program.start_date`, `program.notes`, `program.origin_lab`),
`records.count`, and then for each record (in stored order) `record.source_key` and
`record.record_hash`. So editing any record's fields breaks its `record_hash`; editing a
`record_hash`, a program field, or any header field breaks the `content_hash`; and the signature
covers the `content_hash`.

The reference implementation is `src-tauri/src/coordination/mod.rs`.

---

## Verification checks

`verify_bundle` runs, in order:

1. **format / version** — the ones this verifier understands.
2. **content_hash** — recomputing it from the fields reproduces the stored value.
3. **issuer_signature** — the Ed25519 signature over `content_hash` verifies against
   `issuer.public_key`.
4. **records** — every record's `record_hash` recomputes from its canonical form, and no two
   records share a `source_key` (a duplicate would make merge reconciliation ambiguous).

`verified` is true only when every check passes.

---

## Importing: accept / skip

Merging a selection *log* is a **set union**, so a bundle exposes only two dispositions — unlike the
taxonomy registry's three (accept/override/fork). A selection record has no local counterpart to
"override", and "forking" a log entry is meaningless. For each incoming record, the receiver first
sees a reconciliation *plan* (local status `new`, `identical`, or `blocked`) and then chooses:

| Disposition | `new` local status                         | `identical` | `blocked`                    |
|-------------|--------------------------------------------|-------------|------------------------------|
| **accept**  | merge the record into the local program    | keep local  | skipped (strain not present) |
| **skip**    | insert nothing                             | keep local  | skipped                      |

- A record is **`blocked`** when its strain is not present locally. Selection records reference a
  strain by scientific name + code, and `breeding_records.strain_id` is a foreign key — so the
  strain must exist first. Share it via the [Taxonomy Registry](taxonomy-registry.md), then re-import.
- If the local copy of the program **does not exist**, importing creates a *coordinated copy* — a
  program shell with the bundle's name and metadata — so records have somewhere to attach. An
  **existing** local program is matched by name and left untouched; only its records are merged.
  Importing never overwrites a local program's metadata or an existing local selection record.

The whole import is folded into the receiver's own tamper-evident audit chain (a
`breeding_merge_imported` entry committing to the bundle's `content_hash`), and every per-record
decision is recorded in `breeding_bundle_dispositions`.

---

## Standalone verifier (~40 lines, no SteloPTC)

Anyone can verify a bundle with only Python and PyNaCl — no SteloPTC install, no database, no
network. This reproduces the exact canonical forms above.

```python
import base64, hashlib, json, sys
from nacl.signing import VerifyKey

def sha256_hex(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()

def _field(buf: bytearray, label: str, value):
    buf.extend(label.encode()); buf.append(0x1f)
    buf.extend((value if value is not None else "").encode()); buf.append(0x1e)

def _num(v):
    # Shortest round-tripping decimal, matching Rust's f64 Display; empty when absent.
    if v is None:
        return ""
    return repr(v) if v != int(v) else str(int(v))

def canonical_record(r: dict) -> bytes:
    buf = bytearray()
    _field(buf, "source_key", r["source_key"])
    _field(buf, "strain_scientific_name", r["strain_scientific_name"])
    _field(buf, "strain_code", r["strain_code"])
    _field(buf, "generation_number", str(r["generation_number"]))
    _field(buf, "selection_notes", r.get("selection_notes"))
    _field(buf, "fitness_score", _num(r.get("fitness_score")))
    _field(buf, "selection_date", r.get("selection_date"))
    _field(buf, "selected_by", r.get("selected_by"))
    _field(buf, "notes", r.get("notes"))
    _field(buf, "origin_lab", r["origin_lab"])
    return bytes(buf)

def canonical_content(b: dict) -> bytes:
    buf = bytearray()
    _field(buf, "format", b["format"])
    _field(buf, "version", b["version"])
    _field(buf, "bundle_id", b["bundle_id"])
    _field(buf, "issued_at", b["issued_at"])
    _field(buf, "issuer.lab_name", b["issuer"]["lab_name"])
    _field(buf, "issuer.public_key", b["issuer"]["public_key"])
    p = b["program"]
    _field(buf, "program.name", p["name"])
    _field(buf, "program.goal", p.get("goal"))
    _field(buf, "program.target_traits", p.get("target_traits"))
    _field(buf, "program.start_date", p.get("start_date"))
    _field(buf, "program.notes", p.get("notes"))
    _field(buf, "program.origin_lab", p["origin_lab"])
    _field(buf, "records.count", str(len(b["records"])))
    for r in b["records"]:
        _field(buf, "record.source_key", r["source_key"])
        _field(buf, "record.record_hash", r["record_hash"])
    return bytes(buf)

def verify(bundle_json: str) -> bool:
    b = json.loads(bundle_json)
    assert b["format"] == "steloptc.breeding-coordination", "wrong format"
    assert b["version"] == "1", "unsupported version"

    # 1. content hash
    assert sha256_hex(canonical_content(b)) == b["content_hash"], "content hash mismatch"

    # 2. issuer signature over the content hash
    vk = VerifyKey(base64.b64decode(b["issuer"]["public_key"]))
    vk.verify(b["content_hash"].encode(), base64.b64decode(b["signature"]))  # raises on failure

    # 3. per-record hashes + unique keys
    seen = set()
    for r in b["records"]:
        assert r["source_key"] not in seen, f"duplicate key {r['source_key']}"
        seen.add(r["source_key"])
        assert sha256_hex(canonical_record(r)) == r["record_hash"], f"tampered record {r['source_key']}"

    print(f"OK — program '{b['program']['name']}', {len(b['records'])} records signed by {b['issuer']['lab_name']}")
    return True

if __name__ == "__main__":
    verify(open(sys.argv[1]).read())
```

Run it with `python verify_bundle.py breeding-coordination-xxxxxxxx.json`. A non-zero exit (an
`AssertionError` or a PyNaCl `BadSignatureError`) means the bundle was altered or was not signed by
the claimed lab.

> The `_num` helper matches Rust's `f64` Display for the finite fitness scores this field holds
> (e.g. `8.5`, `9.0` → `9`). If your workflow uses fitness scores with many decimal places, compare
> the recomputed `record_hash` produced by SteloPTC itself as the source of truth.

---

## Scope, disclosed honestly

Matching the WP-66 ("no broadcast"), WP-70, and WP-71 ("no network transport") precedents:

- **No coordination server.** SteloPTC does not run or poll a breeding-coordination *server*.
  Exporting downloads a signed JSON file the operator moves through their own channel; importing
  reads one. The cryptographic guarantee is independent of who carries the bytes.
- **Merging is additive.** It never overwrites or deletes a local selection record, and never
  changes a local program's metadata — it inserts the records you don't yet have (accept) or nothing
  (skip), and logs every decision.
- **Two dispositions, not three.** Merging an append-only selection log is a set union; there is no
  local counterpart to "override" and forking a log entry is meaningless — so the bundle exposes
  accept / skip only.
- **Strains are a prerequisite.** A selection record can only merge when its strain already exists
  locally; otherwise it is `blocked` until the strain is shared via the taxonomy registry.

---

## Where it lives

| Layer            | Location                                                                    |
|------------------|-----------------------------------------------------------------------------|
| Pure core        | `src-tauri/src/coordination/mod.rs` (model, canonical forms, sign/verify)   |
| DB lifecycle     | `src-tauri/src/coordination/store.rs` (export / preview / import / merge)    |
| Command gating   | `src-tauri/src/commands/coordination.rs`                                     |
| Schema           | migration 051 — `breeding_bundles` + `breeding_bundle_dispositions` + `breeding_records.origin_lab` |
| UI               | Audit Log → **Cross-Lab Breeding Coordination** panel                       |
