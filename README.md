<div align="center">

# 🌱 SteloPTC

### Plant Tissue Culture Tracking — with provenance you can prove

**A desktop & mobile lab platform for tracking tissue-culture specimens through their entire lifecycle — initiation, subculture, splitting, cryopreservation, and compliance — on a tamper-evident, cryptographically verifiable record.**

[![Version](https://img.shields.io/badge/version-1.47.0-2e7d32.svg)](CHANGELOG.md)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20·%20Linux%20·%20macOS%20·%20Android-1565c0.svg)](#platform-support--maturity)
[![Built with](https://img.shields.io/badge/Rust%20·%20Tauri%202%20·%20Svelte%205-informational.svg)](#tech-stack)
[![Tests](https://img.shields.io/badge/tests-602%20Rust%20·%20106%20TS-4caf50.svg)](#testing--quality)
[![License](https://img.shields.io/badge/license-proprietary-lightgrey.svg)](LICENSE)

**Start here:** [User Manual](UserManual.md) · [Roadmap](ROADMAP.md) · [Changelog](CHANGELOG.md) · [Local AI setup](docs/local-ai.md)

</div>

---

## What is SteloPTC?

SteloPTC manages the full lifecycle of plant tissue-culture specimens for commercial and
research laboratories — and it does one thing no ordinary lab notebook does: **it makes the
history of every culture tamper-evident.**

Every meaningful action (create a specimen, record a passage, split a culture, register a
strain, edit a species) is written into an append-only, SHA-256 **hash chain**. Change any
past entry and every downstream hash breaks — so you can trace, and *cryptographically
verify*, the complete provenance of any culture, even dozens of generations and splits later.

Built with **Rust + Tauri 2 + Svelte 5** for native performance on the desktop and Android,
it runs **local-first and fully offline** — your lab data lives on your machine, not in
someone else's cloud.

Beyond plant tissue culture, the same engine ships three lab profiles out of the box —
**Plant Tissue Culture**, **Cell Culture**, and **Mycology** — and can be extended with
plugin vocabulary packs.

---

## Highlights

**🔬 Specimen tracking**
Accession numbers, provenance and lineage trees, health/disease status, stages, quarantine
and IP flags, generation depth, cumulative passage count, and Population Doubling Level (PDL).

**🌿 Splitting & passages**
Atomic split into letter-suffixed child accessions (`001` → `001A`/`001B`), per-child
configuration, draft media batches, a safety confirmation dialog, and a vertical passage
timeline — all recorded on each specimen's own cryptographic chain.

**🔐 Cryptographic audit chain**
Per-lineage SHA-256 hash chain from species → strain → specimen, with in-app Row/Chain
verification, **Merkle checkpoints**, portable offline-verifiable proofs, on-chain (Dogecoin)
anchoring, and a per-user signed-event ledger. See
[`docs/merkle-checkpoints.md`](docs/merkle-checkpoints.md).

**🛂 Federated passports, taxonomy registry & breeding coordination** *(inter-lab exchange)*
Issue a **signed, self-contained specimen passport** — the specimen's identity and full
provenance — that a partner lab verifies **independently** (with only your public key and the
embedded, recomputable audit chain) and **imports into its own audit chain**. Export a
**signed taxonomy registry** of your taxa, species, and strains that another lab verifies the same
way and reconciles record-by-record (**accept / override / fork**). And merge two labs' **breeding
program selection records** with a **signed coordination bundle** the partner verifies and folds into
its own copy of the program as a set union (**accept / skip**). No central authority; a ~40-line
standalone verifier ships for each. See
[`docs/specimen-passport.md`](docs/specimen-passport.md),
[`docs/taxonomy-registry.md`](docs/taxonomy-registry.md), and
[`docs/breeding-coordination.md`](docs/breeding-coordination.md).

**🧬 Strains, taxonomy & pedigree**
First-class strain/cultivar registry with a four-value verification model, a hybridization
wizard (F1–F4 / backcross labeling), a full Kingdom→Species→Strain taxonomy navigator,
multi-generational pedigree tools, breeding programs, and Darwin Core export.

**🧪 Media, inventory & cryostorage**
MS/WPM/B5/etc. media batches with auto-calculated salts and hormone tracking, full supply
inventory with reorder alerts, prepared stock solutions, and an LN₂ cryostorage inventory
with atomic freeze/thaw.

**📋 Compliance & reporting**
Auto-flagging rules (expired permits, HLB testing, quarantine, mycoplasma), agency tracking
(USDA APHIS, TX Ag, FL FDACS), professional print/PDF reports, an analytics dashboard,
regulatory export bundles (FDA 21 CFR Part 11, USDA PPQ 526, CITES), and a **submission
pipeline** that monitors compliance state and auto-generates signed, ready-to-submit
packages when preconditions are met. See
[`docs/regulatory-exports.md`](docs/regulatory-exports.md).

**🤖 Local AI assistant** *(optional, on-device)*
Note summaries, passage-comment drafts, and photo contamination checks via **Ollama or
LocalAI** — running entirely on your own hardware. Every suggestion is a draft a human must
approve before it touches a record. **Nothing is ever sent to a cloud AI service.**
See [`docs/local-ai.md`](docs/local-ai.md).

**📷 QR codes, photos & data portability**
Per-specimen QR generation, print labels and camera scanning; photo attachments with a
lightbox gallery; multi-sheet Excel/CSV/JSON export and round-trip Excel import; on-demand
and encrypted cloud backup.

**♿ Built for real labs**
Mobile-first responsive UI, dark mode, WCAG 2.1 AA accessibility pass, keyboard shortcuts,
contextual help tooltips, role-based access, and an interactive lab-map floor plan.

> The full, exhaustive feature list — with the release each feature shipped in — lives in
> [`ROADMAP.md`](ROADMAP.md) and [`CHANGELOG.md`](CHANGELOG.md). This README is the overview.

---

## Platform support & maturity

SteloPTC is honest about what is production-ready and what is still maturing.

| Platform | Status | Notes |
|---|---|---|
| **Windows** (desktop) | ✅ **Primary target — stable** | Signed `.msi` attached to every [Release](../../releases); built by CI |
| **Linux** (desktop) | ✅ Supported | `.deb` / `.AppImage` from source or CI |
| **macOS** (desktop) | ✅ Buildable | Builds from source (Xcode CLT); not yet distributed via CI |
| **Android** 7.0+ (API 24–35) | ✅ **Stable** | Release-signed `.apk` on every [Release](../../releases); debug APK on every push |
| **iOS** 13+ | 🧪 Experimental | CI scaffold only, **never verified end-to-end**, not distributed — needs a maintainer with a Mac + Apple Developer account |
| **PWA** (browser) | 🧩 Read-only shell | Installable; all read views work offline, but **data mutations still require the desktop app** (no remote API yet) |

A handful of features are intentionally shipped **foundation-only** and clearly marked as
such in-app and in the docs: PostgreSQL backend (connector only — SQLite serves all
reads/writes), LAN sync (change-detection without a network transport), S3/SFTP cloud backup
(config-only; `local_nas`/`smb` are live), and plugin WASM rule execution (validated but not
yet executed). Full detail in [`ROADMAP.md`](ROADMAP.md).

---

## Download & install

### Windows

Download the latest signed `.msi` from **[Releases](../../releases/latest)** and run the
installer.

### Android

- **Stable:** download the release-signed `.apk` attached to the latest
  **[Release](../../releases/latest)** (supports in-place upgrades).
- **Latest build:** grab the `SteloPTC-Android-Debug` artifact from the most recent green
  run of **[Build Android APK](../../actions/workflows/build-android.yml)** (every push,
  30-day retention).

To sideload: transfer the `.apk` to your device, open it, allow *Install unknown apps* for
your file manager, and install. Requires **Android 7.0 (API 24)+**.

### First launch

On a fresh database the only account is **`admin` / `admin`** — the app immediately forces a
password change before anything else can be done.

---

## Building from source

### Prerequisites

| Tool | Version | Install |
|---|---|---|
| Rust | 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Node.js | 18+ | [nodejs.org](https://nodejs.org) |
| Tauri CLI | latest | `cargo install tauri-cli` |

**Linux** also needs:

```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev librsvg2-dev libssl-dev
```

### Run & build

```bash
git clone https://github.com/jnowat/steloptc.git
cd steloptc
npm install --legacy-peer-deps

cargo tauri dev                 # run with hot-reload

cargo tauri build --bundles msi # Windows MSI + exe
cargo tauri build               # Linux .deb / .AppImage
```

Output lands in `src-tauri/target/release/bundle/`.

### Android

```bash
./scripts/setup-android.sh      # one-time prerequisite install
npm run android:build-debug     # unsigned, sideloadable APK
npm run android:build           # release APK (needs signing env vars)
```

Release signing uses the `ANDROID_KEY_STORE_PATH`, `ANDROID_KEY_STORE_PASSWORD`,
`ANDROID_KEY_ALIAS`, and `ANDROID_KEY_PASSWORD` environment variables. Target SDK is API 35,
minimum API 24, NDK r27. See [`ROADMAP.md`](ROADMAP.md) for iOS status and the PWA build.

---

## Local AI assistant

SteloPTC's AI is **optional and entirely on-device**. Configure it in
**Settings → AI Assistant**:

1. Install [Ollama](https://ollama.com) and pull the models:
   ```bash
   ollama pull llama3.1   # text
   ollama pull llava      # vision
   ```
2. In **Settings → AI Assistant**, keep the runtime on `Ollama` and click **Test Connection**
   — it confirms reachability and lists your installed models.

Prefer an existing OpenAI-compatible server? Switch the runtime to **LocalAI** and point it
at your endpoint. Every AI result is a **draft that a human must approve** before it is
appended to any record, and the approval is audit-logged to the approving user.

👉 Full setup, model recommendations, and troubleshooting: **[`docs/local-ai.md`](docs/local-ai.md)**

---

## Data integrity & security

Provenance and integrity are the point of SteloPTC, not an afterthought:

- **Per-lineage SHA-256 hash chain** — species anchor a chain; strains and specimens seed from
  their parent's hash; splits inherit the parent's last hash, making fork points
  cryptographically unambiguous. Any out-of-band edit is detectable.
- **Merkle checkpoints & portable proofs** — seal a range of history to a single Merkle root;
  three-stage verification (count → root → per-entry content) catches deletions, hash
  tampering, and content edits. Proofs verify offline with a standalone script.
- **On-chain anchoring** — publish a checkpoint's Merkle root to the Dogecoin chain in an
  `OP_RETURN` output for third-party-verifiable timestamping. SteloPTC prepares the exact
  bytes and independently verifies the on-chain data (trusting only the block explorer, not
  the lab); broadcasting uses your own external wallet.
- **Signed event ledger** — a hash-chained ledger of lifecycle events, each additionally
  signed with the acting user's own Ed25519 key, adding non-repudiation on top of
  tamper-evidence: an entry's authorship can't be forged by someone who can write to the
  database but doesn't hold the signer's key.
- **Authentication & roles** — bcrypt password hashing, session tokens, forced first-login
  password change, and four roles (Admin / Supervisor / Tech / Guest).
- **Locked-down CSP** — `script-src 'self'`; no remote scripts.
- **Encrypted cloud backup** — Argon2id + AES-256-GCM, passphrase never persisted.

See [`docs/merkle-checkpoints.md`](docs/merkle-checkpoints.md),
[`docs/merkle-proofs.md`](docs/merkle-proofs.md),
[`docs/on-chain-anchoring.md`](docs/on-chain-anchoring.md), and
[`docs/signed-event-ledger.md`](docs/signed-event-ledger.md) for the specifications.

---

## Tech stack

| Layer | Technology |
|---|---|
| Backend | Rust 1.75+ |
| Framework | Tauri v2 (native desktop + Android) |
| Frontend | Svelte 5, TypeScript, Vite 6 |
| Database | SQLite (bundled, WAL mode); optional PostgreSQL connector (foundation-only) |
| Auth | bcrypt, session tokens, forced first-login password change |
| Crypto | SHA-256 audit chain · Ed25519 signed exports · Argon2id + AES-256-GCM backup |
| Local AI | Ollama · LocalAI / OpenAI-compatible (on-device only) |
| QR / Excel | qrcode + html5-qrcode · SheetJS (xlsx) |

---

## Testing & quality

SteloPTC ships with backend (Rust) and frontend (TypeScript) test suites, both run in CI on
every push and required to pass before merge.

```bash
npm test                                                 # frontend (Vitest) — 106 assertions
cd src-tauri && cargo test --lib --no-default-features   # backend — 549 tests
npm run check                                            # svelte-check + TypeScript
```

> `cargo test --lib --no-default-features` runs the 549 pure-logic tests without GTK/WebKit.
> The full `tauri-commands` feature build (used in CI) adds the command-layer tests on top.

CI: `test.yml` (Vitest + cargo), `build-windows.yml` (signed MSI), `build-android.yml`
(release APK), plus non-blocking Criterion benchmarks.

---

## More documentation

| Document | What's in it |
|---|---|
| [User Manual](UserManual.md) | End-to-end guide for lab staff — every workflow, step by step |
| [Roadmap](ROADMAP.md) | The plan, the current state, and per-work-packet status |
| [Changelog](CHANGELOG.md) | Release-by-release history |
| [Local AI setup](docs/local-ai.md) | Ollama / LocalAI configuration & troubleshooting |
| [Merkle checkpoints](docs/merkle-checkpoints.md) · [proofs](docs/merkle-proofs.md) · [on-chain anchoring](docs/on-chain-anchoring.md) · [signed event ledger](docs/signed-event-ledger.md) | Hash-chain, tamper-evidence, anchoring & signed-ledger specifications |
| [Regulatory exports](docs/regulatory-exports.md) | FDA / USDA / CITES export bundles |
| [Plugin authoring](docs/plugin-authoring.md) | `.steloplugin` vocabulary-pack format |
| [Vocabulary system](docs/vocabulary-system.md) | How lab-profile vocabularies work |

---

## License

This software is proprietary. See [LICENSE](./LICENSE) for the full commercial license
agreement. For purchasing information, contact **licensing@stelolab.local**.
