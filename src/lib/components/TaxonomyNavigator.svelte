<script lang="ts">
  import { onMount } from 'svelte';
  import { listSpecies, listStrainsBySpecies, searchSpecimens } from '../api';
  import { addNotification, selectedStrainId } from '../stores/app';
  import StrainManager from './StrainManager.svelte';

  let allSpecies = $state<any[]>([]);
  let speciesSearch = $state('');
  let selectedSpeciesId = $state('');
  let selectedSpeciesObj = $state<any>(null);

  let strains = $state<any[]>([]);
  let strainsLoading = $state(false);
  let statusFilter = $state('all');

  // Slide-in panel for specimens bound to a strain
  let panelStrainId = $state<string | null>($selectedStrainId);
  let panelStrain = $state<any>(null);
  let panelSpecimens = $state<any[]>([]);
  let panelLoading = $state(false);
  let showManager = $state(false);

  let filteredSpecies = $derived(
    speciesSearch.trim()
      ? allSpecies.filter(sp =>
          `${sp.genus} ${sp.species_name} ${sp.species_code}`.toLowerCase().includes(speciesSearch.toLowerCase())
        )
      : allSpecies
  );

  let filteredStrains = $derived(() => {
    if (statusFilter === 'all') return strains;
    if (statusFilter === 'confirmed_any') return strains.filter(s => s.status === 'confirmed_manual' || s.status === 'confirmed_genomic');
    return strains.filter(s => s.status === statusFilter);
  });

  onMount(async () => {
    allSpecies = await listSpecies().catch(() => []);
    // If a strain was pre-selected (e.g. from SpecimenDetail), try to open its panel
    const preselect = $selectedStrainId;
    if (preselect) {
      await openStrainPanel(preselect, null);
      selectedStrainId.set(null);
    }
  });

  async function selectSpecies(sp: any) {
    selectedSpeciesId = sp.id;
    selectedSpeciesObj = sp;
    panelStrainId = null;
    panelStrain = null;
    panelSpecimens = [];
    showManager = false;
    strainsLoading = true;
    try {
      strains = await listStrainsBySpecies(sp.id);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      strainsLoading = false;
    }
  }

  async function openStrainPanel(strainId: string, strain: any) {
    panelStrainId = strainId;
    panelStrain = strain;
    panelLoading = true;
    panelSpecimens = [];
    try {
      const result = await searchSpecimens({ strain_id: strainId, page: 1, per_page: 200 });
      panelSpecimens = result.items ?? result ?? [];
    } catch {
      panelSpecimens = [];
    } finally {
      panelLoading = false;
    }
  }

  function closePanel() {
    panelStrainId = null;
    panelStrain = null;
    panelSpecimens = [];
  }

  function statusBadgeClass(status: string): string {
    switch (status) {
      case 'claimed': return 'status-claimed';
      case 'confirmed_manual': return 'status-confirmed_manual';
      case 'confirmed_genomic': return 'status-confirmed_genomic';
      default: return 'status-unverified';
    }
  }

  function statusLabel(status: string): string {
    switch (status) {
      case 'claimed': return 'Claimed';
      case 'confirmed_manual': return '⚠ Manual ID';
      case 'confirmed_genomic': return '✓ Genomic';
      default: return 'Unverified';
    }
  }

  function statusTooltip(status: string): string {
    switch (status) {
      case 'unverified': return 'No identity assertion has been made for this strain.';
      case 'claimed': return 'Identity asserted by lab staff but not independently verified.';
      case 'confirmed_manual': return 'Manually confirmed. Not equivalent to genomic verification.';
      case 'confirmed_genomic': return 'Genomic verification confirmed. Fingerprint data on record.';
      default: return '';
    }
  }
</script>

