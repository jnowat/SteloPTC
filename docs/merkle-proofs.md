# SteloPTC Portable Merkle Proofs

_Specification for the proof format introduced in WP-21 (v1.10.0)._

This document describes the JSON proof format exported by **Export Proof** in the Audit Log
UI, and the three-stage algorithm used to verify it — both inside SteloPTC and with the
standalone Python verifier at the end of this file.

For the underlying Merkle-checkpoint architecture (hash-chaining, checkpoint creation,
`verify_against_checkpoint`) see `docs/merkle-checkpoints.md`.

---

## 1. Purpose

A **portable Merkle proof** is a self-contained JSON file that lets any party verify the
integrity of a range of one specimen lineage's audit history without access to the SteloPTC
application or database.

Each proof bundles:
* The sealed checkpoint metadata (lineage, seq range, entry count, Merkle root).
* Every audit entry in that range with its **canonical pipe-delimited form**, `prev_hash`,
  `entry_hash`, and an **individual Merkle inclusion path** from the leaf to the root.

---

## 2. Proof JSON format

```json
{
  "version": "1",
  "exported_at": "2026-01-15T09:30:00.000Z",
  "checkpoint": {
    "id":          "550e8400-e29b-41d4-a716-446655440000",
    "lineage_id":  "3f2a1c7b-...",
    "start_seq":   1,
    "end_seq":     42,
    "entry_count": 42,
    "merkle_root": "a3b4c5d6...",
    "created_at":  "2026-01-10T00:00:00.000Z"
  },
  "entries": [
    {
      "chain_seq":   1,
      "canonical":   "3f2a1c7b-...|1|2026-01-01T10:00:00.000Z|user-uuid|specimen|3f2a1c7b-...|create|",
      "prev_hash":   "0000000000000000000000000000000000000000000000000000000000000000",
      "entry_hash":  "e3b0c44298fc1c14...",
      "merkle_path": [
        { "sibling_hash": "abc123...", "position": "right" }
      ]
    }
  ]
}
```

**Top-level fields:**

| Field         | Type   | Description                                      |
|---------------|--------|--------------------------------------------------|
| `version`     | string | Proof format version; always `"1"`.              |
| `exported_at` | string | ISO-8601 UTC timestamp of export.                |
| `checkpoint`  | object | Checkpoint metadata (see below).                 |
| `entries`     | array  | Ordered by `chain_seq` ascending.                |

**`checkpoint` object:**

| Field         | Type   | Description                                              |
|---------------|--------|----------------------------------------------------------|
| `id`          | string | UUID of the checkpoint record.                           |
| `lineage_id`  | string | UUID of the specimen lineage sealed by this checkpoint.  |
| `start_seq`   | int    | First `chain_seq` in the sealed range.                   |
| `end_seq`     | int    | Last `chain_seq` in the sealed range.                    |
| `entry_count` | int    | Number of entries; must equal `len(entries)`.            |
| `merkle_root` | string | Lowercase hex SHA-256 Merkle root of all entry hashes.   |
| `created_at`  | string | ISO-8601 UTC timestamp when the checkpoint was created.  |

**Each entry object:**

| Field          | Type   | Description                                                     |
|----------------|--------|-----------------------------------------------------------------|
| `chain_seq`    | int    | Sequence number within the lineage.                             |
| `canonical`    | string | Pipe-delimited canonical form (see §3).                         |
| `prev_hash`    | string | 64-char lowercase hex SHA-256 of the preceding entry (or zeros).|
| `entry_hash`   | string | 64-char lowercase hex SHA-256 of `canonical + prev_hash`.       |
| `merkle_path`  | array  | Inclusion path from this leaf to the Merkle root (see §4).      |

---

## 3. Canonical entry form

The `canonical` string is the exact bytes used to compute `entry_hash`.

```
lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
```

* Fields are pipe-separated (`|`).
* `NULL` optional fields serialize as empty string (no placeholder).
* No trailing newline.
* Field order is **fixed** — never reorder; append new fields at the end only.

**Example:**
```
3f2a1c7b-1234-...|1|2026-01-01T10:00:00.000Z|user-uuid|specimen|3f2a1c7b-1234-...|create|
```

---

## 4. Entry hash computation

```
entry_hash = SHA-256(UTF-8(canonical) || UTF-8(prev_hash))
```

* `prev_hash` for `chain_seq = 1` is 64 hex zeros (`"000…000"`), unless this lineage
  was created as a child of another (split), in which case it is the parent's last
  `entry_hash`.

---

## 5. Merkle tree construction

The same binary Merkle tree algorithm used by the checkpoint engine (see
`docs/merkle-checkpoints.md`):

1. Leaves are the `entry_hash` values, ordered by `chain_seq` ascending.
2. **Duplicate-last rule**: if a level has an odd number of nodes, the last node is
   duplicated before pairing. This matches Bitcoin's Merkle construction.
3. Each pair of adjacent nodes is hashed: `SHA-256(UTF-8(left) || UTF-8(right))`.
4. Repeat until one node remains — that is the `merkle_root`.

