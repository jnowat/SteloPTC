<script lang="ts">
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import {
    getLabIdentity, exportTaxonomyRegistry, previewTaxonomyRegistryImport,
    importTaxonomyRegistry, listTaxonomyRegistries, getTaxonomyRegistryJson,
    type IssuerIdentity, type RegistryImportPreview, type RegistryRecordRow,
    type RecordDecision, type RecordDisposition,
  } from '../api';

  // WP-71: Shared taxonomy registry — federated, signed reference-data exchange.
  // A lab exports a signed, self-contained registry of its taxa/species/strains
  // that a partner lab verifies with only the issuer's public key and the
  // embedded, recomputable per-record hashes. On import the receiver decides, per
  // record, whether to accept (adopt), override (keep local), or fork (add a
  // divergent copy); the merge is folded into this lab's own audit chain. Import
  // is additive — it never overwrites or deletes a local record, and a strain is
  // always imported as unverified. No network transport: exporting downloads a
  // JSON file; importing reads one. See docs/taxonomy-registry.md.

  const canWrite = $derived(
    $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor' || $currentUser?.role === 'tech',
  );

  let open = $state(false);
  let identity = $state<IssuerIdentity | null>(null);

  let exporting = $state(false);

  let inbox = $state('');
  let previewing = $state(false);
  let importing = $state(false);
  let preview = $state<RegistryImportPreview | null>(null);
  // source_key → chosen disposition (defaults to the previewed suggestion).
  let choices = $state<Record<string, RecordDisposition>>({});

  let records = $state<RegistryRecordRow[]>([]);
  let loadingRecords = $state(false);

  async function toggle() {
    open = !open;
    if (open && !identity) await refresh();
  }

  async function refresh() {
    try {
      identity = await getLabIdentity();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load lab identity', 'error');
    }
    await loadRecords();
  }

  async function loadRecords() {
    loadingRecords = true;
    try {
      records = await listTaxonomyRegistries();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to load registry register', 'error');
    } finally {
      loadingRecords = false;
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
    exporting = true;
    try {
      const registry = await exportTaxonomyRegistry();
      const json = JSON.stringify(registry, null, 2);
      downloadJson(json, `taxonomy-registry-${registry.registry_id.slice(0, 8)}.json`);
      addNotification(`Exported registry — ${registry.records.length} records`, 'success');
      await loadRecords();
    } catch (e: any) {
      addNotification(e?.message || 'Failed to export registry', 'error');
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
      addNotification('Paste or load a registry to preview', 'error');
      return;
    }
    previewing = true;
    try {
      preview = await previewTaxonomyRegistryImport(inbox);
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
      addNotification('Preview a verified registry before importing', 'error');
      return;
    }
    importing = true;
    try {
      const decisions: RecordDecision[] = preview.records.map((r) => ({
        source_key: r.source_key,
        disposition: choices[r.source_key] ?? r.suggested_disposition,
      }));
      const result = await importTaxonomyRegistry(inbox, decisions);
      addNotification(
        `Imported: ${result.inserted} added, ${result.forked} forked, ${result.kept_local} kept local, ${result.skipped} skipped`,
        'success',
      );
      inbox = '';
      preview = null;
      choices = {};
      await loadRecords();
    } catch (e: any) {
      addNotification(e?.message || 'Import failed', 'error');
    } finally {
      importing = false;
    }
  }

  async function reexport(rec: RegistryRecordRow) {
    try {
      const json = await getTaxonomyRegistryJson(rec.id);
      downloadJson(json, `taxonomy-registry-${rec.registry_id.slice(0, 8)}.json`);
    } catch (e: any) {
      addNotification(e?.message || 'Failed to export registry', 'error');
    }
  }

  function short(s: string | null, n = 12): string {
    if (!s) return '—';
    return s.length > n ? `${s.slice(0, n)}…` : s;
  }

  const dispositionOptions: RecordDisposition[] = ['accept', 'override', 'fork'];
</script>

<div class="card" style="margin-bottom:16px;">
  <div class="tr-header">
    <strong>🌐 Shared Taxonomy Registry (Federated)</strong>
    <button class="btn btn-sm" onclick={toggle}>{open ? 'Hide' : 'Show'}</button>
  </div>

  {#if open}
    <p class="tr-intro">
      A taxonomy registry is a signed, self-contained bundle of this lab's taxa, species,
      and strains that a partner lab can verify <em>independently</em> — using only this
      lab's public key and the embedded, recomputable per-record hashes. On import you
      decide, per record, whether to <strong>accept</strong> (adopt), <strong>override</strong>
      (keep your local version), or <strong>fork</strong> (add a divergent copy). Importing is
      additive — it never overwrites or deletes a local record — and strains always arrive
      <em>unverified</em> (re-confirm locally). No network transfer: exporting downloads a JSON
      file; importing reads one. See <code>docs/taxonomy-registry.md</code>.
    </p>

    <!-- Lab identity -->
    <div class="tr-section">
      <div class="tr-section-title">This lab's issuer identity</div>
      {#if identity}
        <div class="tr-id-row">
          <span class="tr-label">Lab name</span>
          <span class="tr-name">{identity.lab_name}</span>
        </div>
        <div class="tr-id-row">
          <span class="tr-label">Public key</span>
          <code class="tr-mono">{identity.public_key}</code>
        </div>
        <p class="tr-hint">
          This is the same lab key used for specimen passports — set the lab name in the
          Specimen Passports panel. Share this public key so partners can verify the
          registries you export.
        </p>
      {:else}
        <p class="tr-empty">Loading identity…</p>
      {/if}
    </div>

    <!-- Export -->
    {#if canWrite}
      <div class="tr-section">
        <div class="tr-section-title">Export this lab's registry</div>
        <button class="btn btn-sm" disabled={exporting} onclick={doExport}>
          {exporting ? 'Exporting…' : 'Export & Download'}
        </button>
        <p class="tr-hint">Signs and downloads all of this lab's taxa, species, and strains as JSON.</p>
      </div>
    {/if}

    <!-- Preview / Import -->
    <div class="tr-section">
      <div class="tr-section-title">Preview or import a received registry</div>
      <textarea class="tr-textarea" rows="4" bind:value={inbox} placeholder="Paste registry JSON here, or load a file…"></textarea>
      <div class="tr-actions">
        <label class="btn btn-sm tr-file-btn">
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
        <div class="tr-verdict {preview.verification.verified ? 'tr-ok' : 'tr-fail'}">
          <div class="tr-verdict-head">
            {preview.verification.verified ? '✓' : '✗'} {preview.verification.message}
          </div>
          <div class="tr-verdict-meta">
            Issuer: <strong>{preview.verification.issuer_lab}</strong> ·
            {preview.verification.taxon_count} taxa · {preview.verification.species_count} species ·
            {preview.verification.strain_count} strains
          </div>
          <ul class="tr-checks">
            {#each preview.verification.checks as c}
              <li class={c.ok ? 'tr-check-ok' : 'tr-check-fail'}>
                {c.ok ? '✓' : '✗'} <strong>{c.name}</strong> — {c.detail}
              </li>
            {/each}
          </ul>
        </div>

        {#if preview.verification.verified && preview.records.length > 0}
          <div class="tr-table-wrap">
            <table class="tr-table">
              <thead>
                <tr>
                  <th>Type</th>
                  <th>Record</th>
                  <th>Local status</th>
                  <th>Disposition</th>
                </tr>
              </thead>
              <tbody>
                {#each preview.records as r}
                  <tr>
                    <td><span class="tr-type">{r.record_type}</span></td>
                    <td title={r.detail}>{r.name}</td>
                    <td><span class="tr-status tr-status-{r.local_status}">{r.local_status}</span></td>
                    <td>
                      <select bind:value={choices[r.source_key]} disabled={!canWrite}>
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
    <div class="tr-section">
      <div class="tr-section-title">Registry register</div>
      {#if loadingRecords}
        <p class="tr-empty">Loading…</p>
      {:else if records.length === 0}
        <p class="tr-empty">No registries exported or imported yet.</p>
      {:else}
        <div class="tr-table-wrap">
          <table class="tr-table">
            <thead>
              <tr>
                <th>Direction</th>
                <th>Issuer</th>
                <th>Records</th>
                <th>Content hash</th>
                <th>When</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {#each records as rec}
                <tr>
                  <td><span class="tr-dir tr-dir-{rec.direction}">{rec.direction}</span></td>
                  <td>{rec.issuer_lab}</td>
                  <td title="{rec.taxon_count} taxa · {rec.species_count} species · {rec.strain_count} strains">{rec.record_count}</td>
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
  .tr-header { display: flex; justify-content: space-between; align-items: center; }
  .tr-intro { font-size: 0.85rem; color: var(--color-text-secondary, #555); line-height: 1.45; margin: 0.5rem 0 0.75rem; }
  .tr-section { border-top: 1px solid var(--color-border, #eee); padding: 0.6rem 0; }
  .tr-section-title { font-weight: 600; font-size: 0.85rem; margin-bottom: 0.4rem; }
  .tr-id-row { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; margin-bottom: 0.35rem; }
  .tr-label { font-size: 0.75rem; font-weight: 600; min-width: 5.5rem; color: var(--color-text-secondary, #666); }
  .tr-name { font-weight: 600; }
  .tr-hint { font-size: 0.75rem; color: var(--color-text-secondary, #777); margin: 0.1rem 0 0; }
  .tr-mono {
    font-family: var(--font-mono, monospace); font-size: 0.72rem; word-break: break-all;
    background: var(--color-surface-2, #f7f7f8); padding: 0.25rem 0.4rem; border-radius: 4px;
    border: 1px solid var(--color-border, #e2e2e2); flex: 1; min-width: 12rem;
  }
  .tr-textarea { width: 100%; font-family: var(--font-mono, monospace); font-size: 0.72rem; resize: vertical; }
  .tr-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; margin-top: 0.4rem; }
  .tr-file-btn { cursor: pointer; }
  .tr-verdict { margin-top: 0.6rem; padding: 0.5rem 0.6rem; border-radius: 6px; font-size: 0.82rem; }
  .tr-ok { background: rgba(22, 101, 52, 0.08); border: 1px solid rgba(22, 101, 52, 0.35); }
  .tr-fail { background: rgba(185, 28, 28, 0.08); border: 1px solid rgba(185, 28, 28, 0.35); }
  .tr-verdict-head { font-weight: 600; }
  .tr-verdict-meta { font-size: 0.78rem; color: var(--color-text-secondary, #555); margin: 0.25rem 0; }
  .tr-checks { list-style: none; margin: 0.25rem 0 0; padding: 0; display: flex; flex-direction: column; gap: 0.15rem; }
  .tr-check-ok { color: #166534; font-size: 0.78rem; }
  .tr-check-fail { color: #b91c1c; font-size: 0.78rem; }
  .tr-empty { font-size: 0.85rem; color: var(--color-text-secondary, #777); padding: 0.3rem 0; }
  .tr-table-wrap { overflow-x: auto; margin-top: 0.5rem; }
  .tr-table { width: 100%; border-collapse: collapse; font-size: 0.8rem; }
  .tr-table th, .tr-table td { text-align: left; padding: 0.35rem 0.5rem; border-bottom: 1px solid var(--color-border, #eee); white-space: nowrap; }
  .tr-type { font-size: 0.72rem; font-weight: 600; text-transform: uppercase; color: var(--color-text-secondary, #666); }
  .tr-status { font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.4rem; border-radius: 999px; text-transform: uppercase; }
  .tr-status-new { background: rgba(46, 125, 50, 0.12); color: #2e7d32; }
  .tr-status-identical { background: rgba(120, 120, 120, 0.14); color: #555; }
  .tr-status-conflict { background: rgba(217, 119, 6, 0.14); color: #b45309; }
  .tr-dir { font-size: 0.7rem; font-weight: 600; padding: 0.1rem 0.4rem; border-radius: 999px; text-transform: uppercase; }
  .tr-dir-issued { background: rgba(21, 101, 192, 0.12); color: #1565c0; }
  .tr-dir-imported { background: rgba(46, 125, 50, 0.12); color: #2e7d32; }
</style>
