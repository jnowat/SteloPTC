<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getTaxonColumn, listSpeciesForTaxon, listStrainsBySpecies,
    searchSpecimens, searchTaxonomy, getStrain,
    type TaxonColumnItem, type SpeciesNodeSummary, type TaxonomySearchResult,
  } from '../api';
  import { addNotification, navigateTo, selectedSpecimenId, selectedStrainId } from '../stores/app';
  import StrainManager from './StrainManager.svelte';
  import StrainDetail from './StrainDetail.svelte';

  // ── Types ──────────────────────────────────────────────────────────────────

  interface ColumnItem {
    id: string;
    primaryLabel: string;
    secondaryLabel: string;
    strainCount: number;
    specimenCount: number;
    rank?: string;
    statusKey?: string;
    isHybrid?: boolean;
    isCrossSpecies?: boolean;
  }

  interface Column {
    kind: 'taxon' | 'species' | 'strain';
    parentId: string | null;
    rank?: string;
    items: ColumnItem[];
    loading: boolean;
    selectedId: string | null;
  }

  // ── State ──────────────────────────────────────────────────────────────────

  let columns = $state<Column[]>([]);

  // Strain quick-action panel
  let panelStrainId = $state<string | null>(null);
  let panelStrain = $state<any | null>(null);
  let panelSpecimens = $state<any[]>([]);
  let panelLoading = $state(false);
  let showStrainDetail = $state(false);
  let detailStrainId = $state<string | null>(null);

  // StrainManager overlay
  let showManager = $state(false);
  let managerSpeciesId = $state<string | null>(null);
  let managerSpeciesName = $state('');

  // Filters
  let showFilters = $state(false);
  let filterStatus = $state('all');

  // Global search
  let searchQuery = $state('');
  let searchResults = $state<TaxonomySearchResult[]>([]);
  let showSearchResults = $state(false);
  let searchDebounce: ReturnType<typeof setTimeout> | null = null;
  let searchInputEl: HTMLInputElement;

  // Keyboard focus tracking
  let focusedCol = $state(0);
  let focusedRow = $state(0);
  let columnsContainerEl: HTMLDivElement;

  // Raw strain data for panel display
  let rawStrainData = new Map<string, any>();

  // ── Derived ────────────────────────────────────────────────────────────────

  let selectedPath = $derived(
    columns
      .filter(c => c.selectedId !== null)
      .map(c => {
        const item = c.items.find(i => i.id === c.selectedId);
        return { id: c.selectedId as string, label: item?.primaryLabel ?? '', kind: c.kind, rank: c.rank };
      })
  );

  // For the strains column, apply the status filter
  function getDisplayItems(col: Column): ColumnItem[] {
    if (col.kind !== 'strain') return col.items;
    if (filterStatus === 'all') return col.items;
    if (filterStatus === 'confirmed_any') {
      return col.items.filter(i => i.statusKey === 'confirmed_manual' || i.statusKey === 'confirmed_genomic');
    }
    return col.items.filter(i => i.statusKey === filterStatus);
  }

  // ── Converters ─────────────────────────────────────────────────────────────

  function taxonToItem(t: TaxonColumnItem): ColumnItem {
    return {
      id: t.id,
      primaryLabel: t.name,
      secondaryLabel: t.rank,
      strainCount: t.strain_count,
      specimenCount: t.specimen_count,
      rank: t.rank,
    };
  }

  function speciesToItem(sp: SpeciesNodeSummary): ColumnItem {
    return {
      id: sp.id,
      primaryLabel: `${sp.genus} ${sp.species_name}`,
      secondaryLabel: sp.species_code,
      strainCount: sp.strain_count,
      specimenCount: sp.specimen_count,
    };
  }

  function strainToItem(s: any): ColumnItem {
    return {
      id: s.id,
      primaryLabel: s.name,
      secondaryLabel: s.code,
      strainCount: 0,
      specimenCount: s.specimen_count ?? 0,
      statusKey: s.status,
      isHybrid: !!s.is_hybrid,
      isCrossSpecies: !!s.is_cross_species,
    };
  }

  // ── Column loading ─────────────────────────────────────────────────────────

  async function loadKingdomColumn() {
    columns = [{ kind: 'taxon', parentId: null, rank: 'kingdom', items: [], loading: true, selectedId: null }];
    try {
      const taxa = await getTaxonColumn(undefined);
      columns[0] = { ...columns[0], items: taxa.map(taxonToItem), loading: false };
      columns = [...columns];
    } catch (e: any) {
      addNotification(`Failed to load kingdoms: ${e.message}`, 'error');
      columns[0] = { ...columns[0], loading: false };
      columns = [...columns];
    }
  }

  async function appendTaxonColumn(parentId: string) {
    columns = [...columns, { kind: 'taxon', parentId, items: [], loading: true, selectedId: null }];
    const idx = columns.length - 1;
    try {
      const taxa = await getTaxonColumn(parentId);
      columns[idx] = { ...columns[idx], items: taxa.map(taxonToItem), loading: false };
      columns = [...columns];
    } catch (e: any) {
      addNotification(`Failed to load taxa: ${e.message}`, 'error');
      columns[idx] = { ...columns[idx], loading: false };
      columns = [...columns];
    }
  }

  async function appendSpeciesColumn(genusId: string) {
    columns = [...columns, { kind: 'species', parentId: genusId, items: [], loading: true, selectedId: null }];
    const idx = columns.length - 1;
    try {
      const species = await listSpeciesForTaxon(genusId);
      columns[idx] = { ...columns[idx], items: species.map(speciesToItem), loading: false };
      columns = [...columns];
    } catch (e: any) {
      addNotification(`Failed to load species: ${e.message}`, 'error');
      columns[idx] = { ...columns[idx], loading: false };
      columns = [...columns];
    }
  }

  async function appendStrainsColumn(speciesId: string) {
    columns = [...columns, { kind: 'strain', parentId: speciesId, items: [], loading: true, selectedId: null }];
    const idx = columns.length - 1;
    try {
      const strains: any[] = await listStrainsBySpecies(speciesId);
      rawStrainData = new Map(strains.map(s => [s.id, s]));
      columns[idx] = { ...columns[idx], items: strains.map(strainToItem), loading: false };
      columns = [...columns];
    } catch (e: any) {
      addNotification(`Failed to load strains: ${e.message}`, 'error');
      columns[idx] = { ...columns[idx], loading: false };
      columns = [...columns];
    }
  }

  // ── Selection & navigation ─────────────────────────────────────────────────

  async function selectItem(colIndex: number, item: ColumnItem) {
    // Mark selected, truncate subsequent columns, close panel
    const newCols = columns.slice(0, colIndex + 1).map((c, i) =>
      i === colIndex ? { ...c, selectedId: item.id } : c
    );
    columns = newCols;
    panelStrainId = null;
    panelStrain = null;
    panelSpecimens = [];

    const col = columns[colIndex];

    if (col.kind === 'taxon') {
      if (item.rank === 'genus') {
        await appendSpeciesColumn(item.id);
      } else {
        await appendTaxonColumn(item.id);
      }
    } else if (col.kind === 'species') {
      await appendStrainsColumn(item.id);
    } else if (col.kind === 'strain') {
      await openStrainPanel(item.id);
    }

    savePath();
    scrollToLastColumn();
  }

  async function openStrainPanel(strainId: string) {
    panelStrainId = strainId;
    panelStrain = rawStrainData.get(strainId) ?? null;
    panelLoading = true;
    panelSpecimens = [];

    // Mark strain selected in its column
    const strainColIdx = columns.findIndex(c => c.kind === 'strain');
    if (strainColIdx >= 0) {
      columns[strainColIdx] = { ...columns[strainColIdx], selectedId: strainId };
      columns = [...columns];
    }

    try {
      const [specsResult, strainData] = await Promise.all([
        searchSpecimens({ strain_id: strainId, page: 1, per_page: 200 }),
        panelStrain ? Promise.resolve(null) : getStrain(strainId).catch(() => null),
      ]);
      panelSpecimens = specsResult?.items ?? specsResult ?? [];
      if (strainData) panelStrain = strainData;
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
    // Deselect in strains column
    const idx = columns.findIndex(c => c.kind === 'strain');
    if (idx >= 0) {
      columns[idx] = { ...columns[idx], selectedId: null };
      columns = [...columns];
    }
  }

  function navigateToBreadcrumb(pathIndex: number) {
    // Keep path nodes 0..pathIndex, show the column right after them
    // e.g. clicking node 1 (phylum) keeps: kingdoms col (sel), phyla col (sel), classes col (unsel)
    const newCols = columns.slice(0, pathIndex + 2).map((c, i) =>
      i === pathIndex + 1 ? { ...c, selectedId: null } : c
    );
    columns = newCols;
    panelStrainId = null;
    panelStrain = null;
    panelSpecimens = [];
  }

  function resetToRoot() {
    if (columns.length > 0) {
      columns = [{ ...columns[0], selectedId: null }];
    }
    panelStrainId = null;
    panelStrain = null;
    panelSpecimens = [];
  }

  // ── Search ─────────────────────────────────────────────────────────────────

  function handleSearchInput() {
    if (searchDebounce) clearTimeout(searchDebounce);
    const q = searchQuery.trim();
    if (q.length < 2) {
      searchResults = [];
      showSearchResults = false;
      return;
    }
    searchDebounce = setTimeout(async () => {
      try {
        const results = await searchTaxonomy(q);
        searchResults = results;
        showSearchResults = results.length > 0;
      } catch {
        searchResults = [];
      }
    }, 300);
  }

  async function navigateToSearchResult(result: TaxonomySearchResult) {
    showSearchResults = false;
    searchQuery = '';

    if (result.result_type === 'specimen') {
      selectedSpecimenId.set(result.id);
      navigateTo('specimen-detail');
      return;
    }

    // Reset to just kingdoms
    if (columns.length === 0 || columns[0].items.length === 0) {
      await loadKingdomColumn();
    } else {
      columns = [{ ...columns[0], selectedId: null }];
      panelStrainId = null;
    }

    // Walk through the taxon path
    for (let i = 0; i < result.taxon_ids.length; i++) {
      const col = columns[i];
      if (!col) break;
      const item = col.items.find(it => it.id === result.taxon_ids[i]);
      if (!item) break;
      await selectItem(i, item);
    }

    // Navigate to species if provided
    if (result.species_id && result.result_type !== 'taxon') {
      const spCol = columns.find(c => c.kind === 'species');
      if (spCol) {
        const idx = columns.indexOf(spCol);
        const item = spCol.items.find(it => it.id === result.species_id);
        if (item) await selectItem(idx, item);
      }
    }

    // Open strain panel if provided
    if (result.strain_id && (result.result_type === 'strain')) {
      const stCol = columns.find(c => c.kind === 'strain');
      if (stCol) {
        const item = stCol.items.find(it => it.id === result.strain_id);
        if (item) await openStrainPanel(result.strain_id);
      }
    }
  }

  let groupedResults = $derived.by(() => {
    const taxa = searchResults.filter(r => r.result_type === 'taxon');
    const species = searchResults.filter(r => r.result_type === 'species');
    const strains = searchResults.filter(r => r.result_type === 'strain');
    const specimens = searchResults.filter(r => r.result_type === 'specimen');
    return { taxa, species, strains, specimens };
  });

  // ── Keyboard navigation ────────────────────────────────────────────────────

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === '/' && !showSearchResults) {
      e.preventDefault();
      searchInputEl?.focus();
      return;
    }
    const col = columns[focusedCol];
    if (!col) return;
    const displayItems = getDisplayItems(col);

    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        focusedRow = Math.min(focusedRow + 1, displayItems.length - 1);
        scrollFocusedIntoView();
        break;
      case 'ArrowUp':
        e.preventDefault();
        focusedRow = Math.max(focusedRow - 1, 0);
        scrollFocusedIntoView();
        break;
      case 'ArrowRight':
      case 'Enter':
        if (showSearchResults) break;
        e.preventDefault();
        if (displayItems[focusedRow]) {
          selectItem(focusedCol, displayItems[focusedRow]).then(() => {
            focusedCol = columns.length - 1;
            focusedRow = 0;
          });
        }
        break;
      case 'ArrowLeft':
        e.preventDefault();
        if (focusedCol > 0) {
          focusedCol -= 1;
          const prevCol = columns[focusedCol];
          focusedRow = Math.max(0, prevCol.items.findIndex(i => i.id === prevCol.selectedId));
        }
        break;
      case 'Escape':
        e.preventDefault();
        if (panelStrainId) {
          closePanel();
        } else if (focusedCol > 0) {
          focusedCol -= 1;
          const prevCol = columns[focusedCol];
          focusedRow = Math.max(0, prevCol.items.findIndex(i => i.id === prevCol.selectedId));
        } else {
          showSearchResults = false;
        }
        break;
    }
  }

  function scrollFocusedIntoView() {
    setTimeout(() => {
      const el = document.getElementById(`col-${focusedCol}-row-${focusedRow}`);
      el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }, 0);
  }

  // ── State persistence ──────────────────────────────────────────────────────

  function savePath() {
    try {
      const taxonIds = columns
        .filter(c => c.kind === 'taxon' && c.selectedId)
        .map(c => c.selectedId!);
      const spCol = columns.find(c => c.kind === 'species');
      const stCol = columns.find(c => c.kind === 'strain');
      localStorage.setItem('stelo_taxonomy_path', JSON.stringify({
        taxonIds,
        speciesId: spCol?.selectedId ?? null,
        strainId: stCol?.selectedId ?? panelStrainId ?? null,
      }));
    } catch {
      // localStorage may be unavailable
    }
  }

  async function restoreSavedPath() {
    try {
      const raw = localStorage.getItem('stelo_taxonomy_path');
      if (!raw) return;
      const { taxonIds, speciesId, strainId } = JSON.parse(raw) as {
        taxonIds: string[];
        speciesId: string | null;
        strainId: string | null;
      };

      for (let i = 0; i < taxonIds.length; i++) {
        const col = columns[i];
        if (!col) break;
        const item = col.items.find(it => it.id === taxonIds[i]);
        if (!item) break;
        await selectItem(i, item);
      }

      if (speciesId) {
        const spCol = columns.find(c => c.kind === 'species');
        if (spCol) {
          const idx = columns.indexOf(spCol);
          const item = spCol.items.find(it => it.id === speciesId);
          if (item) await selectItem(idx, item);
        }
      }

      if (strainId) {
        const stCol = columns.find(c => c.kind === 'strain');
        if (stCol) {
          const item = stCol.items.find(it => it.id === strainId);
          if (item) await openStrainPanel(strainId);
        }
      }
    } catch {
      // Ignore — corrupted or stale saved path
    }
  }

  // ── Utility ────────────────────────────────────────────────────────────────

  function scrollToLastColumn() {
    setTimeout(() => {
      columnsContainerEl?.scrollTo({ left: columnsContainerEl.scrollWidth, behavior: 'smooth' });
    }, 50);
  }

  function colHeaderLabel(col: Column): string {
    if (col.kind === 'species') return 'Species';
    if (col.kind === 'strain') return 'Strains';
    const r = col.rank ?? '';
    return r.charAt(0).toUpperCase() + r.slice(1);
  }

  function statusBadgeClass(key: string): string {
    switch (key) {
      case 'claimed': return 'st-claimed';
      case 'confirmed_manual': return 'st-manual';
      case 'confirmed_genomic': return 'st-genomic';
      default: return 'st-unverified';
    }
  }

  function statusLabel(key: string): string {
    switch (key) {
      case 'claimed': return 'Claimed';
      case 'confirmed_manual': return '⚠ Manual';
      case 'confirmed_genomic': return '✓ Genomic';
      default: return 'Unverified';
    }
  }

  function healthBar(val: string | null): string {
    switch (val) {
      case '4': return '■■■■';
      case '3': return '■■■□';
      case '2': return '■■□□';
      case '1': return '■□□□';
      case '0': return '□□□□';
      default: return '?';
    }
  }

  function countLabel(item: ColumnItem): string {
    if (item.strainCount > 0 && item.specimenCount > 0) {
      return `${item.strainCount} str · ${item.specimenCount} sp`;
    }
    if (item.strainCount > 0) return `${item.strainCount} strain${item.strainCount !== 1 ? 's' : ''}`;
    if (item.specimenCount > 0) return `${item.specimenCount} specimen${item.specimenCount !== 1 ? 's' : ''}`;
    return '';
  }

  // ── Kingdoms for filter quick-jump ─────────────────────────────────────────

  let kingdoms = $derived(columns[0]?.items ?? []);

  // ── Mount ──────────────────────────────────────────────────────────────────

  onMount(async () => {
    await loadKingdomColumn();

    const preselect = $selectedStrainId;
    if (preselect) {
      selectedStrainId.set(null);
      await openStrainPanel(preselect);
    } else {
      await restoreSavedPath();
    }
  });
