<script lang="ts">
  import { onMount } from 'svelte';
  import { listAllFruitingRecords, type FruitingRecordWithSpecimen } from '../api';
  import { navigateTo, selectedSpecimenId, addNotification } from '../stores/app';

  let records: FruitingRecordWithSpecimen[] = $state([]);
  let loading = $state(true);
  let error = $state('');

  onMount(load);

  async function load() {
    loading = true;
    error = '';
    try {
      records = await listAllFruitingRecords();
    } catch (e: any) {
      error = e.message || 'Failed to load fruiting records';
      addNotification(error, 'error');
    } finally {
      loading = false;
    }
  }

  function openSpecimen(id: string) {
    selectedSpecimenId.set(id);
    navigateTo('specimen-detail', id);
  }

  function num(v: number | null, digits = 1): string {
    return v == null ? '—' : v.toFixed(digits);
  }

  // Totals for the summary row. Biological efficiency (BE) = dry-yield-relative
  // to substrate isn't tracked here, so we surface the two figures a grower
  // actually logs: total flushes and total fresh harvest weight.
  const totalFresh = $derived(
    records.reduce((sum, r) => sum + (r.fresh_weight_g ?? 0), 0)
  );
  const specimenCount = $derived(new Set(records.map((r) => r.specimen_id)).size);
</script>

<div class="page-header">
  <div>
    <h1>Fruiting</h1>
    <p class="subtitle">Flush yields and harvest conditions across all mycology specimens</p>
  </div>
  <button class="btn" onclick={load} title="Refresh fruiting records">&#8635; Refresh</button>
</div>

{#if loading}
  <div class="empty-state"><div class="spinner"></div><p>Loading fruiting records…</p></div>
{:else if error}
  <div class="empty-state"><p style="color:#dc2626">{error}</p></div>
{:else if records.length === 0}
  <div class="empty-state">
    <p>No fruiting records yet. Record a flush from a specimen's detail page to see it here.</p>
  </div>
{:else}
  <div class="summary-row">
    <span class="badge badge-blue">{records.length} flush{records.length === 1 ? '' : 'es'}</span>
    <span class="badge badge-green">{totalFresh.toFixed(1)} g fresh total</span>
    <span class="total">across {specimenCount} specimen{specimenCount === 1 ? '' : 's'}</span>
  </div>

  <div class="card">
    <table>
      <thead>
        <tr>
          <th>Harvest Date</th>
          <th>Accession</th>
          <th>Species</th>
          <th class="num">Flush</th>
          <th class="num">Fresh (g)</th>
          <th class="num">Dry (g)</th>
          <th class="num">Temp (°C)</th>
          <th class="num">RH (%)</th>
          <th>Notes</th>
        </tr>
      </thead>
      <tbody>
        {#each records as r (r.id)}
          <tr onclick={() => openSpecimen(r.specimen_id)} title="Open specimen {r.specimen_accession}">
            <td>{r.harvest_date}</td>
            <td class="accession">{r.specimen_accession}</td>
            <td class="species">{r.species_label || '—'}</td>
            <td class="num">{r.flush_number}</td>
            <td class="num">{num(r.fresh_weight_g)}</td>
            <td class="num">{num(r.dry_weight_g)}</td>
            <td class="num">{num(r.fruiting_temp_c)}</td>
            <td class="num">{num(r.fruiting_rh_percent, 0)}</td>
            <td class="notes">{r.notes ?? ''}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <p class="read-only-note">Fruiting overview is read-only. Click a row to open the specimen and record or edit a flush.</p>
{/if}

<style>
  .subtitle {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
    margin-top: 2px;
  }
  .summary-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
    flex-wrap: wrap;
  }
  .summary-row .total {
    color: var(--color-text-muted);
    font-size: var(--font-size-sm);
  }
  tbody tr {
    cursor: pointer;
  }
  tbody tr:hover {
    background: var(--color-sidebar-hover);
  }
  .accession {
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .notes {
    color: var(--color-text-muted);
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .read-only-note {
    margin-top: var(--space-3);
    color: var(--color-text-muted);
    font-size: var(--font-size-xs);
  }
</style>
