<script lang="ts">
  import { onMount } from 'svelte';
  import { getWorkQueue } from '../api';
  import { navigateTo, selectedSpecimenId, workQueueCount, addNotification } from '../stores/app';

  interface WorkQueueItem {
    specimen_id: string;
    accession_number: string;
    species_name: string | null;
    location: string | null;
    reason: string;
    reason_code: string;
    urgency: string;
    days_overdue: number | null;
  }

  let items: WorkQueueItem[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  onMount(async () => {
    await load();
  });

  async function load() {
    loading = true;
    error = '';
    try {
      items = await getWorkQueue();
      workQueueCount.set(items.length);
    } catch (e: any) {
      error = e.message || 'Failed to load work queue';
      addNotification(error, 'error');
    } finally {
      loading = false;
    }
  }

  function openSpecimen(id: string) {
    selectedSpecimenId.set(id);
    navigateTo('specimen-detail', id);
  }

  function urgencyLabel(urgency: string): string {
    switch (urgency) {
      case 'critical': return 'Critical';
      case 'high':     return 'High';
      default:         return 'Normal';
    }
  }

  function urgencyBadgeClass(urgency: string): string {
    switch (urgency) {
      case 'critical': return 'badge-red';
      case 'high':     return 'badge-yellow';
      default:         return 'badge-blue';
    }
  }

  function reasonIcon(code: string): string {
    switch (code) {
      case 'quarantine':    return '&#128274;';
      case 'contamination': return '&#9762;';
      case 'no_passages':   return '&#128203;';
      case 'subculture_due':return '&#127793;';
      case 'media_expired': return '&#129514;';
      default:              return '&#9679;';
    }
  }

  const criticalCount = $derived(items.filter(i => i.urgency === 'critical').length);
  const highCount     = $derived(items.filter(i => i.urgency === 'high').length);
  const normalCount   = $derived(items.filter(i => i.urgency === 'normal').length);
</script>

<div class="page-header">
  <div>
    <h1>Work Queue</h1>
    <p class="subtitle">Specimens requiring attention, sorted by urgency</p>
  </div>
  <button class="btn" onclick={load} title="Refresh work queue">&#8635; Refresh</button>
</div>

{#if loading}
  <div class="empty-state"><div class="spinner"></div><p>Loading work queue…</p></div>
{:else if error}
  <div class="empty-state"><p style="color:#dc2626">{error}</p></div>
{:else if items.length === 0}
  <div class="empty-state">
    <div class="checkmark">&#10003;</div>
    <p>All clear — no specimens need attention right now.</p>
  </div>
{:else}
  <div class="summary-row">
    {#if criticalCount > 0}
      <span class="badge badge-red">{criticalCount} Critical</span>
    {/if}
    {#if highCount > 0}
      <span class="badge badge-yellow">{highCount} High</span>
    {/if}
    {#if normalCount > 0}
      <span class="badge badge-blue">{normalCount} Normal</span>
    {/if}
    <span class="total">{items.length} total item{items.length === 1 ? '' : 's'}</span>
  </div>

  <div class="card">
    <table>
      <thead>
        <tr>
          <th>Urgency</th>
          <th>Accession</th>
          <th>Species</th>
          <th>Location</th>
          <th>Issue</th>
        </tr>
      </thead>
      <tbody>
        {#each items as item (item.specimen_id + item.reason_code)}
          <tr class="queue-row urgency-{item.urgency}" onclick={() => openSpecimen(item.specimen_id)} title="Open specimen {item.accession_number}">
            <td>
              <span class="badge {urgencyBadgeClass(item.urgency)}">{urgencyLabel(item.urgency)}</span>
            </td>
            <td class="accession">{item.accession_number}</td>
            <td class="species">{item.species_name ?? '—'}</td>
            <td class="location">{item.location ?? '—'}</td>
            <td class="reason">
              <span class="reason-icon">{@html reasonIcon(item.reason_code)}</span>
              {item.reason}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <p class="read-only-note">Work queue is read-only. Click a row to open the specimen and take action.</p>
{/if}

<style>
  .subtitle {
    font-size: 13px;
    color: #6b7280;
    margin-top: 4px;
  }

  .summary-row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  .total {
    font-size: 13px;
    color: #6b7280;
    margin-left: 4px;
  }

  .queue-row {
    cursor: pointer;
    transition: background 0.1s;
  }
  .queue-row:hover td { background: #f0f9ff; }
  :global(.dark) .queue-row:hover td { background: #1e3a5f; }

  /* Left border accent by urgency */
  .queue-row.urgency-critical td:first-child { border-left: 3px solid #dc2626; }
  .queue-row.urgency-high     td:first-child { border-left: 3px solid #d97706; }
  .queue-row.urgency-normal   td:first-child { border-left: 3px solid #2563eb; }

  .accession {
    font-family: monospace;
    font-size: 12px;
    white-space: nowrap;
  }
  .species { font-style: italic; }
  .location { font-size: 12px; color: #6b7280; white-space: nowrap; }
  :global(.dark) .location { color: #94a3b8; }

  .reason {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .reason-icon { font-size: 14px; flex-shrink: 0; }

  .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e2e8f0;
    border-top-color: #2563eb;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 12px;
  }
  @keyframes spin { to { transform: rotate(360deg); } }

  .checkmark {
    font-size: 48px;
    color: #16a34a;
    margin-bottom: 8px;
  }

  .read-only-note {
    margin-top: 12px;
    font-size: 12px;
    color: #9ca3af;
    text-align: center;
  }
</style>
