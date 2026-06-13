# Changelog

All notable changes to SteloPTC will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.2.5] - 2026-06-13

### Fixed — WP-09: Tauri-reliable print invocation

- **Print Summary** (`SpecimenList.svelte`) — the "Print Summary" button now works on the Windows desktop (Tauri/WebView2) without the "Could not open print window" error. The function first attempts `window.open` (preserving the existing behavior for web/browser builds); if the popup is blocked (as it consistently is in WebView2), it falls back to injecting a hidden `<div>` with the report HTML and a `<style>` element with all print CSS scoped to `@media print`, then calls `window.print()` directly. The `afterprint` event cleans up both elements. The WP-13 output quality (professional header, footer, page counters, typography) is identical in both paths.
- **Culture Certificate** (`SpecimenDetail.svelte`) — `printCultureReport` receives the same popup/fallback treatment. Also fixes a pre-WP-06 silent-fail bug: the function previously did `if (!win) return` with no user notification; it now shows a proper error toast and uses the in-page fallback.
- **QR Label** (`QrModal.svelte`) — `printLabel` receives the same in-page fallback. The `@page { size: 2in 3in; margin: 0 }` rule is placed at the top level of the injected `<style>` element to correctly set the label page size.
- **Version alignment** — `tauri.conf.json` was at `1.2.3` while `package.json` and `Cargo.toml` were at `1.2.4`; all three are now aligned at `1.2.5`.

## [1.2.4] - 2026-06-13

### Added — WP-13: Print / PDF Polish

- **Professional print header band**: every print window (Culture Certificate, Specimens Summary) now opens with a consistent three-column header — a reserved logo area (64 px placeholder), the lab brand + report name, and a right-aligned metadata block (accession, generated date, prepared-by user).
- **Professional print footer band**: footer displays `SteloPTC · Tissue Culture Management System · {date}` on the left and a CSS page-counter (`Page N of M` in print, `Page N` in preview) on the right.
- **A4 + US Letter support**: both documents use `@page { size: auto; }` so the browser uses whatever paper size is selected in the print dialog (A4 or US Letter) without clipping content.
- **Typography and spacing polish**: font stack updated to `'Segoe UI', -apple-system, Helvetica, Arial, sans-serif`; header brand is 21–22 px/900 weight; metadata is 9.5 px/1.8 line-height; table cells use 9–10.5 px body type with wider column padding; `thead { display: table-header-group }` ensures column headers repeat across page breaks.
- **Page-break hygiene**: `tr { page-break-inside: avoid }` on all table rows and `page-break-inside: avoid` on the specimen info grid prevent orphaned rows and torn table cells.
- **Specimen info grid**: label column widened to 155 px and label font dropped to 9.5 px for a cleaner two-column layout in the Culture Certificate.
- **Filter highlight bar**: the Specimens Summary filter line is now displayed as a left-bordered highlight bar (`border-left: 3px solid #e2e8f0; background: #f8fafc`) for better visual hierarchy.

### Added — WP-14: First Test Harness

- **`src/lib/utils.ts`**: new shared TypeScript module containing pure, testable utility functions — `escHtml`, `healthLabel`, `stageFmt`, `composeLocation`, `formatAccessionNumber`, `computeStockAdjustment`, `datestamp` — extracted and suitable for use across print functions and form validation.
- **`src/lib/utils.test.ts`**: Vitest test suite with 30 passing assertions covering all utility functions (null/undefined handling, HTML escaping, health level mapping and clamping, stage formatting, location composition, accession formatting, stock adjustment bounds).
- **`vitest.config.ts`**: Vitest configuration with jsdom environment and `@sveltejs/vite-plugin-svelte` integration. Test glob: `src/**/*.test.ts`.
- **`npm test` / `npm run test:watch`**: added `test` and `test:watch` scripts to `package.json`; `vitest`, `@testing-library/svelte`, `@testing-library/jest-dom`, and `jsdom` added to `devDependencies`.
- **Rust unit tests — `db::queries`**: `#[cfg(test)]` module with 8 tests covering `generate_accession_number` (first specimen gets `001`, second gets `002`, different species resets sequence, different date resets sequence, 3-digit zero-padding) and `PaginationParams` (offset on page 1, offset on page 2, no underflow on page 0).
- **Rust unit tests — `commands::inventory`**: extracted `apply_stock_adjustment(current, adjustment) -> Result<f64, String>` and `is_low_stock(current, minimum) -> bool` as `pub` helper functions; 8 `#[cfg(test)]` assertions covering positive/negative adjustment, exact-zero, below-zero error, and low-stock threshold comparisons.
- **Rust unit tests — `commands::compliance`**: `#[cfg(test)]` module with 10 assertions using in-memory SQLite (minimal schema created inline); tests cover all four auto-flag rules — expired permit (detected/not-detected), quarantine-no-release (detected/with-date-excluded), positive-not-quarantined (detected/already-quarantined-excluded), citrus HLB missing/recent test, and the cross-cutting rule that archived specimens are excluded from every flag query.
- **`.github/workflows/test.yml`**: new CI workflow that runs `npm test` (Vitest) and `cargo test --lib` (Rust) on every push and pull request to `master`. Runs on separate jobs (`frontend-tests` and `rust-tests`) so failures are reported independently. Blocks merges on any test failure.

