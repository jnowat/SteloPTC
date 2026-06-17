<script lang="ts">
  import { onMount } from 'svelte';
  import { getAuditLog, verifyAuditEntry, verifyAuditLineage } from '../api';
  import { addNotification } from '../stores/app';
  import DataState from './DataState.svelte';

  let entries = $state<any[]>([]);
  let total = $state(0);
  let page = $state(1);
  let totalPages = $state(0);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let filterEntity = $state('');
  let filterAction = $state('');

  // Per-row verification state: entry_id → { pending, ok, message }
  let rowVerify = $state<Record<string, { pending: boolean; ok?: boolean; message?: string }>>({});

  // Clipboard copy state: unique key → true while "copied" is shown
  let copiedId = $state<string | null>(null);

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
    rowVerify = {};
    try {
      const result = await getAuditLog({
        entity_type: filterEntity || undefined,
        action: filterAction || undefined,
        page,
        per_page: 50,
      });
      entries = result.items;
      total = result.total;
      totalPages = result.total_pages;
    } catch (e: any) {
      error = e.message;
      addNotification(e.message, 'error');
    } finally { loading = false; }
  }

  function trunc(val: string | null | undefined, len = 10): string {
    if (!val) return '—';
    return val.length > len ? val.slice(0, len) + '…' : val;
  }

  async function copyHash(val: string, key: string) {
    try {
      await navigator.clipboard.writeText(val);
      copiedId = key;
      setTimeout(() => { copiedId = null; }, 1500);
    } catch { /* clipboard unavailable */ }
  }

  async function verifyRow(entry: any) {
    rowVerify[entry.id] = { pending: true };
    try {
      const result = await verifyAuditEntry(entry.id);
      rowVerify[entry.id] = { pending: false, ok: result.ok, message: result.message };
    } catch (e: any) {
      rowVerify[entry.id] = { pending: false, ok: false, message: e.message };
    }
  }

  async function verifyLineage(entry: any) {
    if (!entry.lineage_id) return;
    rowVerify[entry.id] = { pending: true };
    try {
      const result = await verifyAuditLineage(entry.lineage_id);
      const msg = result.ok
        ? `Lineage chain OK — ${result.checked} entries verified.`
        : `Chain break at seq ${result.first_break_seq}: ${result.message}`;
      rowVerify[entry.id] = { pending: false, ok: result.ok, message: msg };
    } catch (e: any) {
      rowVerify[entry.id] = { pending: false, ok: false, message: e.message };
    }
  }

  // Counts of chained vs legacy rows on the current page
  let chainedCount = $derived(entries.filter(e => e.chain_seq != null).length);
  let legacyCount = $derived(entries.length - chainedCount);
</script>

