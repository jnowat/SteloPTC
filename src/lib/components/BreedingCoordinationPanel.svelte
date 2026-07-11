<script lang="ts">
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import {
    getLabIdentity, listBreedingPrograms, exportCoordinationBundle,
    previewCoordinationImport, importCoordinationBundle, listCoordinationBundles,
    getCoordinationBundleJson,
    type IssuerIdentity, type BreedingProgram, type BundleImportPreview,
    type BundleRow, type SelectionDecision, type SelectionDisposition,
  } from '../api';

  // WP-72: Cross-lab breeding program coordination — federated, signed
  // selection-log exchange. Two labs running separate copies of the same breeding
  // program each accumulate their own selection records; this panel exports one
  // program's records as a signed, self-contained bundle a partner lab verifies
  // with only the issuer's public key and the embedded, recomputable per-record
  // hashes. On import the receiver reconciles per record — accept (merge in) or
  // skip — and the merge is folded into this lab's own audit chain. Merging is a
  // set union: additive, never overwriting a local record, and the local program's
  // metadata is left untouched (an absent program is created as a shell). A
  // selection record's strain must already exist locally (import it via the
  // taxonomy registry first) or the record is blocked. No network transport:
  // exporting downloads a JSON file; importing reads one. See
  // docs/breeding-coordination.md.

  const canWrite = $derived(
    $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor' || $currentUser?.role === 'tech',
  );

  let open = $state(false);
  let identity = $state<IssuerIdentity | null>(null);

  let programs = $state<BreedingProgram[]>([]);
  let selectedProgramId = $state('');
  let exporting = $state(false);

  let inbox = $state('');
  let previewing = $state(false);
  let importing = $state(false);
  let preview = $state<BundleImportPreview | null>(null);
  // source_key → chosen disposition (defaults to the previewed suggestion).
  let choices = $state<Record<string, SelectionDisposition>>({});

  let bundles = $state<BundleRow[]>([]);
  let loadingBundles = $state(false);

  async function toggle() {
    open = !open;
    if (open && !identity) await refresh();
  }

  async function refresh() {
    try {
      identity = await getLabIdentity();
      programs = await listBreedingPrograms();
      if (programs.length > 0 && !selectedProgramId) selectedProgramId = programs[0].id;
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load coordination panel', 'error');
    }
    await loadBundles();
  }

  async function loadBundles() {
    loadingBundles = true;
    try {
      bundles = await listCoordinationBundles();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load bundle register', 'error');
    } finally {
      loadingBundles = false;
    }
  }

  function downloadJson(json: string, filename: string) {
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function doExport() {
    if (!selectedProgramId) {
      addNotification('Select a breeding program to export', 'error');
      return;
    }
    exporting = true;
    try {
      const bundle = await exportCoordinationBundle(selectedProgramId);
      const json = JSON.stringify(bundle, null, 2);
      downloadJson(json, `breeding-coordination-${bundle.bundle_id.slice(0, 8)}.json`);
      addNotification(`Exported '${bundle.program.name}' — ${bundle.records.length} records`, 'success');
      await loadBundles();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to export bundle', 'error');
    } finally {
      exporting = false;
    }
  }

  async function onFile(event: Event) {
    const input = event.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    inbox = await file.text();
    input.value = '';
    preview = null;
  }

  async function doPreview() {
    if (!inbox.trim()) {
      addNotification('Paste or load a bundle to preview', 'error');
      return;
    }
    previewing = true;
    try {
      preview = await previewCoordinationImport(inbox);
      choices = {};
      for (const r of preview.records) choices[r.source_key] = r.suggested_disposition;
      addNotification(preview.verification.message, preview.verification.verified ? 'success' : 'error');
    } catch (e: any) {
      preview = null;
      addNotification(e?.message || 'Preview failed', 'error');
    } finally {
      previewing = false;
    }
  }

  async function doImport() {
    if (!preview?.verification.verified) {
      addNotification('Preview a verified bundle before importing', 'error');
      return;
    }
    importing = true;
    try {
      const decisions: SelectionDecision[] = preview.records.map((r) => ({
        source_key: r.source_key,
        disposition: choices[r.source_key] ?? r.suggested_disposition,
      }));
      const result = await importCoordinationBundle(inbox, decisions);
      const created = result.program_created ? ' (program created)' : '';
      addNotification(
        `Merged '${result.verification.program_name}'${created}: ${result.inserted} added, ${result.kept_local} kept local, ${result.skipped} skipped`,
        'success',
      );
      inbox = '';
      preview = null;
      choices = {};
      await refresh();
    } catch (e: any) {
      addNotification(e?.message || 'Import failed', 'error');
    } finally {
      importing = false;
    }
  }

  async function reexport(rec: BundleRow) {
    try {
      const json = await getCoordinationBundleJson(rec.id);
      downloadJson(json, `breeding-coordination-${rec.bundle_id.slice(0, 8)}.json`);
    } catch (e: any) {
      addNotification(e?.message || 'Failed to export bundle', 'error');
    }
  }

  function short(s: string | null, n = 12): string {
    if (!s) return '—';
    return s.length > n ? `${s.slice(0, n)}…` : s;
  }

  const dispositionOptions: SelectionDisposition[] = ['accept', 'skip'];
</script>

<div class="card" style="margin-bottom:16px;">
  <div class="bc-header">
    <strong>🧬 Cross-Lab Breeding Coordination (Federated)</strong>
    <button class="btn btn-sm" onclick={toggle}>{open ? 'Hide' : 'Show'}</button>
  </div>

  {#if open}
    <p class="bc-intro">
      Two labs running separate copies of the <em>same</em> breeding program each keep their
      own selection records. Export one program as a signed, self-contained bundle a partner
      lab verifies <em>independently</em> — using only this lab's public key and the embedded,
      recomputable per-record hashes. On import you decide, per record, whether to
      <strong>accept</strong> (merge it in) or <strong>skip</strong>. Merging is a set union —
      additive, never overwriting a local record, and it never changes the local program's
      metadata (an absent program is created as a coordinated copy). A selection record's
      strain must already exist locally (share it via the Taxonomy Registry first) or the
      record is <em>blocked</em>. No network transfer: exporting downloads a JSON file;
      importing reads one. See <code>docs/breeding-coordination.md</code>.
    </p>

    <!-- Lab identity -->
    <div class="bc-section">
      <div class="bc-section-title">This lab's issuer identity</div>
      {#if identity}
        <div class="bc-id-row">
          <span class="bc-label">Lab name</span>
          <span class="bc-name">{identity.lab_name}</span>
        </div>
        <div class="bc-id-row">
          <span class="bc-label">Public key</span>
          <code class="bc-mono">{identity.public_key}</code>
        </div>
        <p class="bc-hint">
          This is the same lab key used for specimen passports and the taxonomy registry — set
          the lab name in the Specimen Passports panel. Share this public key so partners can
          verify the bundles you export.
        </p>
      {:else}
        <p class="bc-empty">Loading identity…</p>
      {/if}
    </div>

    <!-- Export -->
    {#if canWrite}
      <div class="bc-section">
        <div class="bc-section-title">Export a breeding program's selection records</div>
        {#if programs.length === 0}
          <p class="bc-empty">No breeding programs yet — create one in the Breeding Programs view.</p>
        {:else}
          <div class="bc-actions">
            <select bind:value={selectedProgramId} class="bc-select">
              {#each programs as p}
                <option value={p.id}>{p.name}</option>
              {/each}
            </select>
            <button class="btn btn-sm" disabled={exporting} onclick={doExport}>
              {exporting ? 'Exporting…' : 'Export & Download'}
            </button>
          </div>
          <p class="bc-hint">Signs and downloads the selected program's selection records as JSON.</p>
        {/if}
      </div>
    {/if}

    <!-- Preview / Import -->
    <div class="bc-section">
      <div class="bc-section-title">Preview or import a received bundle</div>
      <textarea class="bc-textarea" rows="4" bind:value={inbox} placeholder="Paste coordination bundle JSON here, or load a file…"></textarea>
      <div class="bc-actions">
        <label class="btn btn-sm bc-file-btn">
          Load file…
          <input type="file" accept="application/json,.json" onchange={onFile} hidden />
        </label>
        <button class="btn btn-sm" disabled={previewing} onclick={doPreview}>{previewing ? 'Previewing…' : 'Preview'}</button>
        {#if canWrite}
          <button class="btn btn-sm btn-primary" disabled={importing || !preview?.verification.verified} onclick={doImport}>
            {importing ? 'Importing…' : 'Import with choices'}
          </button>
        {/if}
      </div>

      {#if preview}
        <div class="bc-verdict {preview.verification.verified ? 'bc-ok' : 'bc-fail'}">
          <div class="bc-verdict-head">
            {preview.verification.verified ? '✓' : '✗'} {preview.verification.message}
          </div>
          <div class="bc-verdict-meta">
            Program: <strong>{preview.verification.program_name}</strong> ·
            Issuer: <strong>{preview.verification.issuer_lab}</strong> ·
            {preview.verification.record_count} records ·
            {preview.program_exists_locally ? 'merges into your existing program' : 'creates a new coordinated copy'}
          </div>
          <ul class="bc-checks">
            {#each preview.verification.checks as c}
              <li class={c.ok ? 'bc-check-ok' : 'bc-check-fail'}>
                {c.ok ? '✓' : '✗'} <strong>{c.name}</strong> — {c.detail}
              </li>
            {/each}
          </ul>
        </div>

        {#if preview.verification.verified && preview.records.length > 0}
          <div class="bc-table-wrap">
            <table class="bc-table">
              <thead>
                <tr>
                  <th>Strain</th>
                  <th>Gen</th>
                  <th>Origin</th>
                  <th>Local status</th>
                  <th>Disposition</th>
                </tr>
              </thead>
              <tbody>
                {#each preview.records as r}
                  <tr>
                    <td title={r.detail}>{r.strain_scientific_name} {r.strain_code}</td>
                    <td>{r.generation_number}</td>
                    <td>{r.origin_lab}</td>
                    <td><span class="bc-status bc-status-{r.local_status}">{r.local_status}</span></td>
                    <td>
                      <select bind:value={choices[r.source_key]} disabled={!canWrite || r.local_status === 'blocked'}>
                        {#each dispositionOptions as d}
                          <option value={d}>{d}</option>
                        {/each}
                      </select>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      {/if}
    </div>

    <!-- Register -->
    <div class="bc-section">
      <div class="bc-section-title">Coordination bundle register</div>
      {#if loadingBundles}
        <p class="bc-empty">Loading…</p>
      {:else if bundles.length === 0}
        <p class="bc-empty">No bundles exported or imported yet.</p>
      {:else}
        <div class="bc-table-wrap">
          <table class="bc-table">
            <thead>
              <tr>
                <th>Direction</th>
                <th>Program</th>
                <th>Issuer</th>
                <th>Records</th>
                <th>Content hash</th>
                <th>When</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {#each bundles as rec}
                <tr>
                  <td><span class="bc-dir bc-dir-{rec.direction}">{rec.direction}</span></td>
                  <td>{rec.program_name}</td>
                  <td>{rec.issuer_lab}</td>
                  <td>{rec.record_count}</td>
                  <td><code title={rec.content_hash}>{short(rec.content_hash, 12)}</code></td>
                  <td>{short(rec.created_at, 19)}</td>
                  <td>
                    {#if rec.direction === 'issued'}
                      <button class="btn btn-sm" onclick={() => reexport(rec)}>Export</button>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .bc-header { display: flex; justify-content: space-between; align-items: center; }
  .bc-intro { font-size: 0.85rem; color: var(--color-text-secondary, #555); line-height: 1.45; margin: 0.5rem 0 0.75rem; }
  .bc-section { border-top: 1px solid var(--color-border, #eee); padding: 0.6rem 0; }
  .bc-section-title { font-weight: 600; font-size: 0.85rem; margin-bottom: 0.4rem; }
  .bc-id-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.35rem; }
  .bc-label { font-size: 0.75rem; font-weight: 600; min-width: 5.5rem; color: var(--color-text-secondary, #666); }
  .bc-name { font-weight: 600; }
  .bc-hint { font-size: 0.75rem; color: var(--color-text-secondary, #777); margin: 0.1rem 0 0; }
  .bc-mono {
    font-family: var(--font-mono, monospace); font-size: 0.72rem; word-break: break-all;
    background: var(--color-surface-2, #f7f7f8); padding: 0.25rem 0.4rem; border-radius: 4px;
    border: 1px solid var(--color-border, #e2e2e2); flex: 1; min-width: 12rem;
  }
  .bc-select { max-width: 16rem; }
  .bc-textarea { width: 100%; font-family: var(--font-mono, monospace); font-size: 0.72rem; resize: vertical; }
  .bc-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; margin-top: 0.4rem; }
  .bc-file-btn { cursor: pointer; }
  .bc-verdict { margin-top: 0.6rem; padding: 0.5rem 0.6rem; border-radius: 6px; font-size: 0.82rem; }
  .bc-ok { background: rgba(22, 101, 52, 0.08); border: 1px solid rgba(22, 101, 52, 0.35); }
  .bc-fail { background: rgba(185, 28, 28, 0.08); border: 1px solid rgba(185, 28, 28, 0.35); }
  .bc-verdict-head { font-weight: 600; }
  .bc-verdict-meta { font-size: 0.78rem; color: var(--color-text-secondary, #555); margin: 0.25rem 0; }
  .bc-checks { list-style: none; margin: 0.25rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.15rem; }
  .bc-check-ok { color: #166534; font-size: 0.78rem; }
  .bc-check-fail { color: #b91c1c; font-size: 0.78rem; }
  .bc-empty { font-size: 0.85rem; color: var(--color-text-secondary, #777); padding: 0.3rem 0; }
  .bc-table-wrap { overflow-x: auto; margin-top: 0.5rem; }
  .bc-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
  .bc-table th, .bc-table td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid var(--color-border, #eee); white-space: nowrap; }
  .bc-status { font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.4rem; border-radius: 999px; text-transform: uppercase; }
  .bc-status-new { background: rgba(46, 125, 50, 0.12); color: #2e7d32; }
  .bc-status-identical { background: rgba(120, 120, 120, 0.14); color: #555; }
  .bc-status-blocked { background: rgba(217, 119, 6, 0.14); color: #b45309; }
  .bc-dir { font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.4rem; border-radius: 999px; text-transform: uppercase; }
  .bc-dir-issued { background: rgba(21, 101, 192, 0.12); color: #1565c0; }
  .bc-dir-imported { background: rgba(46, 125, 50, 0.12); color: #2e7d32; }
</style>