</script>

<!-- ── Global keyboard handler ──────────────────────────────────────────────── -->
<svelte:window onkeydown={handleKeydown} />

<div class="tx-nav">

  <!-- ── Header: search + filter toggle ──────────────────────────────────── -->
  <div class="tx-header">
    <div class="tx-search-wrap" role="combobox" aria-expanded={showSearchResults} aria-haspopup="listbox" aria-controls="tx-search-listbox">
      <input
        bind:this={searchInputEl}
        type="search"
        class="tx-search-input"
        placeholder="Search taxa, species, strains, accessions… (or press /)"
        bind:value={searchQuery}
        oninput={handleSearchInput}
        onfocus={() => { if (searchResults.length > 0) showSearchResults = true; }}
        onblur={() => setTimeout(() => { showSearchResults = false; }, 200)}
        aria-label="Global taxonomy search"
        aria-autocomplete="list"
      />
      {#if showSearchResults && searchQuery.length >= 2}
        <div id="tx-search-listbox" class="tx-search-dropdown" role="listbox" aria-label="Search results">
          {#if groupedResults.taxa.length > 0}
            <div class="sr-group-label">Taxa</div>
            {#each groupedResults.taxa as r}
              <button class="sr-item" role="option" aria-selected="false" onclick={() => navigateToSearchResult(r)}>
                <span class="sr-name">{r.display_name}</span>
                <span class="sr-secondary">{r.secondary}</span>
              </button>
            {/each}
          {/if}
          {#if groupedResults.species.length > 0}
            <div class="sr-group-label">Species</div>
            {#each groupedResults.species as r}
              <button class="sr-item" role="option" aria-selected="false" onclick={() => navigateToSearchResult(r)}>
                <span class="sr-name">{r.display_name}</span>
                <span class="sr-secondary">{r.secondary}</span>
              </button>
            {/each}
          {/if}
          {#if groupedResults.strains.length > 0}
            <div class="sr-group-label">Strains</div>
            {#each groupedResults.strains as r}
              <button class="sr-item" role="option" aria-selected="false" onclick={() => navigateToSearchResult(r)}>
                <span class="sr-name">{r.display_name}</span>
                <span class="sr-secondary">{r.secondary}</span>
              </button>
            {/each}
          {/if}
          {#if groupedResults.specimens.length > 0}
            <div class="sr-group-label">Specimens</div>
            {#each groupedResults.specimens as r}
              <button class="sr-item" role="option" aria-selected="false" onclick={() => navigateToSearchResult(r)}>
                <span class="sr-name">{r.display_name}</span>
                <span class="sr-secondary">{r.secondary} — navigate to SpecimenDetail</span>
              </button>
            {/each}
          {/if}
        </div>
      {/if}
    </div>
    <button
      class="btn btn-sm"
      class:active={showFilters}
      onclick={() => (showFilters = !showFilters)}
      aria-pressed={showFilters}
      aria-label="Toggle filters"
    >
      Filters {showFilters ? '▲' : '▼'}
    </button>
  </div>

  <!-- ── Filter bar ──────────────────────────────────────────────────────── -->
  {#if showFilters}
    <div class="tx-filters" role="group" aria-label="Navigator filters">
      <label class="filter-label">
        Strain status
        <select bind:value={filterStatus} class="filter-select" aria-label="Filter by strain status">
          <option value="all">All</option>
          <option value="unverified">Unverified</option>
          <option value="claimed">Claimed</option>
          <option value="confirmed_manual">Confirmed (Manual)</option>
          <option value="confirmed_genomic">Confirmed (Genomic)</option>
          <option value="confirmed_any">Confirmed (Any)</option>
        </select>
      </label>
      {#if kingdoms.length > 0}
        <div class="filter-kingdoms">
          <span class="filter-label-text">Jump to:</span>
          {#each kingdoms as k}
            <button
              class="kingdom-pill"
              class:active={columns[0]?.selectedId === k.id}
              onclick={() => selectItem(0, k)}
              title="{k.strainCount} strains · {k.specimenCount} specimens"
            >
              {k.primaryLabel}
            </button>
          {/each}
        </div>
      {/if}
    </div>
  {/if}

  <!-- ── Breadcrumb ──────────────────────────────────────────────────────── -->
  <nav class="tx-breadcrumb" aria-label="Taxonomy path">
    <button class="bc-item bc-root" onclick={resetToRoot}>All</button>
    {#each selectedPath as node, i}
      <span class="bc-sep" aria-hidden="true">›</span>
      <button
        class="bc-item"
        class:bc-last={i === selectedPath.length - 1}
        onclick={() => navigateToBreadcrumb(i)}
        title={node.rank ?? node.kind}
      >
        {node.label}
      </button>
    {/each}
  </nav>

  <!-- ── Column browser ─────────────────────────────────────────────────── -->
  <div
    class="tx-columns"
    bind:this={columnsContainerEl}
    role="region"
    aria-label="Taxonomy hierarchy"
  >
    {#each columns as col, ci}
      <div
        class="tx-column"
        class:focused-col={focusedCol === ci}
        role="listbox"
        aria-label={colHeaderLabel(col)}
        aria-multiselectable="false"
      >
        <!-- Column header -->
        <div class="col-head">
          <span class="col-head-label">{colHeaderLabel(col)}</span>
          {#if col.kind === 'species' && col.selectedId}
            <button
              class="btn btn-sm btn-primary col-head-action"
              onclick={() => {
                const item = col.items.find(i => i.id === col.selectedId);
                if (item) {
                  managerSpeciesId = col.selectedId;
                  managerSpeciesName = item.primaryLabel;
                  showManager = true;
                }
              }}
              title="Manage strains for selected species"
            >
              Manage
            </button>
          {/if}
        </div>

        <!-- Items -->
        <div class="col-body">
          {#if col.loading}
            <div class="col-empty">Loading…</div>
          {:else if getDisplayItems(col).length === 0}
            <div class="col-empty">
              {col.items.length === 0 ? 'None' : 'No matches'}
            </div>
          {:else}
            {#each getDisplayItems(col) as item, ri}
              <button
                id="col-{ci}-row-{ri}"
                class="col-item"
                class:selected={col.selectedId === item.id}
                class:kbd-focused={focusedCol === ci && focusedRow === ri}
                role="option"
                aria-selected={col.selectedId === item.id}
                onclick={() => { focusedCol = ci; focusedRow = ri; selectItem(ci, item); }}
                onmouseenter={() => { focusedCol = ci; focusedRow = ri; }}
                title={item.primaryLabel}
              >
                <div class="ci-primary">{item.primaryLabel}</div>
                <div class="ci-meta">
                  {#if item.statusKey}
                    <span class="ci-badge {statusBadgeClass(item.statusKey)}">
                      {statusLabel(item.statusKey)}
                    </span>
                  {/if}
                  {#if item.isHybrid}
                    <span class="ci-hybrid">H</span>
                  {/if}
                  {#if item.isCrossSpecies}
                    <span class="ci-cross" title="Cross-species hybrid">⚠</span>
                  {/if}
                  {#if countLabel(item)}
                    <span class="ci-counts">{countLabel(item)}</span>
                  {/if}
                </div>
              </button>
            {/each}
          {/if}
        </div>
      </div>
    {/each}

    {#if columns.length === 0}
      <div class="tx-empty-state">
        <div style="font-size:28px;margin-bottom:10px;">🌿</div>
        <p>Loading taxonomy…</p>
      </div>
    {/if}
  </div>

  <!-- ── Strain quick-action panel ──────────────────────────────────────── -->
  {#if panelStrainId}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="panel-overlay" onclick={closePanel}></div>
    <aside class="tx-panel" aria-label="Strain specimens panel">
      <div class="panel-head">
        {#if panelStrain}
          <div class="panel-strain-info">
            <div class="panel-strain-top">
              <code class="panel-code">{panelStrain.code}</code>
              {#if panelStrain.status}
                <span class="ci-badge {statusBadgeClass(panelStrain.status)}">
                  {statusLabel(panelStrain.status)}
                </span>
              {/if}
              {#if panelStrain.is_hybrid}
                <span class="ci-hybrid">Hybrid</span>
              {/if}
            </div>
            <div class="panel-strain-name">{panelStrain.name}</div>
          </div>
        {:else}
          <div class="panel-strain-name">Strain Specimens</div>
        {/if}
        <div class="panel-head-actions">
          {#if panelStrain}
            <button
              class="btn btn-sm"
              onclick={() => { detailStrainId = panelStrainId; showStrainDetail = true; }}
              title="View full strain details"
            >
              Details
            </button>
          {/if}
          <button class="panel-close" onclick={closePanel} aria-label="Close panel">✕</button>
        </div>
      </div>

      {#if panelLoading}
        <div class="col-empty" style="padding:32px;">Loading specimens…</div>
      {:else if panelSpecimens.length === 0}
        <div class="col-empty" style="padding:32px;">No specimens bound to this strain.</div>
      {:else}
        <div class="panel-count">{panelSpecimens.length} specimen{panelSpecimens.length !== 1 ? 's' : ''}</div>
        <div class="panel-body">
          {#each panelSpecimens as sp}
            <button
              class="panel-spec-row"
              onclick={() => { selectedSpecimenId.set(sp.id); navigateTo('specimen-detail'); }}
              title="Open specimen detail"
            >
              <div>
                <div class="psr-acc">{sp.accession_number}</div>
                <div class="psr-meta">{sp.stage ?? '—'} · {sp.location ?? 'No location'}</div>
              </div>
              <span class="psr-health" title="Health status">{healthBar(sp.health_status)}</span>
            </button>
          {/each}
        </div>
      {/if}
    </aside>
  {/if}

  <!-- ── StrainDetail slide-over ─────────────────────────────────────────── -->
  {#if showStrainDetail && detailStrainId}
    <StrainDetail
      strainId={detailStrainId}
      onclose={() => { showStrainDetail = false; detailStrainId = null; }}
    />
  {/if}

  <!-- ── StrainManager overlay ──────────────────────────────────────────── -->
  {#if showManager && managerSpeciesId}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="manager-overlay" onclick={(e) => { if (e.target === e.currentTarget) showManager = false; }}>
      <div class="manager-inner">
        <div class="manager-head">
          <button class="btn btn-sm" onclick={() => (showManager = false)}>← Close</button>
          <h3>{managerSpeciesName}</h3>
        </div>
        <StrainManager
          speciesId={managerSpeciesId}
          speciesName={managerSpeciesName}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  /* ── Layout ──────────────────────────────────────────────────────────── */
  .tx-nav {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 96px);
    min-height: 400px;
    position: relative;
    overflow: hidden;
  }

  /* ── Header ──────────────────────────────────────────────────────────── */
  .tx-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 12px;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
  }
  .tx-search-wrap {
    position: relative;
    flex: 1;
  }
  .tx-search-input {
    width: 100%;
    font-size: 13px;
    padding: 6px 10px;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: white;
    color: #1e293b;
  }
  .tx-search-input:focus { outline: 2px solid #2563eb; outline-offset: -1px; }

  /* Search dropdown */
  .tx-search-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: white;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.12);
    z-index: 100;
    max-height: 360px;
    overflow-y: auto;
  }
  .sr-group-label {
    padding: 6px 12px 2px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #94a3b8;
  }
  .sr-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 7px 12px;
    border: none;
    background: none;
    cursor: pointer;
    text-align: left;
    gap: 8px;
  }
  .sr-item:hover { background: #f1f5f9; }
  .sr-name { font-size: 13px; color: #1e293b; font-weight: 500; }
  .sr-secondary { font-size: 11px; color: #94a3b8; flex-shrink: 0; }

  /* ── Filters ──────────────────────────────────────────────────────────── */
  .tx-filters {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 8px 12px;
    background: #f8fafc;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
    flex-wrap: wrap;
  }
  .filter-label {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: #475569;
  }
  .filter-select { font-size: 12px; padding: 3px 6px; }
  .filter-label-text { font-size: 12px; color: #475569; }
  .filter-kingdoms { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .kingdom-pill {
    padding: 2px 10px;
    border-radius: 12px;
    border: 1px solid #cbd5e1;
    background: white;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.1s;
  }
  .kingdom-pill:hover { border-color: #93c5fd; background: #eff6ff; }
  .kingdom-pill.active { background: #2563eb; border-color: #2563eb; color: white; }

  /* ── Breadcrumb ───────────────────────────────────────────────────────── */
  .tx-breadcrumb {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 12px;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
    overflow-x: auto;
    white-space: nowrap;
  }
  .bc-item {
    background: none;
    border: none;
    font-size: 12px;
    color: #6b7280;
    cursor: pointer;
    padding: 2px 5px;
    border-radius: 4px;
    transition: all 0.1s;
  }
  .bc-item:hover { background: #f1f5f9; color: #1e293b; }
  .bc-root { color: #2563eb; font-weight: 500; }
  .bc-last { color: #1e293b; font-weight: 600; }
  .bc-sep { color: #cbd5e1; font-size: 12px; user-select: none; }

  /* ── Column container ─────────────────────────────────────────────────── */
  .tx-columns {
    display: flex;
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    scroll-behavior: smooth;
    gap: 0;
  }
  .tx-empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: #94a3b8;
    font-size: 14px;
  }

  /* ── Individual column ────────────────────────────────────────────────── */
  .tx-column {
    width: 185px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    border-right: 1px solid #e2e8f0;
    overflow: hidden;
  }
  .tx-column.focused-col { border-right-color: #93c5fd; }
  .col-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 10px 6px;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
    background: #f8fafc;
  }
  .col-head-label {
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: #94a3b8;
  }
  .col-head-action { font-size: 10px; padding: 2px 6px; }
  .col-body { flex: 1; overflow-y: auto; padding: 4px; }
  .col-empty { padding: 16px 10px; font-size: 12px; color: #94a3b8; text-align: center; }

  /* ── Column items ─────────────────────────────────────────────────────── */
  .col-item {
    display: flex;
    flex-direction: column;
    gap: 3px;
    width: 100%;
    padding: 7px 8px;
    border: 1px solid transparent;
    border-radius: 5px;
    background: none;
    cursor: pointer;
    text-align: left;
    transition: all 0.1s;
    margin-bottom: 1px;
  }
  .col-item:hover { background: #f1f5f9; border-color: #e2e8f0; }
  .col-item.selected {
    background: #eff6ff;
    border-color: #93c5fd;
  }
  .col-item.kbd-focused { outline: 2px solid #2563eb; outline-offset: -2px; }
  .ci-primary { font-size: 12px; font-weight: 500; color: #1e293b; line-height: 1.3; word-break: break-word; }
  .ci-meta { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; margin-top: 2px; }
  .ci-counts { font-size: 10px; color: #94a3b8; }
  .ci-hybrid {
    font-size: 9px;
    background: #ede9fe;
    color: #6d28d9;
    border-radius: 8px;
    padding: 0px 5px;
    font-weight: 600;
  }
  .ci-cross { font-size: 10px; color: #dc2626; }

  /* ── Status badges ────────────────────────────────────────────────────── */
  .ci-badge {
    display: inline-block;
    font-size: 9px;
    font-weight: 600;
    padding: 1px 5px;
    border-radius: 8px;
  }
  .st-unverified { background: #f1f5f9; color: #475569; }
  .st-claimed { background: #dbeafe; color: #1e40af; }
  .st-manual { background: #fef3c7; color: #92400e; }
  .st-genomic { background: #dcfce7; color: #166534; }

  /* ── Strain panel ─────────────────────────────────────────────────────── */
  .panel-overlay {
    position: absolute;
    inset: 0;
    z-index: 10;
    background: rgba(0,0,0,0.1);
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
    animation: slideIn 0.18s ease;
  }
  @keyframes slideIn {
    from { transform: translateX(100%); }
    to { transform: translateX(0); }
  }
  .panel-head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 14px 14px 12px;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
    gap: 8px;
  }
  .panel-strain-info { flex: 1; min-width: 0; }
  .panel-strain-top { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; margin-bottom: 4px; }
  .panel-code { font-size: 11px; font-weight: 700; background: #f1f5f9; padding: 2px 5px; border-radius: 3px; font-family: monospace; }
  .panel-strain-name { font-size: 14px; font-weight: 600; color: #1e293b; }
  .panel-head-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
  .panel-close {
    background: none;
    border: none;
    font-size: 14px;
    cursor: pointer;
    color: #6b7280;
    padding: 2px 6px;
    border-radius: 4px;
    line-height: 1;
  }
  .panel-close:hover { background: #f3f4f6; }
  .panel-count { padding: 6px 14px 2px; font-size: 11px; color: #94a3b8; flex-shrink: 0; }
  .panel-body { flex: 1; overflow-y: auto; padding: 6px 8px; }
  .panel-spec-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    padding: 8px 10px;
    border: 1px solid #f1f5f9;
    border-radius: 6px;
    margin-bottom: 3px;
    background: none;
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }
  .panel-spec-row:hover { background: #f8fafc; }
  .psr-acc { font-size: 12px; font-weight: 600; color: #1e293b; }
  .psr-meta { font-size: 11px; color: #6b7280; margin-top: 1px; }
  .psr-health { font-size: 10px; color: #6b7280; font-family: monospace; letter-spacing: -2px; flex-shrink: 0; }

  /* ── StrainManager overlay ────────────────────────────────────────────── */
  .manager-overlay {
    position: absolute;
    inset: 0;
    z-index: 30;
    background: rgba(0,0,0,0.2);
    display: flex;
    align-items: stretch;
  }
  .manager-inner {
    flex: 1;
    background: white;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .manager-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-bottom: 1px solid #e2e8f0;
    flex-shrink: 0;
  }
  .manager-head h3 { font-size: 15px; font-weight: 600; }

  /* ── Dark mode ────────────────────────────────────────────────────────── */
  :global(.dark) .tx-header { border-bottom-color: #334155; }
  :global(.dark) .tx-search-input { background: #1e293b; border-color: #334155; color: #e2e8f0; }
  :global(.dark) .tx-search-dropdown { background: #1e293b; border-color: #334155; }
  :global(.dark) .sr-item:hover { background: #0f172a; }
  :global(.dark) .sr-name { color: #e2e8f0; }
  :global(.dark) .tx-filters { background: #0f172a; border-bottom-color: #334155; }
  :global(.dark) .tx-breadcrumb { border-bottom-color: #334155; }
  :global(.dark) .bc-item:hover { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .bc-last { color: #e2e8f0; }
  :global(.dark) .tx-column { border-right-color: #334155; }
  :global(.dark) .col-head { background: #0f172a; border-bottom-color: #334155; }
  :global(.dark) .col-item:hover { background: #1e293b; border-color: #334155; }
  :global(.dark) .col-item.selected { background: #1e3a5f; border-color: #2563eb; }
  :global(.dark) .ci-primary { color: #e2e8f0; }
  :global(.dark) .tx-panel { background: #1e293b; border-left-color: #334155; }
  :global(.dark) .panel-head { border-bottom-color: #334155; }
  :global(.dark) .panel-strain-name { color: #e2e8f0; }
  :global(.dark) .panel-code { background: #334155; color: #e2e8f0; }
  :global(.dark) .panel-spec-row { border-color: #334155; }
  :global(.dark) .panel-spec-row:hover { background: #0f172a; }
  :global(.dark) .psr-acc { color: #e2e8f0; }
  :global(.dark) .manager-inner { background: #1e293b; }
  :global(.dark) .manager-head { border-bottom-color: #334155; }
  :global(.dark) .st-unverified { background: #334155; color: #94a3b8; }
  :global(.dark) .st-claimed { background: #1e40af; color: #dbeafe; }
  :global(.dark) .st-manual { background: #78350f; color: #fde68a; }
  :global(.dark) .st-genomic { background: #166534; color: #dcfce7; }
  :global(.dark) .kingdom-pill { background: #1e293b; border-color: #334155; color: #cbd5e1; }
  :global(.dark) .kingdom-pill:hover { border-color: #60a5fa; background: #1e3a5f; }
  :global(.dark) .kingdom-pill.active { background: #2563eb; border-color: #2563eb; color: white; }

  /* ── Mobile ───────────────────────────────────────────────────────────── */
  @media (max-width: 768px) {
    .tx-nav { height: calc(100vh - 140px); }
    .tx-column { width: 160px; }
    .tx-panel { width: min(300px, 88vw); }
  }
  @media (max-width: 480px) {
    .tx-column { width: 140px; }
    .tx-filters { gap: 8px; }
  }

  /* Subtle active indicator for the filter button */
  .btn.active { background: #dbeafe; color: #1e40af; border-color: #93c5fd; }
</style>
