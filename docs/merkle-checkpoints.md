# Merkle Checkpoints — SteloPTC Audit Trust Layer

**Work Packet:** WP-20  
**Shipped:** v1.9.0  
**Depends on:** WP-18 (hash-chain columns), WP-19 (verification commands)

---

## What are Merkle checkpoints?

A Merkle checkpoint is a tamper-evident snapshot of a range of the audit log.
It stores a single **Merkle root** — a SHA-256 hash that summarises every audit
entry in the sealed range. If any entry is later modified, deleted, or inserted,
re-building the root from the current chain will produce a different value and
the checkpoint will fail verification.

Checkpoints complement the per-entry hash chain (WP-18):

| Mechanism | Detects |
|---|---|
| Hash chain (WP-18) | Content edits to a single entry; broken prev_hash links |
| Merkle checkpoint | Modifications to `entry_hash` values; entry deletions or insertions; any combination of the above within the sealed range |

---

## Canonical serialisation (WP-18, unchanged)

Each audit entry's leaf hash is its `entry_hash` column — a SHA-256 hex string
computed at write time as:

```
entry_hash = SHA-256( canonical_bytes || prev_hash_utf8 )
```

where `canonical_bytes` is the pipe-delimited UTF-8 string:

```
lineage_id|chain_seq|timestamp|user_id|entity_type|entity_id|action|details
```

NULL optional fields serialise as empty string.  Field order is fixed; new
fields are appended only.

---

## Merkle tree construction

**This construction rule is permanently locked.** Changing it invalidates all
existing stored roots.

```
Algorithm:

Given a list of leaf hashes L = [h0, h1, ..., hn]:

1. If L is empty → return ZERO_HASH
   (64 zeroes: "0000...0000")

2. If len(L) == 1 → return L[0]
   (no extra hash round for a single leaf)

3. If len(L) is odd → append a duplicate of the last element:
   L = L + [L[last]]

4. Pair adjacent elements and hash each pair:
   parent_i = SHA-256( L[2i].as_bytes() || L[2i+1].as_bytes() )
   (inputs are the hex-encoded hash strings, not raw bytes)

5. Replace L with the parent layer and repeat from step 3
   until only one element remains → that is the root.
```

**Locked rule for odd counts:** Duplicate the last leaf at every level where
the count is odd, then pair normally.  This matches Bitcoin's Merkle tree rule
and must be reproduced identically by any external verifier.

### Example — 3 leaves

```
Leaves:    [h0, h1, h2]
Padded:    [h0, h1, h2, h2]   ← h2 duplicated (odd count)
Level 1:   [SHA256(h0||h1), SHA256(h2||h2)]
Level 2:   [SHA256(level1[0]||level1[1])]   ← root
```

### Example — 4 leaves

```
Leaves:    [h0, h1, h2, h3]
Level 1:   [SHA256(h0||h1), SHA256(h2||h3)]
Level 2:   [SHA256(level1[0]||level1[1])]   ← root
```

---

## Database schema

```sql
CREATE TABLE audit_checkpoints (
    id            TEXT PRIMARY KEY,
    lineage_id    TEXT NOT NULL,
    start_seq     INTEGER NOT NULL,
    end_seq       INTEGER NOT NULL,
    entry_count   INTEGER NOT NULL,
    merkle_root   TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    created_by    TEXT REFERENCES users(id),
    anchored_txid TEXT          -- Phase-2 hook: Dogecoin txid (WP-65+), NULL for now
);
```

- **`lineage_id`** — the audit lineage (entity ID) this checkpoint covers.
- **`start_seq` / `end_seq`** — the inclusive chain_seq range sealed by this checkpoint.
- **`entry_count`** — number of entries in the range; used to detect deletions.
- **`merkle_root`** — 64-char lowercase hex SHA-256 root.
- **`anchored_txid`** — Phase-2 placeholder; stores the on-chain txid once the root is published to Dogecoin (WP-65+). NULL until then.

---

## Commands

### `create_audit_checkpoint`

