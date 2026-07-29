<script lang="ts">
  import { onMount } from 'svelte';
  import { currentUser } from '../stores/auth';
  import { addNotification } from '../stores/app';
  import {
    importNcbiTaxonomy,
    resolveNcbiConflict,
    listNcbiSyncLog,
    type NcbiTaxonRecord,
    type ImportNcbiTaxonomyResult,
    type NcbiSyncLog,
  } from '../api';
  import { parseNcbiInput, isBackboneRank, type ParseResult, type ParsedTaxon } from '../ncbiParse';

  // ── Import state ──────────────────────────────────────────────────────────

  let rawInput = $state('');
  let importing = $state(false);
  let dryRunResult = $state<ImportNcbiTaxonomyResult | null>(null);
  let lastImportResult = $state<ImportNcbiTaxonomyResult | null>(null);

  // Rows the operator has explicitly toggled. Absence means "use the default",
  // which is: include anything the backbone can actually hold. Keying on the
  // taxon ID rather than the array index means re-pasting or re-sorting the
  // input does not silently move a tick from one row to another.
  let overrides = $state<Record<number, boolean>>({});

  let parsed = $derived<ParseResult>(parseNcbiInput(rawInput));

  function defaultInclude(rec: ParsedTaxon): boolean {
    return isBackboneRank(rec.rank);
  }

  function isIncluded(rec: ParsedTaxon): boolean {
    return overrides[rec.ncbi_taxon_id] ?? defaultInclude(rec);
  }

  let selected = $derived(parsed.records.filter(isIncluded));

  // Changing the input invalidates a dry run computed from the previous text.
  // Without this the "Confirm Import" button stays live and would import
  // something other than what the preview described.
  let dryRunFor = $state('');
  $effect(() => {
    const signature = JSON.stringify(selected.map((r) => r.ncbi_taxon_id));
    if (dryRunResult && signature !== dryRunFor) {
      dryRunResult = null;
    }
  });

  // ── Sync log state ────────────────────────────────────────────────────────

  let pendingConflicts = $state<NcbiSyncLog[]>([]);
  let recentLogs = $state<NcbiSyncLog[]>([]);
  let loadingLogs = $state(false);
  let resolving = $state<string | null>(null);

  // ── Helpers ───────────────────────────────────────────────────────────────

  function toRecords(rows: ParsedTaxon[]): NcbiTaxonRecord[] {
    return rows.map((r) => ({
      ncbi_taxon_id: r.ncbi_taxon_id,
      name: r.name,
      rank: r.rank,
      parent_ncbi_id: r.parent_ncbi_id,
    }));
  }

  function toggle(rec: ParsedTaxon) {
    overrides = { ...overrides, [rec.ncbi_taxon_id]: !isIncluded(rec) };
  }

  function selectAll() {
    const next: Record<number, boolean> = {};
    for (const r of parsed.records) next[r.ncbi_taxon_id] = true;
    overrides = next;
  }

  function selectNone() {
    const next: Record<number, boolean> = {};
    for (const r of parsed.records) next[r.ncbi_taxon_id] = false;
    overrides = next;
  }

  function selectBackboneOnly() {
    overrides = {};
  }

  function clearInput() {
    rawInput = '';
    overrides = {};
    dryRunResult = null;
    lastImportResult = null;
  }

  async function copyLookupUrl() {
    if (!parsed.lookupUrl) return;
    try {
      await navigator.clipboard.writeText(parsed.lookupUrl);
      addNotification('Lookup URL copied to the clipboard.', 'success');
    } catch {
      // Clipboard access can be denied; the URL is visible and selectable anyway.
      addNotification('Could not copy — select the URL and copy it manually.', 'warning');
    }
  }

  async function handleDryRun() {
    if (selected.length === 0) return;
    importing = true;
    dryRunResult = null;
    try {
      dryRunResult = await importNcbiTaxonomy(toRecords(selected), true);
      dryRunFor = JSON.stringify(selected.map((r) => r.ncbi_taxon_id));
    } catch (e: any) {
      addNotification(e.message ?? 'Dry run failed.', 'error');
    } finally {
      importing = false;
    }
  }

  async function handleImport() {
    if (selected.length === 0) return;
    importing = true;
    try {
      const result = await importNcbiTaxonomy(toRecords(selected), false);
      lastImportResult = result;
      addNotification(
        `Import complete — ${result.imported} imported, ${result.updated} updated, ` +
          `${result.parents_linked} parent link(s), ${result.conflicts.length} conflict(s).`,
        result.conflicts.length > 0 ? 'warning' : 'success'
      );
      dryRunResult = null;
      rawInput = '';
      overrides = {};
      await refreshLogs();
    } catch (e: any) {
      addNotification(e.message ?? 'Import failed.', 'error');
    } finally {
      importing = false;
    }
  }

  async function handleResolve(syncLogId: string, resolution: 'kept_local' | 'accepted_ncbi' | 'merged') {
    resolving = syncLogId;
    try {
      await resolveNcbiConflict(syncLogId, resolution);
      addNotification('Conflict resolved.', 'success');
      await refreshLogs();
    } catch (e: any) {
      addNotification(e.message ?? 'Resolution failed.', 'error');
    } finally {
      resolving = null;
    }
  }

  async function refreshLogs() {
    loadingLogs = true;
    try {
      [pendingConflicts, recentLogs] = await Promise.all([
        listNcbiSyncLog(true),
        listNcbiSyncLog(false, 50),
      ]);
    } catch (e: any) {
      addNotification(e.message ?? 'Failed to load sync log.', 'error');
    } finally {
      loadingLogs = false;
    }
  }

  function parseConflictDetails(raw: string | null): { name?: { local: string; ncbi: string }; rank?: { local: string; ncbi: string } } | null {
    if (!raw) return null;
    try { return JSON.parse(raw); } catch { return null; }
  }

  // Worked examples of every accepted shape, so "what does it want?" is one
  // click rather than a trip to the docs.
  const EXAMPLES: Array<{ label: string; hint: string; text: string }> = [
    {
      label: 'JSON records',
      hint: 'The canonical shape — one object per taxon.',
      text: `[
  { "ncbi_taxon_id": 4751, "name": "Fungi", "rank": "kingdom", "parent_ncbi_id": null },
  { "ncbi_taxon_id": 5204, "name": "Basidiomycota", "rank": "phylum", "parent_ncbi_id": 4751 }
]`,
    },
    {
      label: 'E-utilities esummary',
      hint: 'Paste an esummary.fcgi JSON response unmodified.',
      text: `{
  "header": { "type": "esummary", "version": "0.3" },
  "result": {
    "uids": ["5204"],
    "5204": { "uid": "5204", "scientificname": "Basidiomycota", "rank": "phylum", "parenttaxid": "4751" }
  }
}`,
    },
    {
      label: 'E-utilities efetch XML',
      hint: 'Carries the full lineage — one record brings its whole backbone.',
      text: `<TaxaSet>
  <Taxon>
    <TaxId>5326</TaxId>
    <ScientificName>Pleurotus</ScientificName>
    <ParentTaxId>5204</ParentTaxId>
    <Rank>genus</Rank>
    <LineageEx>
      <Taxon><TaxId>4751</TaxId><ScientificName>Fungi</ScientificName><Rank>kingdom</Rank></Taxon>
      <Taxon><TaxId>5204</TaxId><ScientificName>Basidiomycota</ScientificName><Rank>phylum</Rank></Taxon>
    </LineageEx>
  </Taxon>
</TaxaSet>`,
    },
    {
      label: 'Table (CSV / TSV)',
      hint: 'Any spreadsheet with taxid, name, and rank columns.',
      text: `taxid,name,rank,parent_taxid
4751,Fungi,kingdom,
5204,Basidiomycota,phylum,4751`,
    },
    {
      label: 'taxdump rows',
      hint: 'nodes.dmp and names.dmp lines, in either order.',
      text: `5204\t|\t4751\t|\tphylum\t|\t\t|
5204\t|\tBasidiomycota\t|\t\t|\tscientific name\t|`,
    },
  ];

  let showExamples = $state(false);

  function loadExample(text: string) {
    rawInput = text;
    overrides = {};
    dryRunResult = null;
    showExamples = false;
  }

  onMount(refreshLogs);