## [1.2.3] - 2026-06-13

### Fixed
- Dark mode table text invisible: `td` elements now have an explicit `color: #e2e8f0` override in `:global(.dark td)` so text is always readable regardless of CSS variable resolution order.
- `SkeletonLoader.svelte`: replaced `@media (prefers-color-scheme: dark)` with `:global([data-theme="dark"])` so the skeleton respects the app's manual dark-mode toggle rather than the system preference.
- `EmptyState.svelte`: same `@media` → `[data-theme="dark"]` fix; title and message colors now use `var(--color-text-muted)` / `var(--color-text-faint)` tokens.

### Added
- `DataState.svelte`: new reusable component that unifies all four data-fetch states — `loading` (animated skeleton), `error` (inline retry panel), `empty` (friendly message + optional CTA), and `ready` (renders children). Replaces ad-hoc `{#if loading}` / `{:else if}` branches throughout list views.

### Changed
- `SpecimenList.svelte`: integrated `DataState` for loading and error states; inline error now shows a retry button; FirstRun and filter-empty states remain within the ready slot.
- `MediaList.svelte`: loading, error, and empty states replaced with `DataState`.
- `InventoryManager.svelte`: loading, error, and empty states (inventory items + prepared solutions) replaced with `DataState`.
- `ReminderList.svelte`: loading, error, and empty states replaced with `DataState`.
- `ComplianceView.svelte`: loading, error, and per-tab empty states replaced with `DataState`; flags tab shows a "✅ All clear" empty state; records tab shows an empty state with a "+ New Record" CTA.
- `AuditLog.svelte`: plain-text loading/empty replaced with `DataState` skeleton + descriptive empty state.
- `ErrorLog.svelte`: fetch errors are now surfaced inline (with retry) instead of silently swallowed; existing custom loading/empty UI preserved.

## [1.2.2] - 2026-06-13

### Added
- `src/lib/styles/tokens.css`: central design-token file defining CSS custom properties for colors (light + dark), spacing scale, typography sizes, border radii, shadows, z-index layers, and transitions. Single source of truth for all visual constants.

### Changed
- `app.ts`: dark mode subscriber now sets a `data-theme="dark"` / `data-theme="light"` attribute on `<html>` in addition to the existing `.dark` class, enabling token-based theming.
- `App.svelte`: imports `tokens.css` once at the top of the style block; `.app` background and color now reference `--color-bg` / `--color-text` tokens.
- `Dashboard.svelte`: all hardcoded color, spacing, radius, and typography values replaced with design tokens; `:global(.dark)` overrides removed in favour of automatic token switching via `data-theme`.
- `Sidebar.svelte`: all hardcoded values replaced with sidebar-scoped tokens (`--color-sidebar-*`) and shared tokens for spacing, radii, shadows, z-index, and transitions.

## [1.2.1] - 2026-06-13

### Added
- `SkeletonLoader.svelte`: reusable animated shimmer skeleton for table list views, supports configurable row/column counts and both light and dark mode.
- `EmptyState.svelte`: reusable friendly empty state with icon, title, message, and optional call-to-action button.

### Changed
- `SpecimenList.svelte`: loading state now shows an animated skeleton; filtered-empty state uses the new `EmptyState` component with a search icon and helpful message.
- `MediaList.svelte`: loading state shows skeleton; empty state shows a friendly prompt with a "New Batch" CTA.
- `InventoryManager.svelte`: loading state shows skeleton; empty inventory shows a "New Item" CTA; filter-empty and no-solutions states use `EmptyState`.
- `ReminderList.svelte`: loading state shows skeleton; empty state shows a "New Reminder" CTA.

## [1.2.0] - 2026-06-13

### Added

