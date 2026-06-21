# SteloPTC → Stelo Lab Suite — Engineering Roadmap

**Status as of late June 2026:** **v1.11.0** · Trust Layer (WP-18–21) complete · Dead Specimen workflow + WP-22 (lab_profile) shipped · Phase C + Phase TX-1 in active development

**Schema:** 15 migrations (latest: migration 015 — `event_type` on subcultures + `app_config` table for lab_profile).

**Recent major deliveries:**
- Full cryptographic Trust Layer (WP-18–21): per-lineage hash chain, Merkle checkpoints, portable proof export, auto-checkpointing, offline verification.
- Dead Specimen workflow: "Record Death & Archive" terminal event instead of normal passage.
- WP-22: `lab_profile` setting (`plant_tissue_culture` | `cell_culture` | `mycology`).

**Current focus:** Phase C de-hardening + Phase TX-1 (Strain/Cultivar foundation).

**Assets to preserve:** error-logging system, immutable audit trail + cryptographic layer, contamination tracking.

**Goal:** Harden PTC, then expand to Cell Culture and Mycology verticals from one shared engine.

---

## 0. How to use this document with Claude Code

(This section remains unchanged from previous version.)

---

## 1. Recommended sequence (the strategic call)

1. **Phase A — Ship PTC v1.0** ✅ DONE (v1.1.0)
2. **Phase B — Polish, stability & Trust Layer** ✅ DONE (v1.11.0)
3. **Phase C — De-harden the domain** (in progress)
4. **Phase TX — Taxonomic & Provenance Module** (in progress, equal priority)
5. **Phase D & E — Cell Culture & Mycology verticals**

---

## 2. PHASE A — Critical path to PTC v1.0

**Phase A is complete (shipped as v1.1.0).**

### WP-01 to WP-05 ✅ All delivered
(Details unchanged from previous version.)

---

## 3. PHASE B — Polish & Trust Layer

**Phase B + Trust Layer (WP-06 through WP-21) is now complete as of v1.11.0.**

### WP-06 to WP-17 ✅ All delivered
(Details unchanged.)

### Trust(less) and Audit Layer (WP-18–21) ✅ Complete

- **WP-18**: Per-lineage SHA-256 hash chain on `audit_log`.
- **WP-19**: Contamination inheritance on split + verification UX improvements.
- **WP-20**: Merkle checkpoints + three-stage verification.
- **WP-21**: Portable Merkle proof export + offline verification + basic auto-checkpointing.

All cryptographic invariants are encoded in tests (59 tests total). The Trust Layer is production-ready for single-user desktop use.

---

## 4. PHASE C — De-harden the domain (in progress)

### WP-22 ✅ Delivered (v1.11.0)
- Introduced `lab_profile` (`plant_tissue_culture` | `cell_culture` | `mycology`).
- Single-row `app_config` table.
- Admin-only read/write commands with safeguard once specimens exist.
- `src/lib/profile.ts` store.

### WP-23–27 (remaining Phase C work)
Still pending. Focus is on converting CHECK constraints and hardcoded vocabularies into profile-scoped lookup tables.

---

## 5. PHASE TX — Taxonomic & Provenance Module (in progress)

Phase TX-1 (WP-28–29) is the current active workstream alongside remaining Phase C items.

---

## 9. Risk register & guardrails

(Existing risk register remains relevant. New risks around death-event modeling and dashboard interactivity will be added as those packets are scoped.)

---

## 10. Versioning plan (updated)

| Version | Contains | Status |
|---------|----------|--------|
| v1.8.0  | Split workflow overhaul | ✅ shipped |
| v1.9.0  | WP-20 Merkle checkpoints | ✅ shipped |
| v1.10.0 | WP-21 Portable proofs + auto-checkpointing | ✅ shipped |
| **v1.11.0** | Dead Specimen workflow + WP-22 `lab_profile` | ✅ shipped |
| v1.12.x | Remaining Phase C de-hardening (WP-23+) | in progress |
| v2.x    | Phase TX-1 (Strain/Cultivar foundation) | planned |

> **Note:** The Trust Layer (WP-18–21) is now complete. Future work focuses on Phase C de-hardening and Phase TX-1.

---

*This roadmap reflects the state after the WP-21 + Dead Specimen + WP-22 delivery (v1.11.0). The core cryptographic foundation is solid. Next major efforts are usability improvements (Dead Specimen handling, Dashboard interactivity) and domain de-hardening.*