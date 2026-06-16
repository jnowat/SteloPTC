<script lang="ts">
  import { onMount } from 'svelte';
  import { getAuditLog } from '../api';
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
  let copiedId = $state<string | null>(null);

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
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

  async function copyHash(val: string, id: string) {
    try {
      await navigator.clipboard.writeText(val);
      copiedId = id;
      setTimeout(() => { copiedId = null; }, 1500);
    } catch {
      // clipboard not available — silent fail
    }
  }
</script>

<div>
  <div class="page-header">
    <h1>Audit Log ({total})</h1>
  </div>

  <div class="card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <select title="Filter audit entries by entity type (specimen, subculture, media_batch, etc.)" bind:value={filterEntity} onchange={() => { page = 1; load(); }}>
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
        <select title="Filter by action: create, update, delete, archive, login" bind:value={filterAction} onchange={() => { page = 1; load(); }}>
          <option value="">All actions</option>
          <option value="create">Create</option>
          <option value="update">Update</option>
          <option value="delete">Delete</option>
          <option value="archive">Archive</option>
          <option value="login">Login</option>
        </select>
      </div>
      <div>
        <button title="Clear all search filters and show all audit entries" class="btn" onclick={() => { filterEntity = ''; filterAction = ''; page = 1; load(); }}>Reset</button>
      </div>
    </div>
  </div>

  <DataState
    {loading}
    {error}
    empty={entries.length === 0}
    rows={6}
    cols={8}
    emptyIcon="📋"
    emptyTitle="No audit entries found"
    emptyMessage="Audit events will appear here as users create, update, or delete records."
    onretry={load}
  >
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th title="Date and time when the audit event was recorded">Timestamp</th>
            <th title="Username of the account that performed the action">User</th>
            <th title="The type of action performed: create, update, delete, archive, or login">Action</th>
            <th title="The type of entity affected and its short ID">Entity</th>
            <th title="Additional context about the change, or the new value after the action was applied">Details</th>
            <th title="Sequential position of this entry in the tamper-evident hash chain. Rows inserted before v1.5.0 show — here.">#</th>
            <th title="SHA-256 hash of the previous entry in the chain. Click to copy the full hash. Rows inserted before v1.5.0 show — here.">Prev Hash</th>
            <th title="SHA-256 hash of this entry's content combined with the previous hash — forms the chain link. Click to copy the full hash. Rows inserted before v1.5.0 show — here.">Entry Hash</th>
          </tr>
        </thead>
        <tbody>
          {#each entries as e}
            {@const chained = e.chain_seq != null}
            <tr class={chained ? 'row-chained' : 'row-legacy'}>
              <td style="white-space:nowrap;">{e.created_at}</td>
              <td>{e.username || '—'}</td>
              <td><span class="badge badge-blue" title="Action performed: {e.action}">{e.action}</span></td>
              <td>{e.entity_type}{e.entity_id ? ` (${e.entity_id.slice(0, 8)}...)` : ''}</td>
              <td>{e.details || e.new_value || '—'}</td>
              <td class="chain-cell">
                {#if chained}
                  <span class="chain-badge" title="Chain sequence #{e.chain_seq}">🔒 {e.chain_seq}</span>
                {:else}
                  <span class="legacy-badge" title="This row was written before the hash chain was introduced in v1.5.0">—</span>
                {/if}
              </td>
              <td class="hash-cell">
                {#if e.prev_hash}
                  {@const copyKey = `prev-${e.id}`}
                  <button
                    class="hash-btn"
                    title="prev_hash (click to copy full value):\n{e.prev_hash}"
                    onclick={() => copyHash(e.prev_hash, copyKey)}
                  >
                    <code>{copiedId === copyKey ? '✓ copied' : trunc(e.prev_hash)}</code>
                  </button>
                {:else}
                  <span class="legacy-badge">—</span>
                {/if}
              </td>
              <td class="hash-cell">
                {#if e.entry_hash}
                  {@const copyKey = `entry-${e.id}`}
                  <button
                    class="hash-btn"
                    title="entry_hash (click to copy full value):\n{e.entry_hash}"
                    onclick={() => copyHash(e.entry_hash, copyKey)}
                  >
                    <code>{copiedId === copyKey ? '✓ copied' : trunc(e.entry_hash)}</code>
                  </button>
                {:else}
                  <span class="legacy-badge">—</span>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if totalPages > 1}
      <div style="display:flex;align-items:center;justify-content:center;gap:16px;margin-top:16px;font-size:13px;">
        <button title="Go to the previous page of audit entries" class="btn btn-sm" disabled={page <= 1} onclick={() => { page--; load(); }}>Prev</button>
        <span title="Current page number out of total pages">Page {page} of {totalPages}</span>
        <button title="Go to the next page of audit entries" class="btn btn-sm" disabled={page >= totalPages} onclick={() => { page++; load(); }}>Next</button>
      </div>
    {/if}
  </DataState>
</div>

<style>
  .row-chained {
    background-color: color-mix(in srgb, var(--color-success, #22c55e) 5%, transparent);
  }

  .chain-badge {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-success, #16a34a);
    white-space: nowrap;
  }

  .legacy-badge {
    color: var(--color-text-muted, #9ca3af);
    font-size: 12px;
  }

  .chain-cell {
    white-space: nowrap;
    min-width: 72px;
  }

  .hash-cell {
    min-width: 110px;
  }

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

  .hash-btn:hover {
    background: var(--color-surface-hover, #f3f4f6);
    color: var(--color-text, #111827);
  }

  .hash-btn code {
    font-family: monospace;
    letter-spacing: 0.02em;
  }
</style>
