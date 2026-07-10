# SteloPTC Signed Event Ledger

_Specification for the signed-transaction ledger introduced in WP-67 (Trust Layer Phase 3, v1.43.0)._

This document describes the ledger of **specimen lifecycle events as signed transactions** —
a stronger, opt-in trust layer built on top of the WP-18 hash-chained audit log.

---

## 1. Why a second ledger?

The audit log (WP-18) records *what changed* and is hash-chained, so it is **tamper-evident**:
edit any past entry and every downstream hash breaks. But the audit log is **unsigned** — it
attributes each action to a user id that the database itself writes. Anyone who can write to
the database could, in principle, forge that attribution.

The signed event ledger adds **non-repudiation** on top of tamper-evidence. Each entry is
signed with the **acting user's own Ed25519 private key**. A confirmed entry therefore proves
not just that the history is intact, but *which user's key authorized that specific event* —
attribution that cannot be forged by someone who can write to the database but does not hold
the signer's private key.

This is the layer the roadmap reserved for Trust Layer Phase 3.

---

## 2. Data model (migration 047)

**`user_signing_keys`** — one Ed25519 keypair per user, generated lazily on first use. Distinct
from the single lab-wide export key (`signing_keys`, WP-60): signed ledger entries are
attributed to *individuals*, not the lab.

| Column | Notes |
|---|---|
| `user_id` (PK) | references `users(id)` |
| `public_key_b64`, `private_key_b64` | base64 Ed25519 keypair |
| `created_at` | |

**`signed_events`** — a monotonic, hash-chained, signed ledger:

| Column | Notes |
|---|---|
| `id` (PK) | uuid |
| `seq` (UNIQUE) | global, gapless, starts at 0 — a gap means a deletion |
| `event_type` | e.g. `specimen_created`, `passage`, `split` |
| `entity_type`, `entity_id` | what the event is about |
| `user_id` | the signer |
| `payload` | free-form JSON describing the event |
| `prev_hash` | `event_hash` of `seq-1` (or `ZERO_HASH` for `seq 0`) |
| `event_hash` | `SHA-256(canonical ‖ prev_hash)` — the shared `compute_entry_hash` primitive |
| `signature` | base64 Ed25519 signature over `event_hash` |
| `public_key` | the key that signed this entry (snapshot) |
| `created_at` | |

### Canonical form

The hash is computed over a fixed, pipe-delimited canonical string (never reorder; append
new fields only):

```
seq|timestamp|user_id|event_type|entity_type|entity_id|payload
```

`NULL` optional fields serialize as empty string.

---

## 3. Appending an event

`signed_ledger::append_signed_event(conn, user_id, event_type, entity_type, entity_id, payload)`:

1. Load or create the user's signing keypair.
2. `seq = MAX(seq) + 1` (0 if empty).
3. `prev_hash = ` last entry's `event_hash` (or `ZERO_HASH`).
4. `event_hash = SHA-256(canonical ‖ prev_hash)`.
5. `signature = Ed25519_sign(user_private_key, event_hash)`.
6. Insert the row.

Because each command holds the single database mutex, the read-max-then-insert is race-free.

---

## 4. Verification

`verify_signed_event_ledger` walks every entry in `seq` order and checks four invariants,
returning the `seq` of the first break:

1. **Gapless sequence** — `seq` must equal the running counter; a gap means an entry was
   deleted.
2. **Chain linkage** — each `prev_hash` must equal the previous entry's `event_hash` (the
   first entry links to `ZERO_HASH`).
3. **Content hash** — recomputing `event_hash` from the canonical fields must match the stored
   hash; a mismatch means content was edited.
4. **Signature** — the Ed25519 signature must verify against the entry's `public_key`, *and*
   that key must still match the user's registered key in `user_signing_keys` (a swapped-key
   forgery is rejected).

All four are covered by unit tests (content tampering, deletion, forged signature, swapped
key).

---

## 5. Scope — what ships, and what's incremental

Shipped in v1.43.0:

* The full signing/verification engine (`signed_ledger`), fully unit-tested.
* Commands: `record_signed_event` (any write-capable user, signs their own action),
  `list_signed_events`, `verify_signed_event_ledger`, `get_user_signing_public_key`.
* One wired demonstrating integration: **every newly created specimen** automatically gets a
  signed `specimen_created` genesis transaction, attributed to the creating user. The call is
  best-effort — a ledger hiccup can never fail specimen creation (mirroring the
  `log_audit(...).ok()` convention).
* UI: the **Audit Log → Signed Event Ledger** panel (`SignedLedgerPanel.svelte`) — verify the
  ledger, list recent signed events, and show your own public key.

Incremental follow-up (disclosed, not near-term — matching the WP-63 "exhaustive command-layer
sweep is disproportionate" precedent): extending *automatic* signing to every one of the ~30
mutation commands (passage, split, death, archive, …). The foundation forecloses nothing —
each is a one-line `try_append_signed_event(...)` at the relevant call site, and
`record_signed_event` already lets a client sign any event today.

---

## 6. Tauri commands

| Command | Role | Purpose |
|---|---|---|
| `get_user_signing_public_key` | any authenticated | The caller's Ed25519 public key (created on first use) |
| `record_signed_event` | write-capable | Append a signed transaction for a lifecycle event |
| `list_signed_events` | any authenticated | List signed events, optionally scoped to one entity |
| `verify_signed_event_ledger` | any authenticated | Verify the whole ledger (hashes + sequence + signatures) |