Edge cases:
* Empty leaf list → 64 hex zeros.
* Single leaf → that leaf is the root (no hashing).

---

## 6. Merkle inclusion path

Each entry carries a `merkle_path`: an ordered list of sibling-hash nodes from the leaf
up to the root.

Each node:
```json
{ "sibling_hash": "<64-char hex>", "position": "left" | "right" }
```

To verify that `entry_hash` is included in `merkle_root`:

```
current = entry_hash
for node in merkle_path:
    if node.position == "right":
        current = SHA-256(UTF-8(current) || UTF-8(node.sibling_hash))
    else:
        current = SHA-256(UTF-8(node.sibling_hash) || UTF-8(current))
assert current == merkle_root
```

A single-leaf tree has an empty `merkle_path`; verify by asserting `entry_hash == merkle_root`.

---

## 7. Three-stage verification

SteloPTC's `verify_exported_proof` command (and the Python verifier below) applies three
sequential checks:

**Stage 1 — Content hash integrity**  
For each entry, recompute `SHA-256(canonical || prev_hash)` and assert it equals
`entry_hash`. A mismatch means the canonical content was altered.

**Stage 2 — Hash-chain links**  
For each consecutive pair `(entry[i-1], entry[i])`, assert
`entry[i].prev_hash == entry[i-1].entry_hash`. A mismatch means an entry was inserted,
removed, or reordered.

**Stage 3 — Merkle root**  
Collect all `entry_hash` values in `chain_seq` order, rebuild the Merkle root, and
assert it equals `checkpoint.merkle_root`. A mismatch means entries were altered or
swapped even if individual hashes look correct.

---

## 8. Standalone Python verifier

Save as `verify_merkle_proof.py` and run with Python 3.8+. No third-party dependencies.

```python
#!/usr/bin/env python3
"""Standalone verifier for SteloPTC portable Merkle proofs (format version 1)."""

import hashlib
import json
import sys


def sha256_hex(*parts: str) -> str:
    h = hashlib.sha256()
    for p in parts:
        h.update(p.encode("utf-8"))
    return h.hexdigest()


def build_merkle_root(leaves: list[str]) -> str:
    ZERO_HASH = "0" * 64
    if not leaves:
        return ZERO_HASH
    level = list(leaves)
    while len(level) > 1:
        if len(level) % 2 == 1:
            level.append(level[-1])
        level = [sha256_hex(level[i], level[i + 1]) for i in range(0, len(level), 2)]
    return level[0]


def verify_merkle_path(leaf_hash: str, path: list[dict], expected_root: str) -> bool:
    if not path:
        return leaf_hash == expected_root
    current = leaf_hash
    for node in path:
        if node["position"] == "right":
            current = sha256_hex(current, node["sibling_hash"])
        else:
            current = sha256_hex(node["sibling_hash"], current)
    return current == expected_root


def verify_proof(proof: dict) -> tuple[bool, str]:
    if proof.get("version") != "1":
        return False, f"Unsupported version '{proof.get('version')}'; expected '1'."

    checkpoint = proof["checkpoint"]
    entries = proof["entries"]
    expected_count = checkpoint["entry_count"]

    if len(entries) != expected_count:
        return False, (
            f"Entry count mismatch: proof has {len(entries)} entries "
            f"but checkpoint expected {expected_count}."
        )

    # Stage 1: content hash integrity
    for entry in entries:
        computed = sha256_hex(entry["canonical"], entry["prev_hash"])
        if computed != entry["entry_hash"]:
            return False, (
                f"Hash mismatch at seq {entry['chain_seq']}: "
                "canonical form does not match the stored entry_hash."
            )

    # Stage 2: hash-chain links
    for i in range(1, len(entries)):
        prev, curr = entries[i - 1], entries[i]
        if curr["prev_hash"] != prev["entry_hash"]:
            return False, (
                f"Chain break at seq {curr['chain_seq']}: "
                "prev_hash does not match the preceding entry_hash."
            )

    # Stage 3: Merkle root
    leaf_hashes = [e["entry_hash"] for e in entries]
    computed_root = build_merkle_root(leaf_hashes)
    if computed_root != checkpoint["merkle_root"]:
        return False, (
            "Merkle root mismatch: the recomputed root does not match "
            "the checkpoint's stored root."
        )

    n = len(entries)
    return True, (
        f"Proof verified — all {n} {'entry' if n == 1 else 'entries'} are intact "
        f"and the Merkle root matches the checkpoint."
    )


def main() -> None:
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <proof.json>", file=sys.stderr)
        sys.exit(1)

    with open(sys.argv[1], encoding="utf-8") as fh:
        proof = json.load(fh)

    ok, message = verify_proof(proof)
    status = "PASS" if ok else "FAIL"
    print(f"[{status}] {message}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
```

Usage:

```sh
python3 verify_merkle_proof.py merkle-proof-550e8400.json
```

Expected output on success:
```
[PASS] Proof verified — all 42 entries are intact and the Merkle root matches the checkpoint.
```

Expected output on failure:
```
[FAIL] Hash mismatch at seq 7: canonical form does not match the stored entry_hash.
```

Exit code is `0` on success, `1` on any failure.