- **WP-08 — Specimen Work Queue / Daily Task View**
  - **`get_work_queue` Tauri command** (`work_queue.rs`) — returns a prioritized list of specimens needing attention. Detects five conditions: (1) subculture due or overdue (based on species default interval), (2) media batch expired on most recent subculture, (3) contamination flag on most recent passage, (4) no recorded passages, (5) unresolved quarantine (no release date or release date passed). Results are sorted by urgency (critical → high → normal) then by days overdue descending.
  - **`WorkQueue.svelte`** — new read-only view showing a prioritized table of specimens that need attention. Each row displays urgency badge (Critical/High/Normal), accession number, species, location, and a plain-language description of the issue. Clicking any row navigates to the specimen detail. Shows a summary badge row (count by urgency level) and an "All clear" state when no items are pending.
  - **`getWorkQueue` API function** (`api.ts`) wraps the new command.
  - **`workQueueCount` store** (`app.ts`) — writable integer store, populated on login and refreshed whenever the Work Queue view loads. Persists in memory for the session lifetime.
  - **Work Queue nav entry** (`Sidebar.svelte`) — added between Dashboard and Specimens with a ✅ icon and an amber count badge matching the error-log badge style. Badge animates in when count > 0.
  - **`work-queue` view route** (`App.svelte`, `app.ts`) — added to the View union type and the view-switcher conditional block.
- Version bumped to **1.2.0** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.1.1] - 2026-06-13

### Fixed

- **WP-06 — Print Summary & Print Label error handling**
  - `SpecimenList.svelte` — `printSummaryReport` now wraps the popup open and document write in try/catch blocks. A blocked or failed popup shows a user-facing error notification ("Could not open print window. Please allow popups for this site and try again.") instead of silently doing nothing.
  - `QrModal.svelte` — `printLabel` receives the same treatment: popup-blocked and write failures both surface a clear error notification. Import of `addNotification` added.

### Changed

- **WP-07 — QR Scanner: Reject Non-SteloPTC Codes Gracefully**
  - **`QrScanner.svelte`** — added `looksLikeNonSteloPTC()` helper that flags payloads starting with `http://`, `https://`, `ftp://`, or matching an email pattern. When a non-SteloPTC code is detected, a new `invalidQr` state is set and a yellow warning card ("This QR code is not a SteloPTC specimen label") is shown instead of the green result card. The "Open Specimen" button and `parsedAccession` are suppressed for invalid codes. The scan event is still recorded in the database for audit purposes. Valid SteloPTC JSON payloads and plain-text accession numbers continue to work normally.
- Version bumped to **1.1.1** across `package.json`, `Cargo.toml`, `tauri.conf.json`, and sidebar display.

## [1.1.0] - 2026-06-11

### Added

- **WP-05 — Onboarding empty state + seed-data toggle**
  - **`FirstRun.svelte`** — new component shown whenever the lab has zero specimens. Displays a two-step guide ("Configure your species registry" → "Accession your first specimen"), with direct navigation buttons for each step. Supervisors and admins also see a **Load Sample Data** button (green, clearly labelled).
  - **`Dashboard.svelte`** — shows `FirstRun` instead of the normal stats grid when `total_specimens === 0`. Returns to the full dashboard automatically once specimens exist (or after demo data is loaded).
  - **`SpecimenList.svelte`** — shows `FirstRun` (with an inline "Add First Specimen" shortcut that scrolls to and opens the specimen form) when the list is genuinely empty (no search/filter active and `total === 0`). Filtered-but-empty searches still show the concise "No specimens found" message.
  - **`load_demo_data` Tauri command** (`admin.rs`) — creates 1 demo MS media batch, 3 demo specimens (Asparagus, Nandina, Citrus) using the seeded species registry, each with 3 passages of subculture history, all in a single atomic transaction. Guard: returns an error if any specimens already exist so it can't clobber an active lab. Supervisors and admins only.
  - **`loadDemoData` API function** (`api.ts`) wraps the new command.
  - Removing demo data: use existing **Admin → Dev Tools → Reset Database** (preserves species/users).
- Version bumped to **1.1.0** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 24), `app/build.gradle.kts` (versionCode 24), and sidebar display.

## [1.0.0-2] - 2026-06-11

### Changed

- **WP-04 — Crash-proofing & data-integrity pass**
  - Replaced `.unwrap()` on `path.parent()` in `attachments_dir` with a proper `Result` return; callers propagate the error through the existing error-log + toast system instead of panicking.
  - Wrapped `create_subculture` in a SQLite transaction: the subculture INSERT, specimen `subculture_count` UPDATE, and optional location UPDATE are now atomic — a failure on any step rolls back all changes.
  - Wrapped `create_media_batch` in a SQLite transaction: the media batch INSERT, all hormone/reagent INSERTs, and all inventory stock deduction UPDATEs are now atomic — no partial batch is committed if a hormone or stock update fails.
  - `create_backup` now verifies the WAL checkpoint result; if active readers prevented a full checkpoint (`busy_frames > 0`), the backup is aborted with a descriptive error rather than silently copying an incomplete snapshot.
