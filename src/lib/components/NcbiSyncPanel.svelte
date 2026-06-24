<script lang="ts">
  import { onMount } from 'svelte';
  import { currentUser } from '../stores/auth';
  import { addNotification } from '../stores/app';
  import {
    importNcbiTaxonomy,
    resolveNcbiConflict,
    listNcbiSyncLog,
    type NcbiTaxonRecord,
    type NcbiConflictSummary,
    type ImportNcbiTaxonomyResult,
    type NcbiSyncLog,
  } from '../api';

  // ── state ─────────────────────────────────────────────────────────────────

  let jsonInput = $state('');
  let parseError = $state<string | null>(null);
  let dryRunResult = $state<ImportNcbiTaxonomyResult | null>(null);
  let importing = $state(false);
  let resolving = $state<string | null>(null); // sync_log_id being resolved

  let pendingConflicts = $state<NcbiSyncLog[]>([]);
  let recentLogs = $state<NcbiSyncLog[]>([]);
  let loadingLogs = $state(false);

  // ── helpers ───────────────────────────────────────────────────────────────

  function parseInput(): NcbiTaxonRecord[] | null {
    parseError = null;
    const trimmed = jsonInput.trim();
    if (!trimmed) {
      parseError = 'Paste NCBI taxon JSON above.';
      return null;
    }
    try {
      const parsed = JSON.parse(trimmed);
      const arr: unknown[] = Array.isArray(parsed) ? parsed : [parsed];
      for (const item of arr) {
        if (
          typeof item !== 'object' ||
          item === null ||
          typeof (item as any).ncbi_taxon_id !== 'number' ||
          typeof (item as any).name !== 'string' ||
          typeof (item as any).rank !== 'string'
        ) {
          parseError = 'Each record must have ncbi_taxon_id (number), name (string), and rank (string).';
          return null;
        }
      }
      return arr as NcbiTaxonRecord[];
    } catch (e: any) {
      parseError = `JSON parse error: ${e.message}`;
      return null;
    }
  }

  async function handleDryRun() {
    const taxa = parseInput();
    if (!taxa) return;
    importing = true;
    dryRunResult = null;
    try {
      dryRunResult = await importNcbiTaxonomy(taxa, true);
    } catch (e: any) {
      addNotification(e.message ?? 'Dry-run failed.', 'error');
    } finally {
      importing = false;
    }
  }

  async function handleImport() {
    const taxa = parseInput();
    if (!taxa) return;
    importing = true;
    try {
      const result = await importNcbiTaxonomy(taxa, false);
      addNotification(
        `Import complete — ${result.imported} imported, ${result.updated} updated, ${result.skipped_overrides} skipped (local override), ${result.conflicts.length} conflict(s).`,
        result.conflicts.length > 0 ? 'warning' : 'success'
      );
      dryRunResult = null;
      jsonInput = '';
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

  const jsonPlaceholder = '[{"ncbi_taxon_id": 4930, "name": "Saccharomyces cerevisiae", "rank": "species", "parent_ncbi_id": 4892}]';

  function parseConflictDetails(raw: string | null): { name?: { local: string; ncbi: string }; rank?: { local: string; ncbi: string } } | null {
    if (!raw) return null;
    try { return JSON.parse(raw); } catch { return null; }
  }

  onMount(refreshLogs);
</script>

<div>
  <div class="page-header">
    <h1>NCBI Taxonomy Sync</h1>
  </div>

  {#if $currentUser?.role !== 'admin'}
    <div class="card">
      <p style="color: var(--color-text-muted, #6b7280);">Only administrators can manage NCBI taxonomy sync.</p>
    </div>
  {:else}
    <!-- Import panel -->
    <div class="card" style="margin-bottom: 24px;">
      <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 4px;">Import NCBI Records</h2>
      <p style="font-size: 13px; color: #6b7280; margin-bottom: 16px;">
        Paste a JSON array of NCBI taxon records (fetched from the NCBI Taxonomy API). Each record
        must include <code>ncbi_taxon_id</code>, <code>name</code>, and <code>rank</code>.
        Taxa with <em>local override</em> set will never be modified.
      </p>

      <div class="form-group" style="margin-bottom: 12px;">
        <label for="ncbi-json-input">NCBI Taxon JSON</label>
        <textarea
          id="ncbi-json-input"
          rows={8}
          placeholder={jsonPlaceholder}
          bind:value={jsonInput}
          style="font-family: monospace; font-size: 12px; width: 100%; resize: vertical;"
        ></textarea>
        {#if parseError}
          <p style="font-size: 12px; color: var(--color-danger, #dc2626); margin-top: 4px;">{parseError}</p>
        {/if}
      </div>

      <div style="display: flex; gap: 8px; flex-wrap: wrap;">
        <button
          class="btn btn-secondary"
          onclick={handleDryRun}
          disabled={importing || !jsonInput.trim()}
        >
          {importing && dryRunResult === null ? 'Running…' : 'Dry Run'}
        </button>
        {#if dryRunResult}
          <button
            class="btn btn-primary"
            onclick={handleImport}
            disabled={importing}
          >
            {importing ? 'Importing…' : 'Confirm Import'}
          </button>
        {/if}
      </div>

      <!-- Dry-run result summary -->
      {#if dryRunResult}
        <div style="margin-top: 16px; padding: 12px; background: #f9fafb; border-radius: 6px; border: 1px solid #e5e7eb;">
          <p style="font-weight: 600; margin-bottom: 8px;">Dry-run result (no changes written)</p>
          <ul style="font-size: 13px; list-style: none; padding: 0; margin: 0;">
            <li>Would import: <strong>{dryRunResult.imported}</strong></li>
            <li>Would update: <strong>{dryRunResult.updated}</strong></li>
            <li>Skipped (local override): <strong>{dryRunResult.skipped_overrides}</strong></li>
            <li>Conflicts detected: <strong>{dryRunResult.conflicts.length}</strong></li>
          </ul>

          {#if dryRunResult.conflicts.length > 0}
            <div style="margin-top: 12px;">
              <p style="font-size: 13px; font-weight: 600; color: var(--color-warning, #d97706); margin-bottom: 6px;">
                Conflicts (would be logged for manual review):
              </p>
              {#each dryRunResult.conflicts as c (c.ncbi_taxon_id)}
                <div style="font-size: 12px; padding: 6px 8px; background: #fffbeb; border: 1px solid #fcd34d; border-radius: 4px; margin-bottom: 4px;">
                  <strong>{c.ncbi_name}</strong> (NCBI #{c.ncbi_taxon_id})
                  {#if c.local_name}— local: <em>{c.local_name}</em>{/if}<br />
                  <span style="color: #6b7280;">{c.conflict_details}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Pending conflicts -->
    <div class="card" style="margin-bottom: 24px;">
      <div style="display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px;">
        <h2 style="font-size: 16px; font-weight: 700; margin: 0;">
          Pending Conflicts
          {#if pendingConflicts.length > 0}
            <span style="font-size: 13px; font-weight: 400; color: var(--color-warning, #d97706); margin-left: 6px;">
              ({pendingConflicts.length})
            </span>
          {/if}
        </h2>
        <button class="btn btn-ghost" onclick={refreshLogs} disabled={loadingLogs}>
          {loadingLogs ? 'Loading…' : 'Refresh'}
        </button>
      </div>

      {#if pendingConflicts.length === 0}
        <p style="font-size: 13px; color: #6b7280;">No pending conflicts.</p>
      {:else}
        {#each pendingConflicts as log (log.id)}
          {@const details = parseConflictDetails(log.conflict_details)}
          <div style="padding: 10px 12px; border: 1px solid #fcd34d; border-radius: 6px; background: #fffbeb; margin-bottom: 8px;">
            <div style="font-size: 13px; font-weight: 600; margin-bottom: 4px;">
              NCBI #{log.ncbi_taxon_id}
              {#if log.taxon_id}
                — local taxon <code>{log.taxon_id}</code>
              {/if}
            </div>
            {#if details}
              <div style="font-size: 12px; color: #6b7280; margin-bottom: 8px;">
                {#if details.name}
                  Name: <strong>{details.name.local}</strong> (local) vs <strong>{details.name.ncbi}</strong> (NCBI)
                {/if}
                {#if details.rank}
                  &nbsp;Rank: <strong>{details.rank.local}</strong> vs <strong>{details.rank.ncbi}</strong>
                {/if}
              </div>
            {/if}
            <p style="font-size: 11px; color: #9ca3af; margin-bottom: 8px;">
              Created {log.created_at.slice(0, 10)}
            </p>
            <div style="display: flex; gap: 6px; flex-wrap: wrap;">
              <button
                class="btn btn-secondary"
                style="font-size: 12px; padding: 4px 10px;"
                onclick={() => handleResolve(log.id, 'kept_local')}
                disabled={resolving === log.id}
              >Keep Local</button>
              <button
                class="btn btn-primary"
                style="font-size: 12px; padding: 4px 10px;"
                onclick={() => handleResolve(log.id, 'accepted_ncbi')}
                disabled={resolving === log.id}
              >Accept NCBI</button>
              <button
                class="btn btn-ghost"
                style="font-size: 12px; padding: 4px 10px;"
                onclick={() => handleResolve(log.id, 'merged')}
                disabled={resolving === log.id}
              >Merged</button>
            </div>
          </div>
        {/each}
      {/if}
    </div>

    <!-- Recent sync log -->
    <div class="card">
      <h2 style="font-size: 16px; font-weight: 700; margin-bottom: 12px;">Recent Sync Log (last 50)</h2>

      {#if loadingLogs}
        <p style="font-size: 13px; color: #6b7280;">Loading…</p>
      {:else if recentLogs.length === 0}
        <p style="font-size: 13px; color: #6b7280;">No sync activity yet.</p>
      {:else}
        <div style="overflow-x: auto;">
          <table style="width: 100%; font-size: 12px; border-collapse: collapse;">
            <thead>
              <tr style="text-align: left; border-bottom: 1px solid #e5e7eb;">
                <th style="padding: 6px 8px; font-weight: 600;">Date</th>
                <th style="padding: 6px 8px; font-weight: 600;">Type</th>
                <th style="padding: 6px 8px; font-weight: 600;">NCBI ID</th>
                <th style="padding: 6px 8px; font-weight: 600;">Local Taxon</th>
                <th style="padding: 6px 8px; font-weight: 600;">Resolution</th>
              </tr>
            </thead>
            <tbody>
              {#each recentLogs as log (log.id)}
                <tr style="border-bottom: 1px solid #f3f4f6;">
                  <td style="padding: 6px 8px; white-space: nowrap;">{log.created_at.slice(0, 10)}</td>
                  <td style="padding: 6px 8px;">
                    <span style="padding: 2px 6px; border-radius: 9999px; font-size: 11px; background: {log.sync_type === 'conflict' ? '#fef3c7' : '#f0fdf4'}; color: {log.sync_type === 'conflict' ? '#92400e' : '#166534'};">
                      {log.sync_type}
                    </span>
                  </td>
                  <td style="padding: 6px 8px;">{log.ncbi_taxon_id ?? '—'}</td>
                  <td style="padding: 6px 8px; font-family: monospace;">{log.taxon_id ?? '—'}</td>
                  <td style="padding: 6px 8px;">{log.resolution ?? (log.resolved_at ? 'resolved' : '—')}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  {/if}
</div>
