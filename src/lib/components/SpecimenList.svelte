<script lang="ts">
  import { onMount } from 'svelte';
  import { listSpecimens, searchSpecimens, deleteSpecimen, listSpecies, exportSpecimensCsv, exportSpecimensJson } from '../api';
  import { navigateTo, addNotification } from '../stores/app';
  import { selectedSpecimenId } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import SpecimenForm from './SpecimenForm.svelte';

  let specimens = $state<any[]>([]);
  let species = $state<any[]>([]);
  let total = $state(0);
  let page = $state(1);
  let perPage = $state(50);
  let totalPages = $state(0);
  let loading = $state(true);
  let searchQuery = $state('');
  let filterSpecies = $state('');
  let filterStage = $state('');
  let showForm = $state(false);

  const stages = ['explant', 'callus', 'suspension', 'protoplast', 'shoot', 'root', 'embryogenic', 'plantlet', 'acclimatized', 'stock'];

  onMount(() => {
    load();
    loadSpecies();
  });

  async function loadSpecies() {
    try {
      species = await listSpecies();
    } catch (_e) {}
  }

  async function load() {
    loading = true;
    try {
      let result;
      if (searchQuery || filterSpecies || filterStage) {
        result = await searchSpecimens({
          query: searchQuery || undefined,
          species_id: filterSpecies || undefined,
          stage: filterStage || undefined,
          page,
          per_page: perPage,
        });
      } else {
        result = await listSpecimens(page, perPage);
      }
      specimens = result.items;
      total = result.total;
      totalPages = result.total_pages;
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function openDetail(id: string) {
    selectedSpecimenId.set(id);
    navigateTo('specimen-detail', id);
  }

  async function handleDelete(id: string) {
    if (!confirm('Archive this specimen? It can still be found in searches.')) return;
    try {
      await deleteSpecimen(id);
      addNotification('Specimen archived', 'success');
      load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  async function handleExport(format: 'csv' | 'json') {
    try {
      let data: string;
      if (format === 'csv') {
        data = await exportSpecimensCsv();
      } else {
        data = await exportSpecimensJson();
      }
      const blob = new Blob([data], { type: format === 'csv' ? 'text/csv' : 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `specimens_export.${format}`;
      a.click();
      URL.revokeObjectURL(url);
      addNotification(`Exported as ${format.toUpperCase()}`, 'success');
    } catch (e: any) {
      addNotification(e.message, 'error');
    }
  }

  function handleFormDone() {
    showForm = false;
    load();
  }

  function handleSearch() {
    page = 1;
    load();
  }
</script>

<div>
  <div class="page-header">
    <h1>Specimens ({total})</h1>
    <div style="display:flex;gap:8px;">
      <button class="btn btn-sm" onclick={() => handleExport('csv')}>Export CSV</button>
      <button class="btn btn-sm" onclick={() => handleExport('json')}>Export JSON</button>
      {#if $currentUser?.role !== 'guest'}
        <button class="btn btn-primary" onclick={() => showForm = true}>+ New Specimen</button>
      {/if}
    </div>
  </div>

  <div class="filters card" style="margin-bottom:16px;">
    <div class="form-row-3">
      <div>
        <input type="text" placeholder="Search accession, notes, location..." bind:value={searchQuery} onkeydown={(e) => e.key === 'Enter' && handleSearch()} />
      </div>
      <div>
        <select bind:value={filterSpecies} onchange={handleSearch}>
          <option value="">All species</option>
          {#each species as sp}
            <option value={sp.id}>{sp.species_code} - {sp.genus} {sp.species_name}</option>
          {/each}
        </select>
      </div>
      <div style="display:flex;gap:8px;">
        <select bind:value={filterStage} onchange={handleSearch}>
          <option value="">All stages</option>
          {#each stages as s}
            <option value={s}>{s}</option>
          {/each}
        </select>
        <button class="btn btn-sm" onclick={handleSearch}>Search</button>
      </div>
    </div>
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <SpecimenForm onclose={() => showForm = false} onsave={handleFormDone} />
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else if specimens.length === 0}
    <div class="empty-state">
      <p>No specimens found</p>
      <p style="margin-top:8px;font-size:12px;">Create your first specimen or adjust your filters.</p>
    </div>
  {:else}
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th>Accession</th>
            <th>Species</th>
            <th>Stage</th>
            <th>Location</th>
            <th>Passages</th>
            <th>Health</th>
            <th>Status</th>
            <th>Initiated</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each specimens as s}
            <tr class="clickable" onclick={() => openDetail(s.id)}>
              <td><strong>{s.accession_number}</strong></td>
              <td>{s.species_code || '—'}</td>
              <td><span class="badge badge-blue">{s.stage}</span></td>
              <td>{s.location || '—'}</td>
              <td>{s.subculture_count}</td>
              <td>{s.health_status || '—'}</td>
              <td>
                {#if s.quarantine_flag}
                  <span class="badge badge-red">Quarantine</span>
                {:else}
                  <span class="badge badge-green">Active</span>
                {/if}
              </td>
              <td>{s.initiation_date}</td>
              <td>
                {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                  <button class="btn btn-sm btn-danger" onclick={(e) => { e.stopPropagation(); handleDelete(s.id); }}>Archive</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if totalPages > 1}
      <div class="pagination">
        <button class="btn btn-sm" disabled={page <= 1} onclick={() => { page--; load(); }}>Prev</button>
        <span>Page {page} of {totalPages}</span>
        <button class="btn btn-sm" disabled={page >= totalPages} onclick={() => { page++; load(); }}>Next</button>
      </div>
    {/if}
  {/if}
</div>

<style>
  .clickable { cursor: pointer; }
  .clickable:hover td { background: #eff6ff !important; }
  :global(.dark) .clickable:hover td { background: #1e3a5f !important; }
  .pagination {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    margin-top: 16px;
    font-size: 13px;
  }
</style>
