# SteloPTC On-Chain Anchoring (Dogecoin `OP_RETURN`)

_Specification for the anchoring feature introduced in WP-66 (Trust Layer Phase 2, v1.42.0)._

This document describes how SteloPTC publishes an audit **Merkle checkpoint root**
(see `docs/merkle-checkpoints.md`) to a public blockchain so that a checkpoint's
existence at a point in time can be verified by anyone — including a party who does
**not** trust, and has no access to, the lab's database.

---

## 1. Why anchor?

The hash-chained audit log (WP-18) and Merkle checkpoints (WP-20/21) make the lab's
history *internally* tamper-evident: any edit to a sealed entry breaks the chain. But
those guarantees are all verifiable only against the lab's own data. They cannot, on
their own, prove *when* a given state existed, or stop a lab from rewriting its entire
history including the checkpoints.

Publishing a checkpoint's Merkle root into a public chain closes that gap. Once the root
sits in a confirmed transaction, it is timestamped and immutable for everyone: the lab
can no longer claim a different history existed at that root without contradicting a
public record it does not control. This is the standard use of an `OP_RETURN` commitment.

---

## 2. Honest scope — what ships, and what doesn't

SteloPTC **prepares** the exact bytes to publish and **verifies** what comes back. It does
**not** broadcast the transaction itself.

Broadcasting an `OP_RETURN` requires a funded wallet and either a full node or a
third-party broadcast API — i.e. private keys and money. Putting that inside a
specimen-tracking desktop app would add a large, security-sensitive surface for a step the
operator can already perform with any Dogecoin wallet. The **trust guarantee does not
depend on who broadcasts**: the payload is a public commitment that anyone can verify, so
whether the lab, an auditor, or a notary service sends the transaction is irrelevant to
its meaning.

So the split is:

| Step | Who | Where |
|---|---|---|
| Build the `OP_RETURN` payload for a checkpoint root | SteloPTC | `prepare_checkpoint_anchor` |
| Broadcast a transaction carrying that payload | The operator's external wallet | Outside SteloPTC |
| Record the resulting `txid` | SteloPTC | `record_checkpoint_anchor` |
| Verify the on-chain data commits to the root | SteloPTC (trustless) | `verify_checkpoint_anchor` |

This mirrors the "foundation now, credential-bearing transport later" boundary already
disclosed for WP-50 (PostgreSQL), WP-59 (S3/SFTP) and WP-61 (WASM).

---

## 3. Wire format

The anchor payload is a standard Bitcoin/Dogecoin `OP_RETURN` scriptPubKey:

```
0x6a                       OP_RETURN
0x25                       pushdata length = 37 bytes
53 54 45 4c                "STEL"  — 4-byte protocol marker
01                         1-byte format version
<32 bytes>                 the checkpoint Merkle root (raw bytes of its 64 hex chars)
```

* Total script: **39 bytes**. The 37-byte data payload is well under Dogecoin's 80-byte
  standard-relay `OP_RETURN` limit.
* The `STEL` marker lets a verifier distinguish a SteloPTC anchor from any other
  `OP_RETURN` data on-chain. The version byte lets a future payload format stay
  distinguishable from v1.
* An all-zero Merkle root (an empty checkpoint) is refused — there is nothing to anchor.

The canonical construction and parsing live in `src-tauri/src/anchoring/mod.rs`
(`build_op_return_script_hex`, `parse_op_return_script`), which is dependency-free and
fully unit-tested.

---

## 4. Lifecycle

Each anchor is one row in `checkpoint_anchors` (migration 046) and moves through three
states:

1. **`prepared`** — `prepare_checkpoint_anchor(checkpoint_id)` snapshots the checkpoint's
   Merkle root, builds the `OP_RETURN` script hex, and stores the row. The UI shows the
   copyable hex.
2. **`submitted`** — the operator broadcasts the payload with their own wallet and calls
   `record_checkpoint_anchor(anchor_id, txid)`. The 64-hex `txid` is validated and also
   written back to `audit_checkpoints.anchored_txid` (the Phase-2 hook reserved since
   migration 013), so the checkpoint itself surfaces its anchor.
3. **`confirmed`** — `verify_checkpoint_anchor(anchor_id, op_return_hex)` is given the raw
   `OP_RETURN` data an operator copied from a public block explorer for that `txid`. It
   independently extracts the committed root and compares it to the anchor's stored root.
   On a match, the anchor is stamped `verified_at` and marked confirmed.

Preparing, recording and verifying all require the **supervisor or admin** role; listing
anchors is available to any authenticated user. Every lifecycle action is written to the
audit log.

---

## 5. Independent verification (trustless)

The confirmation check trusts **only two inputs**: the `OP_RETURN` hex and the expected
Merkle root. It never consults the rest of the database. That means an external auditor
can reproduce it with nothing but a block explorer:

1. Look up the recorded `txid` on any Dogecoin block explorer.
2. Copy the `OP_RETURN` output's script hex (it starts `6a25...`).
3. Confirm the bytes after `6a 25` are `53 54 45 4c 01` followed by 32 bytes.
4. Those 32 bytes, in hex, are the checkpoint Merkle root — compare them to the root the
   lab published (e.g. in a Merkle proof export from WP-21).

Equivalently, in Python:

```python
def extract_root(op_return_hex: str) -> str:
    b = bytes.fromhex(op_return_hex.strip())
    if b[0] != 0x6a:
        raise ValueError("not an OP_RETURN output")
    data = b[2:]                       # skip opcode + pushdata length
    assert data[:4] == b"STEL", "not a SteloPTC anchor"
    assert data[4] == 0x01, "unsupported version"
    return data[5:5 + 32].hex()

# extract_root("6a2553544...") == "<checkpoint merkle root>"
```

Because the root is itself the top of a Merkle tree over the sealed audit entries
(`docs/merkle-checkpoints.md`), matching it on-chain proves the *entire* sealed range
existed at the time of the transaction.

---

## 6. Tauri commands

| Command | Role | Purpose |
|---|---|---|
| `preview_checkpoint_anchor_payload` | manage | Show the `OP_RETURN` bytes for a checkpoint without writing a row |
| `prepare_checkpoint_anchor` | manage | Create a `prepared` anchor row |
| `record_checkpoint_anchor` | manage | Attach the broadcast `txid`; sets `audit_checkpoints.anchored_txid` |
| `verify_checkpoint_anchor` | manage | Trustless check against on-chain data; confirms the anchor |
| `list_checkpoint_anchors` | any authenticated | List anchors, optionally scoped to one checkpoint |

The UI lives in the **Audit Log → Checkpoints → On-Chain Anchoring** panel
(`OnChainAnchorPanel.svelte`).