</script>

<div>
  <div class="page-header">
    <h1>NCBI Taxonomy Sync</h1>
  </div>

  {#if $currentUser?.role !== 'admin'}
    <div class="card">
      <p class="muted">Only administrators can manage NCBI taxonomy sync.</p>
    </div>
  {:else}
    <!-- ── Import ───────────────────────────────────────────────────────── -->
    <div class="card" style="margin-bottom: 24px;">
      <h2 class="section-title">Import NCBI Records</h2>
      <p class="muted section-lede">
        Paste whatever you have. This screen recognises E-utilities responses (JSON or XML),
        <code>taxdump</code> rows, a CSV/TSV table, or a plain list of taxon IDs, and shows you
        exactly what it will create before anything is written. The taxonomy backbone stores
        <strong>kingdom through genus</strong>; species belong in the Species Registry. Taxa marked
        <em>local override</em> are never modified.
      </p>

      <div class="input-toolbar">
        <label for="ncbi-input" class="input-label">Paste NCBI data</label>
        <div class="toolbar-actions">
          <button class="btn btn-sm" onclick={() => (showExamples = !showExamples)} title="Show a worked example of each accepted format">
            {showExamples ? 'Hide examples' : 'Show examples'}
          </button>
          <button class="btn btn-sm" onclick={clearInput} disabled={!rawInput} title="Clear the pasted text and start over">
            Clear
          </button>
        </div>
      </div>

      {#if showExamples}
        <div class="examples">
          {#each EXAMPLES as ex}
            <div class="example">
              <div class="example-head">
                <strong>{ex.label}</strong>
                <button class="btn btn-sm" onclick={() => loadExample(ex.text)} title="Load this example into the box above">Load</button>
              </div>
              <p class="muted example-hint">{ex.hint}</p>
            </div>
          {/each}
        </div>
      {/if}

      <textarea
        id="ncbi-input"
        rows={8}
        bind:value={rawInput}
        class="mono-input"
        placeholder={'Paste an esummary/efetch response, taxdump rows, a CSV table, or a list of taxon IDs…'}
        title="Paste NCBI taxonomy data in any supported format — it is parsed as you type"
      ></textarea>

      {#if rawInput.trim()}
        <div class="detect-row">
          <span class="format-badge" title="The format detected from what you pasted">{parsed.formatLabel}</span>
          {#if parsed.records.length > 0}
            <span class="muted">
              {parsed.records.length} record{parsed.records.length === 1 ? '' : 's'} parsed ·
              <strong>{selected.length}</strong> selected for import
            </span>
          {/if}
        </div>
      {/if}

      <!-- Guidance that is not an error -->
      {#each parsed.notes as note}
        <p class="note-line">{note}</p>
      {/each}

      <!-- A list of names or IDs needs a round trip through NCBI -->
      {#if parsed.lookupUrl}
        <div class="lookup-box">
          <p>
            SteloPTC works offline and never calls out to the internet on its own. Open this
            E-utilities URL in a browser, then paste the response back into the box above:
          </p>
          <code class="lookup-url">{parsed.lookupUrl}</code>
          <button class="btn btn-sm" onclick={copyLookupUrl} title="Copy this URL to the clipboard">Copy URL</button>
        </div>
      {/if}

      <!-- Lines the parser could not use -->
      {#if parsed.issues.length > 0}
        <div class="issue-box">
          <p class="issue-title">{parsed.issues.length} line{parsed.issues.length === 1 ? '' : 's'} could not be read</p>
          <ul>
            {#each parsed.issues.slice(0, 8) as issue}
              <li><code>{issue.line}</code> — {issue.reason}</li>
            {/each}
          </ul>
          {#if parsed.issues.length > 8}
            <p class="muted">…and {parsed.issues.length - 8} more.</p>
          {/if}
        </div>
      {/if}

      <!-- Preview: exactly what will be sent -->
      {#if parsed.records.length > 0}
        <div class="preview">
          <div class="preview-head">
            <strong>Preview</strong>
            <div class="preview-actions">
              <button class="btn btn-sm" onclick={selectBackboneOnly} title="Select every record the backbone can hold (kingdom through genus)">Backbone only</button>
              <button class="btn btn-sm" onclick={selectAll} title="Select every parsed record">All</button>
              <button class="btn btn-sm" onclick={selectNone} title="Deselect everything">None</button>
            </div>
          </div>
          <div class="preview-scroll">
            <table>
              <thead>
                <tr>
                  <th><span class="sr-only">Include</span></th>
                  <th title="NCBI taxon ID">Taxon ID</th>
                  <th title="Scientific name">Name</th>
                  <th title="Rank as supplied">Rank</th>
                  <th title="Parent taxon ID, used to build the tree">Parent</th>
                  <th title="Which part of the pasted data this row came from">Source</th>
                </tr>
              </thead>
              <tbody>
                {#each parsed.records as rec (rec.ncbi_taxon_id)}
                  {@const backbone = isBackboneRank(rec.rank)}
                  <tr class:excluded={!isIncluded(rec)}>
                    <td>
                      <input
                        type="checkbox"
                        checked={isIncluded(rec)}
                        onchange={() => toggle(rec)}
                        aria-label="Include {rec.name} in the import"
                        title={backbone ? `Include ${rec.name}` : `${rec.name} has rank "${rec.rank}", which the backbone cannot store — importing it would be a no-op`}
                        class="row-check"
                      />
                    </td>
                    <td>{rec.ncbi_taxon_id}</td>
                    <td><em>{rec.name}</em></td>
                    <td>
                      {rec.rank}
                      {#if !backbone}
                        <span class="badge badge-yellow" title="Below genus — not part of the taxonomy backbone">below genus</span>
                      {/if}
                    </td>
                    <td>{rec.parent_ncbi_id ?? '—'}</td>
                    <td class="muted">{rec.source}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>
        </div>
      {/if}

      <div class="action-row">
        <button
          class="btn"
          onclick={handleDryRun}
          disabled={importing || selected.length === 0}
          title="Show exactly what would change, without writing anything"
        >
          {importing && !dryRunResult ? 'Checking…' : `Dry run (${selected.length})`}
        </button>
        {#if dryRunResult}
          <button
            class="btn btn-primary"
            onclick={handleImport}
            disabled={importing}
            title="Apply the changes described in the dry-run summary"
          >
            {importing ? 'Importing…' : 'Confirm import'}
          </button>
        {/if}
      </div>

      <!-- Dry-run / result summary -->
      {#each [dryRunResult, lastImportResult].filter(Boolean) as result (result!.dry_run)}
        <div class="result-box">
          <p class="result-title">
            {result!.dry_run ? 'Dry run — nothing has been written' : 'Import applied'}
          </p>
          <ul class="result-list">
            <li>{result!.dry_run ? 'Would create' : 'Created'}: <strong>{result!.imported}</strong></li>
            <li>{result!.dry_run ? 'Would update' : 'Updated'}: <strong>{result!.updated}</strong></li>
            <li>
              Parent links {result!.dry_run ? 'resolvable' : 'resolved'}:
              <strong>{result!.parents_linked}</strong>
              <span class="muted">— how much of a tree this builds, not just how many rows</span>
            </li>
            <li>Skipped (local override): <strong>{result!.skipped_overrides}</strong></li>
            <li>Conflicts: <strong>{result!.conflicts.length}</strong></li>
          </ul>

          {#if result!.skipped_records.length > 0}
            <p class="result-subtitle">Not usable ({result!.skipped_records.length})</p>
            <ul class="result-list">
              {#each result!.skipped_records.slice(0, 6) as s}
                <li><strong>{s.name}</strong> (#{s.ncbi_taxon_id}) — {s.reason}</li>
              {/each}
            </ul>
            {#if result!.skipped_records.length > 6}
              <p class="muted">…and {result!.skipped_records.length - 6} more.</p>
            {/if}
          {/if}

          {#if result!.conflicts.length > 0}
            <p class="result-subtitle">Conflicts — logged for manual review</p>
            {#each result!.conflicts as c (c.ncbi_taxon_id)}
              <div class="conflict-card">
                <strong>{c.ncbi_name}</strong> (NCBI #{c.ncbi_taxon_id})
                {#if c.local_name}— local: <em>{c.local_name}</em>{/if}
                <div class="muted">{c.conflict_details}</div>
              </div>
            {/each}
          {/if}
        </div>
      {/each}
    </div>

    <!-- ── Pending conflicts ─────────────────────────────────────────────── -->
    <div class="card" style="margin-bottom: 24px;">
      <div class="card-head">
        <h2 class="section-title" style="margin:0;">
          Pending Conflicts
          {#if pendingConflicts.length > 0}
            <span class="count-badge">({pendingConflicts.length})</span>
          {/if}
        </h2>
        <button class="btn btn-sm" onclick={refreshLogs} disabled={loadingLogs} title="Reload the conflict list and sync log">
          {loadingLogs ? 'Loading…' : 'Refresh'}
        </button>
      </div>

      {#if pendingConflicts.length === 0}
        <p class="muted">No pending conflicts.</p>
      {:else}
        {#each pendingConflicts as log (log.id)}
          {@const details = parseConflictDetails(log.conflict_details)}
          <div class="conflict-card">
            <div class="conflict-head">
              NCBI #{log.ncbi_taxon_id}
              {#if log.taxon_id}— local taxon <code>{log.taxon_id}</code>{/if}
            </div>
            {#if details}
              <div class="conflict-diff">
                {#if details.name}
                  <div class="diff-row">
                    <span class="diff-label">Name</span>
                    <span class="diff-local" title="The value currently stored in this lab">{details.name.local}</span>
                    <span class="diff-arrow" aria-hidden="true">→</span>
                    <span class="diff-ncbi" title="The value NCBI supplied">{details.name.ncbi}</span>
                  </div>
                {/if}
                {#if details.rank}
                  <div class="diff-row">
                    <span class="diff-label">Rank</span>
                    <span class="diff-local" title="The value currently stored in this lab">{details.rank.local}</span>
                    <span class="diff-arrow" aria-hidden="true">→</span>
                    <span class="diff-ncbi" title="The value NCBI supplied">{details.rank.ncbi}</span>
                  </div>
                {/if}
              </div>
            {/if}
            <p class="muted conflict-date">Logged {log.created_at.slice(0, 10)}</p>
            <div class="conflict-actions">
              <button class="btn btn-sm" onclick={() => handleResolve(log.id, 'kept_local')} disabled={resolving === log.id} title="Keep this lab's values and mark the conflict resolved">Keep local</button>
              <button class="btn btn-sm btn-primary" onclick={() => handleResolve(log.id, 'accepted_ncbi')} disabled={resolving === log.id} title="Apply NCBI's values to the local taxon">Accept NCBI</button>
              <button class="btn btn-sm" onclick={() => handleResolve(log.id, 'merged')} disabled={resolving === log.id} title="Mark resolved after editing the taxon by hand">Merged</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <!-- ── Recent sync log ───────────────────────────────────────────────── -->
    <div class="card">
      <h2 class="section-title">Recent Sync Log (last 50)</h2>

      {#if loadingLogs}
        <p class="muted">Loading…</p>
      {:else if recentLogs.length === 0}
        <p class="muted">No sync activity yet.</p>
      {:else}
        <div class="preview-scroll">
          <table>
            <thead>
              <tr>
                <th title="When the entry was written">Date</th>
                <th title="What kind of sync event this was">Type</th>
                <th title="NCBI taxon ID involved">NCBI ID</th>
                <th title="Local taxon ID involved">Local Taxon</th>
                <th title="How a conflict was resolved, if it was">Resolution</th>
              </tr>
            </thead>
            <tbody>
              {#each recentLogs as log (log.id)}
                <tr>
                  <td style="white-space: nowrap;">{log.created_at.slice(0, 10)}</td>
                  <td>
                    <span class="badge {log.sync_type === 'conflict' ? 'badge-yellow' : 'badge-green'}">{log.sync_type}</span>
                  </td>
                  <td>{log.ncbi_taxon_id ?? '—'}</td>
                  <td><code>{log.taxon_id ?? '—'}</code></td>
                  <td>{log.resolution ?? (log.resolved_at ? 'resolved' : '—')}</td>
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
  /* Every colour here goes through a token with a light-mode fallback. The
     previous version hardcoded #f9fafb / #fffbeb / #6b7280 inline, which made
     the whole panel render as light-on-light in dark mode. */

  .muted { color: var(--color-text-muted, #6b7280); font-size: 13px; }
  .section-title { font-size: 16px; font-weight: 700; margin-bottom: 4px; }
  .section-lede { margin-bottom: 16px; line-height: 1.6; }

  .card-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 12px;
  }
  .count-badge {
    font-size: 13px;
    font-weight: 400;
    color: var(--color-warning, #d97706);
    margin-left: 6px;
  }

  .input-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 4px;
    flex-wrap: wrap;
  }
  .input-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text-muted, #6b7280);
    margin: 0;
  }
  .toolbar-actions { display: flex; gap: 6px; }

  .mono-input {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    width: 100%;
    resize: vertical;
  }

  .examples {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 8px;
    margin-bottom: 10px;
  }
  .example {
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: var(--radius-md, 6px);
    padding: 8px 10px;
    background: var(--color-surface-raised, #f8fafc);
  }
  :global(.dark) .example { background: #0f172a; border-color: #334155; }
  .example-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .example-hint { margin-top: 4px; font-size: 12px; }

  .detect-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
    flex-wrap: wrap;
  }
  .format-badge {
    display: inline-block;
    padding: 2px 10px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 600;
    background: var(--color-accent-soft, #dbeafe);
    color: var(--color-accent, #1e40af);
  }
  :global(.dark) .format-badge { background: #1e3a5f; color: #bfdbfe; }

  .note-line {
    margin-top: 8px;
    font-size: 12px;
    line-height: 1.6;
    color: var(--color-text-muted, #6b7280);
    padding-left: 10px;
    border-left: 3px solid var(--color-border, #e2e8f0);
  }

  .lookup-box {
    margin-top: 12px;
    padding: 12px;
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: var(--radius-md, 6px);
    background: var(--color-surface-raised, #f8fafc);
    font-size: 13px;
    line-height: 1.6;
  }
  :global(.dark) .lookup-box { background: #0f172a; border-color: #334155; }
  .lookup-url {
    display: block;
    margin: 8px 0;
    padding: 6px 8px;
    font-size: 11px;
    word-break: break-all;
    background: var(--color-bg, #fff);
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: 4px;
  }
  :global(.dark) .lookup-url { background: #020617; border-color: #334155; }

  .issue-box {
    margin-top: 12px;
    padding: 10px 12px;
    border: 1px solid var(--color-warning-border, #fcd34d);
    border-radius: var(--radius-md, 6px);
    background: var(--color-warning-bg, #fffbeb);
    font-size: 12px;
    line-height: 1.6;
  }
  :global(.dark) .issue-box { background: #422006; border-color: #854d0e; }
  .issue-title { font-weight: 600; margin-bottom: 4px; }
  .issue-box ul { margin: 0; padding-left: 18px; }
  .issue-box code { word-break: break-all; }

  .preview { margin-top: 14px; }
  .preview-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }
  .preview-actions { display: flex; gap: 6px; }
  .preview-scroll { overflow-x: auto; max-height: 340px; overflow-y: auto; }
  .row-check { width: auto; min-height: 0; }
  tr.excluded { opacity: 0.45; }

  .action-row { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 14px; }

  .result-box {
    margin-top: 16px;
    padding: 12px;
    border: 1px solid var(--color-border, #e2e8f0);
    border-radius: var(--radius-md, 6px);
    background: var(--color-surface-raised, #f8fafc);
  }
  :global(.dark) .result-box { background: #0f172a; border-color: #334155; }
  .result-title { font-weight: 600; margin-bottom: 8px; }
  .result-subtitle { font-weight: 600; font-size: 13px; margin: 12px 0 6px; }
  .result-list { font-size: 13px; list-style: none; padding: 0; margin: 0; line-height: 1.8; }

  .conflict-card {
    padding: 10px 12px;
    border: 1px solid var(--color-warning-border, #fcd34d);
    border-radius: var(--radius-md, 6px);
    background: var(--color-warning-bg, #fffbeb);
    margin-bottom: 8px;
    font-size: 13px;
  }
  :global(.dark) .conflict-card { background: #422006; border-color: #854d0e; }
  .conflict-head { font-weight: 600; margin-bottom: 6px; }
  .conflict-date { font-size: 11px; margin-bottom: 8px; }
  .conflict-actions { display: flex; gap: 6px; flex-wrap: wrap; }

  .conflict-diff { display: flex; flex-direction: column; gap: 4px; margin-bottom: 6px; }
  .diff-row { display: flex; align-items: center; gap: 8px; font-size: 12px; flex-wrap: wrap; }
  .diff-label {
    min-width: 44px;
    font-weight: 600;
    color: var(--color-text-muted, #6b7280);
  }
  .diff-local, .diff-ncbi {
    padding: 1px 6px;
    border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  }
  .diff-local { background: rgba(220, 38, 38, 0.12); }
  .diff-ncbi { background: rgba(5, 150, 105, 0.14); }
  .diff-arrow { color: var(--color-text-muted, #6b7280); }

  .sr-only {
    position: absolute;
    width: 1px; height: 1px;
    padding: 0; margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>