<div class="tx-nav">
  <!-- ── Left column: Species list ── -->
  <aside class="tx-left">
    <div class="tx-left-header">
      <h2>Taxonomy</h2>
      <input
        type="search"
        placeholder="Search species…"
        bind:value={speciesSearch}
        class="species-search"
        aria-label="Search species"
      />
    </div>
    <div class="species-list" role="list">
      {#each filteredSpecies as sp}
        <button
          class="species-row"
          class:selected={selectedSpeciesId === sp.id}
          onclick={() => selectSpecies(sp)}
          aria-pressed={selectedSpeciesId === sp.id}
          title="{sp.genus} {sp.species_name}"
        >
          <div class="species-info">
            <span class="species-code">{sp.species_code}</span>
            <span class="species-name">{sp.genus} {sp.species_name}</span>
          </div>
        </button>
      {:else}
        <div class="empty-state" style="padding:20px;font-size:13px;">
          {speciesSearch ? 'No species match your search.' : 'No species found.'}
        </div>
      {/each}
    </div>
  </aside>

  <!-- ── Right column: Strains for selected species ── -->
  <main class="tx-right">
    {#if !selectedSpeciesId}
      <div class="empty-state" style="padding:48px 24px;">
        <div style="font-size:32px;margin-bottom:12px;">&#127807;</div>
        <p style="font-size:14px;color:#6b7280;">Select a species from the left to browse its strains.</p>
      </div>
    {:else if showManager}
      <div class="tx-right-header">
        <button class="btn btn-sm" onclick={() => (showManager = false)}>← Strain Browser</button>
        <h3>{selectedSpeciesObj?.species_code} — {selectedSpeciesObj?.genus} {selectedSpeciesObj?.species_name}</h3>
      </div>
      <StrainManager
        speciesId={selectedSpeciesId}
        speciesName={`${selectedSpeciesObj?.genus ?? ''} ${selectedSpeciesObj?.species_name ?? ''}`.trim()}
      />
    {:else}
      <div class="tx-right-header">
        <div>
          <h3>{selectedSpeciesObj?.genus} {selectedSpeciesObj?.species_name} <span class="sp-code-chip">{selectedSpeciesObj?.species_code}</span></h3>
          <p class="tx-sub">{strains.length} strain{strains.length !== 1 ? 's' : ''} total</p>
        </div>
        <div style="display:flex;gap:8px;">
          <select bind:value={statusFilter} class="tx-filter" aria-label="Filter by status">
            <option value="all">All</option>
            <option value="unverified">Unverified</option>
            <option value="claimed">Claimed</option>
            <option value="confirmed_manual">Confirmed (Manual)</option>
            <option value="confirmed_genomic">Confirmed (Genomic)</option>
            <option value="confirmed_any">Confirmed (Any)</option>
          </select>
          <button class="btn btn-sm btn-primary" onclick={() => (showManager = true)}>Manage Strains</button>
        </div>
      </div>

      {#if strainsLoading}
        <div class="empty-state">Loading strains…</div>
      {:else if filteredStrains().length === 0}
        <div class="empty-state">
          {strains.length === 0
            ? 'No strains registered for this species.'
            : 'No strains match the current filter.'}
        </div>
      {:else}
        <div class="strain-grid">
          {#each filteredStrains() as s}
            <button
              class="strain-card"
              class:panel-open={panelStrainId === s.id}
              onclick={() => openStrainPanel(s.id, s)}
              title="View specimens for {s.name}"
            >
              <div class="sc-top">
                <code class="sc-code">{s.code}</code>
                <span class="status-badge {statusBadgeClass(s.status)}" title={statusTooltip(s.status)}>
                  {statusLabel(s.status)}
                </span>
              </div>
              <div class="sc-name">{s.name}</div>
              <div class="sc-meta">
                {#if s.is_hybrid}<span class="hybrid-chip">Hybrid</span>{/if}
                <span class="sc-count">{s.specimen_count ?? 0} specimen{(s.specimen_count ?? 0) !== 1 ? 's' : ''}</span>
              </div>
            </button>
          {/each}
        </div>
      {/if}
    {/if}
  </main>

  <!-- ── Slide-in panel: specimens for a strain ── -->
  {#if panelStrainId}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="panel-overlay" onclick={closePanel}></div>
    <aside class="tx-panel" aria-label="Specimens for strain">
      <div class="panel-header">
        {#if panelStrain}
          <div>
            <div style="display:flex;align-items:center;gap:8px;flex-wrap:wrap;">
              <code class="sc-code">{panelStrain.code}</code>
              <span class="status-badge {statusBadgeClass(panelStrain.status)}">{statusLabel(panelStrain.status)}</span>
            </div>
            <h4 class="panel-strain-name">{panelStrain.name}</h4>
          </div>
        {:else}
          <h4>Specimens</h4>
        {/if}
        <button class="panel-close" onclick={closePanel} aria-label="Close panel">&#10005;</button>
      </div>

      {#if panelLoading}
        <div class="empty-state" style="padding:32px;">Loading specimens…</div>
      {:else if panelSpecimens.length === 0}
        <div class="empty-state" style="padding:32px;">No specimens bound to this strain.</div>
      {:else}
        <div class="panel-specimens">
          {#each panelSpecimens as sp}
            <div class="panel-spec-row">
              <div>
                <div class="psr-accession">{sp.accession_number}</div>
                <div class="psr-meta">{sp.stage ?? '—'} · {sp.location ?? 'No location'}</div>
              </div>
              <span class="psr-health" title="Health status">
                {sp.health_status === '-1' || sp.health_status === null ? '?' :
                 sp.health_status === '4' ? '■■■■' :
                 sp.health_status === '3' ? '■■■□' :
                 sp.health_status === '2' ? '■■□□' :
                 sp.health_status === '1' ? '■□□□' : '□□□□'}
              </span>
            </div>
          {/each}
        </div>
      {/if}
    </aside>
  {/if}
</div>

<style>
  .tx-nav {
    display: flex;
    height: calc(100vh - 48px - 48px);
    min-height: 400px;
    position: relative;
    overflow: hidden;
    gap: 0;
  }

  /* Left column */
  .tx-left {
    width: 260px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid #e2e8f0;
    overflow: hidden;
  }
  .tx-left-header {
    padding: 16px;
    border-bottom: 1px solid #e2e8f0;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .tx-left-header h2 { font-size: 16px; font-weight: 700; }
  .species-search { font-size: 13px; }
  .species-list {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }
  .species-row {
    display: flex;
    align-items: center;
    width: 100%;
    padding: 10px 12px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 6px;
    text-align: left;
    transition: background 0.12s;
  }
  .species-row:hover { background: #f1f5f9; }
  .species-row.selected { background: #dbeafe; }
  .species-info { display: flex; flex-direction: column; gap: 2px; }
  .species-code { font-size: 11px; font-weight: 700; color: #2563eb; }
  .species-name { font-size: 12px; color: #374151; font-style: italic; }

  /* Right column */
  .tx-right {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .tx-right-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    flex-wrap: wrap;
  }
  .tx-right-header h3 { font-size: 16px; font-weight: 700; }
  .tx-sub { font-size: 12px; color: #6b7280; margin-top: 2px; }
  .sp-code-chip {
    font-size: 11px;
    background: #dbeafe;
    color: #1e40af;
    border-radius: 10px;
    padding: 1px 7px;
    font-weight: 700;
    font-style: normal;
  }
  .tx-filter { width: 180px; font-size: 12px; }

  /* Strain grid */
  .strain-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 12px;
  }
  .strain-card {
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 14px;
    cursor: pointer;
    text-align: left;
    transition: all 0.15s;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }
  .strain-card:hover { border-color: #93c5fd; box-shadow: 0 2px 8px rgba(37,99,235,0.1); }
  .strain-card.panel-open { border-color: #2563eb; background: #eff6ff; }
  .sc-top { display: flex; justify-content: space-between; align-items: center; gap: 6px; }
  .sc-code { font-size: 12px; font-weight: 700; background: #f1f5f9; padding: 2px 5px; border-radius: 3px; font-family: monospace; }
  .sc-name { font-size: 13px; font-weight: 500; color: #1e293b; }
  .sc-meta { display: flex; gap: 8px; align-items: center; }
  .sc-count { font-size: 11px; color: #6b7280; }
  .hybrid-chip { font-size: 10px; background: #ede9fe; color: #6d28d9; border-radius: 10px; padding: 1px 6px; }

  /* Status badges */
  .status-badge {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 10px;
    font-size: 10px;
    font-weight: 600;
  }
  .status-unverified { background: #f1f5f9; color: #475569; }
  .status-claimed { background: #dbeafe; color: #1e40af; }
  .status-confirmed_manual { background: #fef3c7; color: #92400e; }
  .status-confirmed_genomic { background: #dcfce7; color: #166534; }

  /* Slide-in panel */
  .panel-overlay {
    position: absolute;
    inset: 0;
    z-index: 10;
    background: rgba(0,0,0,0.15);
  }
  .tx-panel {
    position: absolute;
    top: 0;
    right: 0;
    bottom: 0;
    width: 320px;
    background: white;
    border-left: 1px solid #e2e8f0;
    z-index: 20;
    display: flex;
    flex-direction: column;
    box-shadow: -4px 0 24px rgba(0,0,0,0.12);
    animation: slideIn 0.2s ease;
  }
  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }
  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px;
    border-bottom: 1px solid #e2e8f0;
  }
  .panel-strain-name { font-size: 14px; font-weight: 600; margin-top: 4px; }
  .panel-close {
    background: none;
    border: none;
    font-size: 16px;
    cursor: pointer;
    color: #6b7280;
    padding: 2px 6px;
    border-radius: 4px;
    min-height: 0;
  }
  .panel-close:hover { background: #f3f4f6; }
  .panel-specimens { flex: 1; overflow-y: auto; padding: 8px; }
  .panel-spec-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-radius: 6px;
    margin-bottom: 4px;
    border: 1px solid #f1f5f9;
  }
  .panel-spec-row:hover { background: #f8fafc; }
  .psr-accession { font-size: 13px; font-weight: 600; }
  .psr-meta { font-size: 11px; color: #6b7280; margin-top: 2px; }
  .psr-health { font-size: 10px; color: #6b7280; font-family: monospace; letter-spacing: -2px; }

  /* Dark mode */
  :global(.dark) .tx-left { border-right-color: #334155; }
  :global(.dark) .tx-left-header { border-bottom-color: #334155; }
  :global(.dark) .species-row:hover { background: #1e293b; }
  :global(.dark) .species-row.selected { background: #1e3a5f; }
  :global(.dark) .species-name { color: #cbd5e1; }
  :global(.dark) .strain-card { background: #1e293b; border-color: #334155; }
  :global(.dark) .strain-card:hover { border-color: #60a5fa; }
  :global(.dark) .strain-card.panel-open { background: #1e3a5f; border-color: #2563eb; }
  :global(.dark) .sc-code { background: #334155; color: #e2e8f0; }
  :global(.dark) .sc-name { color: #e2e8f0; }
  :global(.dark) .tx-panel { background: #1e293b; border-left-color: #334155; }
  :global(.dark) .panel-header { border-bottom-color: #334155; }
  :global(.dark) .panel-spec-row { border-color: #334155; }
  :global(.dark) .panel-spec-row:hover { background: #0f172a; }
  :global(.dark) .panel-close:hover { background: #334155; }
  :global(.dark) .status-unverified { background: #334155; color: #94a3b8; }
  :global(.dark) .status-confirmed_manual { background: #78350f; color: #fde68a; }
  :global(.dark) .status-confirmed_genomic { background: #166534; color: #dcfce7; }
  :global(.dark) .status-claimed { background: #1e40af; color: #dbeafe; }

  @media (max-width: 640px) {
    .tx-left { width: 200px; }
    .tx-panel { width: min(300px, 85vw); }
  }
</style>