**Who can call:** admin or supervisor role.

```
Input:  lineage_id   TEXT     -- which lineage to seal
        start_seq    INTEGER  -- optional; defaults to min chain_seq
        end_seq      INTEGER  -- optional; defaults to max chain_seq

Output: CreateCheckpointResult {
    checkpoint_id, lineage_id, start_seq, end_seq, entry_count, merkle_root
}
```

Builds the Merkle tree over the `entry_hash` values in the given range (ordered
by `chain_seq` ascending) and stores the root.

### `verify_against_checkpoint`

**Who can call:** any authenticated user.

```
Input:  checkpoint_id TEXT

Output: VerifyCheckpointResult {
    checkpoint_id, lineage_id, ok,
    expected_count, actual_count,
    tampered_seq (Option<i64>),
    message
}
```

Verification steps (in order):

1. **Count check** — fetches current entries for the sealed range and
   compares the count to `entry_count`. A mismatch means entries were
   deleted or inserted.

2. **Merkle root check** — rebuilds the root from current `entry_hash`
   values and compares to the stored root. A mismatch means at least one
   `entry_hash` value was changed (indicates hash+content co-tampering).
   If the divergent entry can be identified by individual hash recomputation,
   its `chain_seq` is returned in `tampered_seq`.

3. **Content hash check** — recomputes each entry's `entry_hash` from its
   canonical fields. A mismatch here (while the Merkle root still matches)
   means content was edited without updating `entry_hash`.

### `list_audit_checkpoints`

**Who can call:** any authenticated user.

```
Input:  lineage_id TEXT  -- optional filter

Output: Vec<AuditCheckpoint>
```

Returns all stored checkpoints, most recent first. Optionally filtered to a
single lineage.

---

## UI

The Audit Log view (Settings → Audit Log) has a **Checkpoints** button in the
chain integrity banner. Clicking it expands a panel where you can:

1. **Create a checkpoint** — select a lineage from those visible on the current
   page, optionally set a seq range, then click **Create Checkpoint**.
2. **Verify a checkpoint** — click **Verify** on any existing checkpoint to
   re-check the sealed range against current data.

Success and failure are shown inline with a human-readable message.

---

## Limitations and planned follow-up (WP-21)

- **No automatic checkpointing yet.** Checkpoints are created manually. Automatic
  post-event and pre-backup checkpointing is planned for WP-21.
- **No proof export yet.** Individual Merkle proofs (leaf + sibling path to root)
  are not yet exportable. WP-21 will add `export_audit_proof` and a standalone
  verifier script in `docs/`.
- **Per-lineage only.** Global checkpoints spanning all lineages are not yet
  implemented. The `lineage_id` column makes them easy to add later.
- **`anchored_txid` is always NULL.** On-chain Dogecoin anchoring is a Phase-2
  feature (WP-65+).

---

## Verifying without SteloPTC

An external verifier can reproduce the checkpoint check using only:

1. The `audit_log` table rows for the sealed lineage and seq range.
2. The stored `merkle_root`.

Steps:

```python
import hashlib

def sha256hex(a: str, b: str) -> str:
    return hashlib.sha256((a + b).encode()).hexdigest()

def merkle_root(leaves: list[str]) -> str:
    ZERO_HASH = "0" * 64
    if not leaves:
        return ZERO_HASH
    level = list(leaves)
    while len(level) > 1:
        if len(level) % 2 != 0:
            level.append(level[-1])   # duplicate-last rule
        level = [sha256hex(level[i], level[i+1]) for i in range(0, len(level), 2)]
    return level[0]

# Usage:
# 1. Query: SELECT entry_hash FROM audit_log
#            WHERE lineage_id = ? AND chain_seq BETWEEN ? AND ?
#            ORDER BY chain_seq ASC
# 2. leaves = [row[0] for row in cursor]
# 3. assert merkle_root(leaves) == stored_checkpoint.merkle_root
```

A full standalone verifier (including proof paths) will ship with WP-21.
