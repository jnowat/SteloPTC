# SteloPTC — Specification Index

Technical specifications for the parts of SteloPTC that need to be **verifiable by someone who
doesn't trust us**: the hash chain, the proofs, the signed inter-lab documents, and the extension
points.

If you're looking for something else:

| You want… | Read |
|---|---|
| A product overview | [`../README.md`](../README.md) |
| How to *use* the app | [`../UserManual.md`](../UserManual.md) |
| Per-work-packet engineering status | [`../ROADMAP.md`](../ROADMAP.md) |
| Release-by-release history | [`../CHANGELOG.md`](../CHANGELOG.md) |
| How to contribute code | [`../SKILLS.md`](../SKILLS.md) |

---

## Trust Layer — tamper-evidence & proof

| Spec | Work packet | What it covers |
|---|---|---|
| [Merkle checkpoints](merkle-checkpoints.md) | WP-20 · v1.9.0 | Sealing a range of audit history to a single Merkle root; three-stage verification (count → root → per-entry content) |
| [Portable Merkle proofs](merkle-proofs.md) | WP-21 · v1.10.0 | The exported proof JSON format and the standalone Python verifier that checks it offline |
| [On-chain anchoring](on-chain-anchoring.md) | WP-66 · v1.42.0 | Committing a checkpoint root to Dogecoin in a 39-byte `OP_RETURN`, and verifying it back independently |
| [Signed event ledger](signed-event-ledger.md) | WP-67 · v1.43.0 | Per-user Ed25519-signed, hash-chained lifecycle events — non-repudiation on top of tamper-evidence |

## Federated inter-lab exchange (Phase G)

All three are signed, self-contained JSON documents a partner lab verifies with nothing but the
issuer's public key — no database access, no network. Each ships a standalone ~40-line verifier.

| Spec | Work packet | What travels | Receiver's choice |
|---|---|---|---|
| [Specimen passport](specimen-passport.md) | WP-70 · v1.45.0 | One specimen's identity + full provenance | Verify, then Verify & Import |
| [Shared taxonomy registry](taxonomy-registry.md) | WP-71 · v1.46.0 | Taxa, species and strains | Accept / Override / Fork, per record |
| [Breeding coordination](breeding-coordination.md) | WP-72 · v1.47.0 | One breeding program's selection records | Accept / Skip, per record (set-union merge) |

## Compliance & reporting

| Spec | Work packet | What it covers |
|---|---|---|
| [Regulatory exports](regulatory-exports.md) | WP-60 · v1.40.0 | FDA 21 CFR Part 11 attestation bundles, USDA APHIS PPQ 526 pre-fill, CITES provenance dossiers |

## Extensibility & lab profiles

| Spec | Work packet | What it covers |
|---|---|---|
| [Vocabulary system](vocabulary-system.md) | WP-23 / WP-24 · v1.12.0 | Profile-scoped lookup tables — why vocabulary is data, not schema |
| [Plugin authoring](plugin-authoring.md) | WP-61 · v1.40.0 | The `.steloplugin` manifest format for vocabulary packs |

## Optional integrations

| Spec | Work packet | What it covers |
|---|---|---|
| [Local AI assistant](local-ai.md) | WP-56 / WP-56b · v1.40.0–v1.41.0 | Ollama and LocalAI setup, model recommendations, troubleshooting — entirely on-device |