<div>
  <div class="page-header">
    <h1>Audit Log ({total})</h1>
  </div>

  <!-- Chain integrity summary banner -->
  {#if !loading && entries.length > 0}
    <div class="chain-banner">
      {#if chainedCount > 0}
        <span class="chain-banner-icon">🔒</span>
        <span>
          <strong>{chainedCount}</strong> of {entries.length} visible entries are hash-chained.
          Click <strong>Verify</strong> on any row to check its integrity,
          or <strong>Verify Lineage</strong> to check the full chain for that entity.
        </span>
      {:else}
        <span class="chain-banner-icon chain-banner-icon--legacy">📋</span>
        <span>All visible entries are legacy (pre-v1.5.0) — no chain data to verify.</span>
      {/if}
      {#if legacyCount > 0 && chainedCount > 0}
        <span class="legacy-note">({legacyCount} legacy row{legacyCount !== 1 ? 's' : ''} hidden from count)</span>
      {/if}
    </div>
  {/if}

  <div class="card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <select title="Filter audit entries by entity type" bind:value={filterEntity} onchange={() => { page = 1; load(); }}>
          <option value="">All entities</option>
          <option value="specimen">Specimens</option>
          <option value="media_batch">Media</option>
          <option value="subculture">Subcultures</option>
          <option value="compliance">Compliance</option>
          <option value="user">Users</option>
          <option value="reminder">Reminders</option>
        </select>
      </div>
      <div>
        <select title="Filter by action" bind:value={filterAction} onchange={() => { page = 1; load(); }}>
          <option value="">All actions</option>
          <option value="create">Create</option>
          <option value="update">Update</option>
          <option value="delete">Delete</option>
          <option value="archive">Archive</option>
          <option value="login">Login</option>
        </select>
      </div>
      <div>
        <button title="Clear all filters" class="btn" onclick={() => { filterEntity = ''; filterAction = ''; page = 1; load(); }}>Reset</button>
      </div>
    </div>
  </div>

  <DataState
    {loading}
    {error}
    empty={entries.length === 0}
    rows={6}
    cols={9}
    emptyIcon="📋"
    emptyTitle="No audit entries found"
    emptyMessage="Audit events will appear here as users create, update, or delete records."
    onretry={load}
  >
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th title="Date and time the event was recorded">Timestamp</th>
            <th title="User who performed the action">User</th>
            <th title="Action type: create, update, delete, archive, login">Action</th>
            <th title="Entity type and short ID">Entity</th>
            <th title="Change description or new value">Details</th>
            <th class="chain-th" title="Per-lineage sequence number. 🔒 = chained entry; — = legacy row.">#</th>
            <th class="chain-th" title="SHA-256 of the previous entry in this lineage's chain. Click to copy full hash.">Prev Hash</th>
            <th class="chain-th" title="SHA-256 of this entry's content + prev_hash. Click to copy full hash.">Entry Hash</th>
            <th class="chain-th" title="Verify this entry's hash or its full lineage chain">Verify</th>
          </tr>
        </thead>
        <tbody>
          {#each entries as e}
            {@const chained = e.chain_seq != null}
            {@const rv = rowVerify[e.id]}
            <tr class={chained ? 'row-chained' : 'row-legacy'}>
              <td style="white-space:nowrap;">{e.created_at}</td>
              <td>{e.username || '—'}</td>
              <td><span class="badge badge-blue" title="Action: {e.action}">{e.action}</span></td>
              <td>{e.entity_type}{e.entity_id ? ` (${e.entity_id.slice(0, 8)}…)` : ''}</td>
              <td>{e.details || e.new_value || '—'}</td>

              <!-- Chain seq / lineage badge -->
              <td class="chain-cell">
                {#if chained}
                  <span class="seq-badge" title="Lineage: {e.lineage_id}  |  chain_seq: {e.chain_seq}">
                    🔒 {e.chain_seq}
                  </span>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>

              <!-- prev_hash -->
              <td class="hash-cell">
                {#if e.prev_hash}
                  {@const key = `prev-${e.id}`}
                  <button
                    class="hash-btn"
                    title="prev_hash — click to copy:\n{e.prev_hash}"
                    onclick={() => copyHash(e.prev_hash, key)}
                  ><code>{copiedId === key ? '✓ copied' : trunc(e.prev_hash)}</code></button>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>

              <!-- entry_hash -->
              <td class="hash-cell">
                {#if e.entry_hash}
                  {@const key = `entry-${e.id}`}
                  <button
                    class="hash-btn"
                    title="entry_hash — click to copy:\n{e.entry_hash}"
                    onclick={() => copyHash(e.entry_hash, key)}
                  ><code>{copiedId === key ? '✓ copied' : trunc(e.entry_hash)}</code></button>
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>

              <!-- Verify column -->
              <td class="verify-cell">
                {#if chained}
                  {#if rv?.pending}
                    <span class="verify-pending">…</span>
                  {:else if rv?.ok === true}
                    <span class="verify-ok" title={rv.message}>✓ OK</span>
                  {:else if rv?.ok === false}
                    <span class="verify-fail" title={rv.message}>✗ Fail</span>
                  {:else}
                    <div class="verify-btns">
                      <button class="btn btn-sm" title="Recompute and verify this single entry's hash" onclick={() => verifyRow(e)}>Row</button>
                      <button class="btn btn-sm" title="Verify the entire hash chain for lineage: {e.lineage_id}" onclick={() => verifyLineage(e)}>Chain</button>
                    </div>
                  {/if}
                {:else}
                  <span class="dim">—</span>
                {/if}
              </td>
            </tr>
            <!-- Inline verification message if present -->
            {#if rv && !rv.pending && rv.message}
              <tr class="verify-detail-row">
                <td colspan="9">
                  <span class={rv.ok ? 'verify-detail-ok' : 'verify-detail-fail'}>
                    {rv.ok ? '✓' : '✗'} {rv.message}
                  </span>
                  <button class="dismiss-btn" onclick={() => { const n = {...rowVerify}; delete n[e.id]; rowVerify = n; }}>×</button>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <div style="display:flex;align-items:center;justify-content:center;gap:16px;margin-top:16px;font-size:13px;">
        <button class="btn btn-sm" disabled={page <= 1} onclick={() => { page--; load(); }}>Prev</button>
        <span>Page {page} of {totalPages}</span>
        <button class="btn btn-sm" disabled={page >= totalPages} onclick={() => { page++; load(); }}>Next</button>
      </div>
    {/if}
  </DataState>
</div>

<style>
  .chain-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    margin-bottom: 14px;
    background: color-mix(in srgb, var(--color-success, #22c55e) 8%, var(--color-surface, #fff));
    border: 1px solid color-mix(in srgb, var(--color-success, #22c55e) 30%, transparent);
    border-radius: 6px;
    font-size: 13px;
    color: var(--color-text, #111);
  }

  .chain-banner-icon { font-size: 16px; }
  .chain-banner-icon--legacy { opacity: 0.5; }
  .legacy-note { color: var(--color-text-muted, #9ca3af); font-size: 12px; margin-left: 4px; }

  .chain-th {
    white-space: nowrap;
    font-size: 12px;
  }

  .row-chained {
    background-color: color-mix(in srgb, var(--color-success, #22c55e) 5%, transparent);
  }

  .seq-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-success, #16a34a);
    white-space: nowrap;
    cursor: help;
  }

  .dim { color: var(--color-text-muted, #9ca3af); font-size: 12px; }

  .chain-cell { white-space: nowrap; min-width: 72px; }
  .hash-cell  { min-width: 110px; }
  .verify-cell { min-width: 110px; white-space: nowrap; }

  .hash-btn {
    background: none;
    border: 1px solid var(--color-border, #e5e7eb);
    border-radius: 4px;
    padding: 2px 5px;
    cursor: pointer;
    font-size: 11px;
    color: var(--color-text-secondary, #6b7280);
    transition: background 0.1s, color 0.1s;
  }
  .hash-btn:hover { background: var(--color-surface-hover, #f3f4f6); color: var(--color-text, #111); }
  .hash-btn code { font-family: monospace; letter-spacing: 0.02em; }

  .verify-btns { display: flex; gap: 4px; }

  .verify-pending { font-size: 12px; color: var(--color-text-muted, #9ca3af); }

  .verify-ok {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-success, #16a34a);
    cursor: help;
  }
  .verify-fail {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-danger, #dc2626);
    cursor: help;
  }

  .verify-detail-row td {
    padding: 4px 12px 6px 20px;
    font-size: 12px;
    border-top: none;
    background: color-mix(in srgb, var(--color-surface, #fff) 80%, transparent);
  }

  .verify-detail-ok {
    color: var(--color-success, #16a34a);
    font-weight: 500;
  }
  .verify-detail-fail {
    color: var(--color-danger, #dc2626);
    font-weight: 500;
  }

  .dismiss-btn {
    margin-left: 10px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--color-text-muted, #9ca3af);
    font-size: 14px;
    padding: 0 4px;
    line-height: 1;
  }
  .dismiss-btn:hover { color: var(--color-text, #111); }
</style>
