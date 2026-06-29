<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listProvisionalTaxa,
    createProvisionalTaxon,
    listTaxonMappings,
    mapProvisionalTaxon,
    exportDarwinCore,
    type Taxon,
    type TaxonMapping,
    type DarwinCoreExport,
  } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import DataState from './DataState.svelte';

  const RANKS = ['kingdom', 'phylum', 'class', 'order', 'family', 'genus'] as const;

  const canManage = $derived(
    $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'
  );

  // Provisional taxa list
  let taxa = $state<Taxon[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Selected taxon
  let selectedTaxon = $state<Taxon | null>(null);
  let taxonMappings = $state<TaxonMapping[]>([]);
  let mappingsLoading = $state(false);

  // Create taxon form
  let showCreateForm = $state(false);
  let createForm = $state({
    rank: 'genus' as typeof RANKS[number],
    name: '',
    parent_id: '',
    provisional_notes: '',
  });
  let createSaving = $state(false);

  // Map taxon form
  let showMapForm = $state(false);
  let mapForm = $state({
    accepted_ncbi_id: '',
    accepted_name: '',
    notes: '',
  });
  let mapSaving = $state(false);

  // Darwin Core export
  let exportLoading = $state(false);
  let exportRootId = $state<string>('');

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
    try {
      [taxa, taxonMappings] = await Promise.all([
        listProvisionalTaxa(),
        listTaxonMappings(),
      ]);
    } catch (e: any) {
      error = e.message ?? 'Failed to load provisional taxa';
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    if (!createForm.name.trim()) return;
    createSaving = true;
    try {
      await createProvisionalTaxon({
        rank: createForm.rank,
        name: createForm.name.trim(),
        parent_id: createForm.parent_id.trim() || undefined,
        provisional_notes: createForm.provisional_notes.trim() || undefined,
      });
      addNotification('Provisional taxon created', 'success');
      showCreateForm = false;
      createForm = { rank: 'genus', name: '', parent_id: '', provisional_notes: '' };
      await load();
    } catch (e: any) {
      addNotification(e.message ?? 'Failed to create taxon', 'error');
    } finally {
      createSaving = false;
    }
  }

  async function handleMap() {
    if (!selectedTaxon || !mapForm.accepted_name.trim()) return;
    mapSaving = true;
    try {
      await mapProvisionalTaxon({
        provisional_taxon_id: selectedTaxon.id,
        accepted_ncbi_id: mapForm.accepted_ncbi_id ? parseInt(mapForm.accepted_ncbi_id, 10) : undefined,
        accepted_name: mapForm.accepted_name.trim() || undefined,
        notes: mapForm.notes.trim() || undefined,
      });
      addNotification('Taxon mapping saved', 'success');
      showMapForm = false;
      mapForm = { accepted_ncbi_id: '', accepted_name: '', notes: '' };
      await load();
    } catch (e: any) {
      addNotification(e.message ?? 'Failed to map taxon', 'error');
    } finally {
      mapSaving = false;
    }
  }

  async function handleExport() {
    exportLoading = true;
    try {
      const result: DarwinCoreExport = await exportDarwinCore(
        exportRootId.trim() || undefined
      );
      const json = JSON.stringify(result.records, null, 2);
      const blob = new Blob([json], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `darwin-core-export-${new Date().toISOString().split('T')[0]}.json`;
      a.click();
      URL.revokeObjectURL(url);
      addNotification(`Exported ${result.record_count} Darwin Core record(s)`, 'success');
    } catch (e: any) {
      addNotification(e.message ?? 'Export failed', 'error');
    } finally {
      exportLoading = false;
    }
  }

  function selectTaxon(t: Taxon) {
    selectedTaxon = t;
    showMapForm = false;
  }

  function getMappingsForTaxon(taxonId: string): TaxonMapping[] {
    return taxonMappings.filter(m => m.provisional_taxon_id === taxonId);
  }

  function rankLabel(rank: string): string {
    return rank.charAt(0).toUpperCase() + rank.slice(1);
  }
</script>

<div class="ptm-container">
  <div class="ptm-header">
    <div>
      <h1 class="ptm-title">Provisional Taxa</h1>
      <p class="ptm-subtitle">Manage lab-internal provisional taxa and map them to accepted NCBI records.</p>
    </div>
    {#if canManage}
      <button class="btn btn-primary" onclick={() => (showCreateForm = !showCreateForm)}>
        {showCreateForm ? 'Cancel' : '+ New Provisional Taxon'}
      </button>
    {/if}
  </div>

  {#if showCreateForm}
    <div class="ptm-form-card">
      <h2 class="ptm-form-title">New Provisional Taxon</h2>
      <div class="form-grid">
        <div class="form-group">
          <label for="ptm-rank">Rank *</label>
          <select id="ptm-rank" bind:value={createForm.rank}>
            {#each RANKS as r}
              <option value={r}>{rankLabel(r)}</option>
            {/each}
          </select>
        </div>
        <div class="form-group">
          <label for="ptm-name">Scientific Name *</label>
          <input id="ptm-name" type="text" bind:value={createForm.name}
            placeholder="e.g., Rosaria provisiona" />
        </div>
        <div class="form-group">
          <label for="ptm-parent">Parent Taxon ID</label>
          <input id="ptm-parent" type="text" bind:value={createForm.parent_id}
            placeholder="UUID of parent taxon (optional)" />
        </div>
        <div class="form-group span-2">
          <label for="ptm-notes">Provisional Notes</label>
          <textarea id="ptm-notes" bind:value={createForm.provisional_notes} rows="2"
            placeholder="Lab-internal notes, reasons for provisional status, etc."></textarea>
        </div>
      </div>
      <div class="form-actions">
        <button class="btn" onclick={() => (showCreateForm = false)}>Cancel</button>
        <button class="btn btn-primary" onclick={handleCreate}
          disabled={createSaving || !createForm.name.trim()}>
          {createSaving ? 'Creating…' : 'Create Provisional Taxon'}
        </button>
      </div>
    </div>
  {/if}

  <div class="ptm-layout">
    <!-- Taxon list -->
    <div class="ptm-list">
      <h2 class="ptm-section-title">Provisional Taxa ({taxa.length})</h2>
      <DataState {loading} {error} empty={!loading && taxa.length === 0}
        emptyTitle="No Provisional Taxa"
        emptyMessage="Create a provisional taxon to start managing lab-internal names."
        onretry={load}>
        {#each taxa as t (t.id)}
          <button
            class="ptm-taxon-row {selectedTaxon?.id === t.id ? 'selected' : ''}"
            onclick={() => selectTaxon(t)}
          >
            <span class="ptm-taxon-rank">{rankLabel(t.rank)}</span>
            <span class="ptm-taxon-name">{t.name}</span>
            {#if getMappingsForTaxon(t.id).length > 0}
              <span class="ptm-mapped-badge">Mapped</span>
            {:else}
              <span class="ptm-unmapped-badge">Unmapped</span>
            {/if}
          </button>
        {/each}
      </DataState>
    </div>

    <!-- Detail panel -->
    <div class="ptm-detail">
      {#if selectedTaxon}
        <div class="ptm-detail-header">
          <h2 class="ptm-detail-title">{selectedTaxon.name}</h2>
          <span class="ptm-rank-badge">{rankLabel(selectedTaxon.rank)}</span>
        </div>
        <div class="ptm-detail-meta">
          <span><strong>ID:</strong> <code>{selectedTaxon.id}</code></span>
          {#if selectedTaxon.parent_id}
            <span><strong>Parent ID:</strong> <code>{selectedTaxon.parent_id}</code></span>
          {/if}
          <span><strong>Created:</strong> {selectedTaxon.created_at.split('T')[0]}</span>
        </div>

        <!-- Mappings for this taxon -->
        <div class="ptm-mappings-section">
          <div class="ptm-mappings-header">
            <h3 class="ptm-section-title">Accepted Taxon Mappings</h3>
            {#if canManage}
              <button class="btn btn-sm btn-secondary"
                onclick={() => (showMapForm = !showMapForm)}>
                {showMapForm ? 'Cancel' : '+ Add Mapping'}
              </button>
            {/if}
          </div>

          {#if showMapForm}
            <div class="ptm-map-form">
              <div class="form-group">
                <label for="ptm-ncbi-id">NCBI Taxon ID</label>
                <input id="ptm-ncbi-id" type="number" bind:value={mapForm.accepted_ncbi_id}
                  placeholder="e.g., 1234567" />
              </div>
              <div class="form-group">
                <label for="ptm-acc-name">Accepted Name *</label>
                <input id="ptm-acc-name" type="text" bind:value={mapForm.accepted_name}
                  placeholder="e.g., Rosa damascena" />
              </div>
              <div class="form-group">
                <label for="ptm-map-notes">Notes</label>
                <input id="ptm-map-notes" type="text" bind:value={mapForm.notes}
                  placeholder="e.g., Pending publication" />
              </div>
              <div class="form-actions">
                <button class="btn" onclick={() => (showMapForm = false)}>Cancel</button>
                <button class="btn btn-primary btn-sm" onclick={handleMap}
                  disabled={mapSaving || !mapForm.accepted_name.trim()}>
                  {mapSaving ? 'Saving…' : 'Save Mapping'}
                </button>
              </div>
            </div>
          {/if}

          {#each getMappingsForTaxon(selectedTaxon.id) as m (m.id)}
            <div class="ptm-mapping-card">
              {#if m.accepted_name}
                <div class="ptm-mapping-name">{m.accepted_name}</div>
              {/if}
              {#if m.accepted_ncbi_id}
                <div class="ptm-mapping-meta">NCBI ID: {m.accepted_ncbi_id}</div>
              {/if}
              {#if m.notes}
                <div class="ptm-mapping-notes">{m.notes}</div>
              {/if}
              <div class="ptm-mapping-date">Mapped {m.mapped_at.split('T')[0]}</div>
            </div>
          {:else}
            {#if !showMapForm}
              <p class="ptm-empty-note">No mappings yet. Add a mapping to link this taxon to an accepted NCBI record.</p>
            {/if}
          {/each}
        </div>

      {:else}
        <div class="ptm-no-selection">
          <p>Select a provisional taxon from the list to view its details and mappings.</p>
        </div>
      {/if}
    </div>
  </div>

  <!-- Darwin Core Export section -->
  <div class="ptm-export-section">
    <h2 class="ptm-section-title">Darwin Core Export</h2>
    <p class="ptm-export-desc">
      Export your taxonomy as Darwin Core JSON for sharing with herbaria, museums, or regulatory bodies.
      Leave the Taxon ID field blank to export the full taxonomy.
    </p>
    <div class="ptm-export-controls">
      <div class="form-group ptm-export-input">
        <label for="ptm-export-root">Root Taxon ID (optional)</label>
        <input id="ptm-export-root" type="text" bind:value={exportRootId}
          placeholder="Leave blank for full taxonomy" />
      </div>
      <button class="btn btn-primary" onclick={handleExport} disabled={exportLoading}>
        {exportLoading ? 'Exporting…' : '⬇ Export Darwin Core JSON'}
      </button>
    </div>
  </div>
</div>

<style>
  .ptm-container {
    padding: 24px;
    max-width: 1200px;
    margin: 0 auto;
  }
  .ptm-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 24px;
    gap: 16px;
  }
  .ptm-title {
    font-size: 22px;
    font-weight: 700;
    color: #1e293b;
    margin: 0 0 4px;
  }
  .ptm-subtitle {
    font-size: 13px;
    color: #6b7280;
    margin: 0;
  }
  .ptm-form-card {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
    margin-bottom: 24px;
  }
  .ptm-form-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 16px;
    color: #1e293b;
  }
  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 12px;
    margin-bottom: 16px;
  }
  .span-2 { grid-column: span 2; }
  .form-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }
  .ptm-layout {
    display: grid;
    grid-template-columns: 300px 1fr;
    gap: 20px;
    margin-bottom: 24px;
  }
  .ptm-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .ptm-section-title {
    font-size: 13px;
    font-weight: 600;
    color: #6b7280;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin: 0 0 10px;
  }
  .ptm-taxon-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    cursor: pointer;
    width: 100%;
    text-align: left;
    font-size: 13px;
    transition: border-color 0.15s;
  }
  .ptm-taxon-row:hover { border-color: #93c5fd; }
  .ptm-taxon-row.selected { border-color: #2563eb; background: #eff6ff; }
  .ptm-taxon-rank {
    font-size: 10px;
    color: #6b7280;
    background: #f1f5f9;
    border-radius: 4px;
    padding: 2px 6px;
    flex-shrink: 0;
    text-transform: uppercase;
    font-weight: 600;
  }
  .ptm-taxon-name {
    flex: 1;
    font-weight: 500;
    color: #1e293b;
    font-style: italic;
  }
  .ptm-mapped-badge {
    font-size: 10px;
    background: #d1fae5;
    color: #065f46;
    border-radius: 10px;
    padding: 2px 8px;
    font-weight: 600;
  }
  .ptm-unmapped-badge {
    font-size: 10px;
    background: #fef3c7;
    color: #92400e;
    border-radius: 10px;
    padding: 2px 8px;
    font-weight: 600;
  }
  .ptm-detail {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
  }
  .ptm-detail-header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .ptm-detail-title {
    font-size: 18px;
    font-weight: 700;
    font-style: italic;
    color: #1e293b;
    margin: 0;
  }
  .ptm-rank-badge {
    font-size: 11px;
    background: #f1f5f9;
    color: #475569;
    border-radius: 4px;
    padding: 3px 8px;
    font-weight: 600;
    text-transform: uppercase;
  }
  .ptm-detail-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: #6b7280;
    margin-bottom: 20px;
    padding-bottom: 16px;
    border-bottom: 1px solid #f1f5f9;
  }
  .ptm-detail-meta code {
    font-size: 11px;
    background: #f8fafc;
    padding: 1px 4px;
    border-radius: 3px;
    color: #374151;
  }
  .ptm-mappings-section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ptm-mappings-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .ptm-map-form {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .ptm-mapping-card {
    background: #f0fdf4;
    border: 1px solid #86efac;
    border-radius: 6px;
    padding: 10px 12px;
  }
  .ptm-mapping-name {
    font-weight: 600;
    font-style: italic;
    font-size: 14px;
    color: #166534;
  }
  .ptm-mapping-meta {
    font-size: 11px;
    color: #6b7280;
    margin-top: 3px;
  }
  .ptm-mapping-notes {
    font-size: 12px;
    color: #374151;
    margin-top: 4px;
  }
  .ptm-mapping-date {
    font-size: 11px;
    color: #9ca3af;
    margin-top: 4px;
  }
  .ptm-empty-note {
    font-size: 12px;
    color: #9ca3af;
    font-style: italic;
    margin: 0;
  }
  .ptm-no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: #9ca3af;
    font-size: 13px;
    text-align: center;
  }
  .ptm-export-section {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 20px;
  }
  .ptm-export-desc {
    font-size: 13px;
    color: #6b7280;
    margin: 0 0 16px;
  }
  .ptm-export-controls {
    display: flex;
    align-items: flex-end;
    gap: 12px;
  }
  .ptm-export-input {
    flex: 1;
    max-width: 400px;
    margin-bottom: 0;
  }

  :global(.dark) .ptm-title { color: #f1f5f9; }
  :global(.dark) .ptm-form-card { background: #1e293b; border-color: #334155; }
  :global(.dark) .ptm-taxon-row { background: #1e293b; border-color: #334155; }
  :global(.dark) .ptm-taxon-row.selected { background: #1e3a5f; border-color: #3b82f6; }
  :global(.dark) .ptm-taxon-name { color: #e2e8f0; }
  :global(.dark) .ptm-detail { background: #1e293b; border-color: #334155; }
  :global(.dark) .ptm-detail-title { color: #f1f5f9; }
  :global(.dark) .ptm-export-section { background: #1e293b; border-color: #334155; }
</style>