- Version bumped to **1.0.0-2** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 23), and `app/build.gradle.kts` (versionCode 23).

## [1.0.0-1] - 2026-06-11

### Added

- **First signed GitHub Release** — both the Windows MSI and the Android APK are now attached directly to GitHub Release assets on every `release` event.
  - Windows workflow (`build-windows.yml`) now fires on `release: types: [published]` and uploads the `.msi` via `softprops/action-gh-release@v2`.
  - Android workflow (`build-android.yml`) decodes a base64-encoded release keystore from the `ANDROID_KEYSTORE_BASE64` secret, writes it to a temp path, and passes the path to Gradle as `ANDROID_KEY_STORE_PATH`.
- **Hard-fail Android release signing** — the release APK build no longer falls back to debug signing if keystore secrets are absent; it fails immediately with a descriptive error. All four secrets (`ANDROID_KEYSTORE_BASE64`, `ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`) are validated before the build starts.
- **Release keystore** (`steloptc`, RSA 4096, valid ~27 years) generated and documented in `.github/SIGNING.md`. The same key must be used for all future releases to allow in-place APK upgrades on Android.
- **`build.gradle.kts` signing patch step** — after `cargo tauri android init` regenerates `gen/android/`, CI injects the signing config so the committed file and the CI-generated file stay in sync.

### Changed

- Version bumped to **1.0.0-1** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 22), and `app/build.gradle.kts` (versionCode 22). The version uses a numeric pre-release suffix (`-1`) rather than `rc.1` because the WiX MSI bundler requires pre-release identifiers to be numeric-only (≤ 65535); `rc` is non-numeric and rejected at bundle time.
- README Downloads table updated: both Windows and Android rows now point to GitHub Releases for release binaries.

## [0.1.21] - 2026-06-11

### Changed

- **Content-Security-Policy hardened** — replaced `"csp": null` (no policy) with a locked-down policy in `tauri.conf.json`:
  - `default-src 'self' ipc: http://ipc.localhost` — baseline; covers Tauri IPC for all unspecified directive fallbacks.
  - `script-src 'self'` — no remote or inline scripts; all JS is Vite-bundled.
  - `style-src 'self' 'unsafe-inline' https://fonts.googleapis.com` — Svelte injects inline styles; Inter font CSS loaded from Google Fonts.
  - `font-src 'self' https://fonts.gstatic.com` — Google Fonts glyph files.
  - `img-src 'self' data: blob:` — `data:` for base64 photo lightbox round-trip and QR canvas output; `blob:` for canvas-generated blobs.
  - `connect-src 'self' ipc: http://ipc.localhost` — explicit Tauri IPC allowance (required for `invoke()` calls).
  - `worker-src blob:` — html5-qrcode/ZXing creates its decoder web worker from a `blob:` URI; without it the camera scanner fails.
  - No remote script origins; no `'unsafe-eval'`.
- Version bumped to **0.1.21**.

## [0.1.20] - 2026-06-11

### Added

- **Forced password change on first login** — fresh installations (or any account with `must_change_password = 1`) are blocked from accessing the application until a new password is set.
  - New DB migration (006): adds `must_change_password BOOLEAN NOT NULL DEFAULT 0` to the `users` table; the seeded `admin` row is set to `1` so the default `admin/admin` credential can never grant unguarded access.
  - Login response now carries a `must_change_password` flag; if `true`, the front end routes to a full-screen **Set a New Password** overlay before the app shell renders. All other navigation is blocked until the change is complete.
  - New `ForceChangePassword.svelte` component: validates minimum 8-character length and confirmation match, calls the new `change_password` Tauri command, then clears the gate.
  - New `change_password` Tauri command: validates the new password (≥ 8 chars), bcrypt-hashes it, clears `must_change_password`, and writes an audit entry.
  - `mustChangePassword` Svelte store added to `auth.ts`; `setAuth` now accepts an optional third argument to set it; `clearAuth` resets it.

### Changed

- Login hint updated: "First login: admin / admin (you will be prompted to set a new password)".
- Version bumped to **0.1.20** across `package.json`, `Cargo.toml`, `tauri.conf.json` (versionCode 20), `app/build.gradle.kts` (versionCode 20), and sidebar display.
