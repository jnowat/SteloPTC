<script lang="ts">
  import { onMount } from 'svelte';
  import { getAuditLog } from '../api';
  import { addNotification } from '../stores/app';

  let entries = $state<any[]>([]);
  let total = $state(0);
  let page = $state(1);
  let totalPages = $state(0);
  let loading = $state(true);
  let filterEntity = $state('');
  let filterAction = $state('');

  onMount(() => { load(); });

  async function load() {
    loading = true;
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
    } catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }
</script>

<div>
  <div class="page-header">
    <h1>Audit Log ({total})</h1>
  </div>

  <div class="card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <select bind:value={filterEntity} onchange={() => { page = 1; load(); }}>
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
        <select bind:value={filterAction} onchange={() => { page = 1; load(); }}>
          <option value="">All actions</option>
          <option value="create">Create</option>
          <option value="update">Update</option>
          <option value="delete">Delete</option>
          <option value="archive">Archive</option>
          <option value="login">Login</option>
        </select>
      </div>
      <div>
        <button class="btn" onclick={() => { filterEntity = ''; filterAction = ''; page = 1; load(); }}>Reset</button>
      </div>
    </div>
  </div>

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else if entries.length === 0}
    <div class="empty-state">No audit entries found</div>
  {:else}
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th>Timestamp</th>
            <th>User</th>
            <th>Action</th>
            <th>Entity</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {#each entries as e}
            <tr>
              <td style="white-space:nowrap;">{e.created_at}</td>
              <td>{e.username || '—'}</td>
              <td><span class="badge badge-blue">{e.action}</span></td>
              <td>{e.entity_type}{e.entity_id ? ` (${e.entity_id.slice(0, 8)}...)` : ''}</td>
              <td>{e.details || e.new_value || '—'}</td>
            </tr>
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
  {/if}
</div>
