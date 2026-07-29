<script lang="ts">
  import { onMount } from 'svelte';
  import { listSpecies, createSpecies, rebuildSpeciesTaxonomy, locateSpecies } from '../api';
  import { addNotification, navigateTo, focusSpeciesId } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import StrainManager from './StrainManager.svelte';
  import DataState from './DataState.svelte';

  let species = $state<any[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let showForm = $state(false);
  let form = $state({ genus: '', species_name: '', common_name: '', species_code: '', default_subculture_interval_days: '28', notes: '' });
  let saving = $state(false);

  // Row selection. A species row is the natural entry point to everything that
  // hangs off a species — its strains and its place in the taxonomy — and until
  // now the table was inert, so neither was reachable from here.
  let selectedId = $state<string | null>(null);
  let jumping = $state(false);

  // Strain manager overlay, opened for the selected species.
  let strainSpeciesId = $state<string | null>(null);
  let strainSpeciesName = $state('');

  // Taxonomy repair
  let rebuilding = $state(false);

  const canManage = $derived($currentUser?.role === 'admin' || $currentUser?.role === 'supervisor');
  const selected = $derived(species.find((s) => s.id === selectedId) ?? null);

  // Species whose taxon_path never got written. These are exactly the species
  // that are invisible in the Taxonomy Navigator, which resolves everything
  // through that column — so counting them is how we know whether to offer the
  // rebuild.
  const unclassified = $derived(species.filter((s) => !s.taxon_path));

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
    try {
      species = await listSpecies();
      // Drop a stale selection if the row is gone after a reload.
      if (selectedId && !species.some((s) => s.id === selectedId)) selectedId = null;
    } catch (e: any) {
      error = e.message;
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function speciesLabel(s: any): string {
    return `${s.genus} ${s.species_name}`;
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    saving = true;
    try {
      const created = await createSpecies({
        genus: form.genus,
        species_name: form.species_name,
        common_name: form.common_name || undefined,
        species_code: form.species_code,
        default_subculture_interval_days: form.default_subculture_interval_days ? parseInt(form.default_subculture_interval_days) : undefined,
        notes: form.notes || undefined,
      });
      addNotification('Species added', 'success');
      showForm = false;
      form = { genus: '', species_name: '', common_name: '', species_code: '', default_subculture_interval_days: '28', notes: '' };
      await load();
      // Select what was just created so the strain and taxonomy actions are one
      // click away — adding a species is almost always followed by adding its
      // first strain.
      if (created?.id) selectedId = created.id;
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      saving = false;
    }
  }

  function toggleSelect(id: string) {
    selectedId = selectedId === id ? null : id;
  }

  function handleRowKeydown(e: KeyboardEvent, id: string) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      toggleSelect(id);
    }
  }

  /// Jump into the Taxonomy tab with this species selected. When the species has
  /// no genus taxon the jump would land on an empty tree, so we say so and point
  /// at the rebuild rather than navigating into a dead end.
  async function openInTaxonomy(id: string) {
    jumping = true;
    try {
      const located = await locateSpecies(id);
      if (!located) {
        addNotification(
          'This species is not classified yet — run "Build taxonomy from species" first.',
          'warning'
        );
        return;
      }
      focusSpeciesId.set(id);
      navigateTo('taxonomy');
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      jumping = false;
    }
  }

  function openStrains(s: any) {
    strainSpeciesId = s.id;
    strainSpeciesName = speciesLabel(s);
  }

  async function handleRebuild() {
    rebuilding = true;
    try {
      const result = await rebuildSpeciesTaxonomy();
      if (result.species_linked === 0) {
        addNotification('Every species is already classified — nothing to do.', 'info');
      } else {
        addNotification(
          `Taxonomy rebuilt — ${result.genera_created} genus taxa created, ${result.species_linked} species classified.`,
          'success'
        );
      }
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      rebuilding = false;
    }
  }
</script>

<div>
  <div class="page-header">
    <h1>Species Registry</h1>
    {#if canManage}
      <button class="btn btn-primary" title={showForm ? 'Cancel and close the form' : 'Open form to register a new species'} onclick={() => showForm = !showForm}>
        {showForm ? 'Cancel' : '+ Add Species'}
      </button>
    {/if}
  </div>

  {#if !loading && !error && unclassified.length > 0 && canManage}
    <div class="repair-banner">
      <div>
        <strong>{unclassified.length} species {unclassified.length === 1 ? 'is' : 'are'} not in the taxonomy tree.</strong>
        <span>
          The Taxonomy Navigator browses Kingdom → … → Genus → Species, so a species with no genus
          taxon never appears there. Building the backbone creates the missing genus taxa from the
          genus names already on these records — nothing else changes.
        </span>
      </div>
      <button
        class="btn btn-primary"
        onclick={handleRebuild}
        disabled={rebuilding}
        title="Create the missing genus taxa and classify these species — safe to run repeatedly"
      >
        {rebuilding ? 'Building…' : 'Build taxonomy from species'}
      </button>
    </div>
  {/if}

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">Add Species</h3>
        <div class="form-row">
          <div class="form-group">
            <label for="species-genus" title="Taxonomic genus of the plant species">Genus *</label>
            <input id="species-genus" type="text" title="Enter the genus name, e.g. Citrus — a matching genus taxon is created automatically" bind:value={form.genus} required placeholder="e.g., Citrus" />
          </div>
          <div class="form-group">
            <label for="species-name" title="Specific epithet (species part of the binomial name)">Species Name *</label>
            <input id="species-name" type="text" title="Enter the species epithet, e.g. sinensis" bind:value={form.species_name} required placeholder="e.g., sinensis" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label for="species-common-name" title="Vernacular or common name for this species">Common Name</label>
            <input id="species-common-name" type="text" title="Enter the common name, e.g. Sweet Orange" bind:value={form.common_name} placeholder="e.g., Sweet Orange" />
          </div>
          <div class="form-group">
            <label for="species-code" title="Short unique code used to identify this species in the system">Species Code *</label>
            <input id="species-code" type="text" title="Enter a short unique code for this species, e.g. CIT-SIN" bind:value={form.species_code} required placeholder="e.g., CIT-SIN" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label for="species-subculture-interval" title="Default number of days between subculture transfers for this species">Default Subculture Interval (days)</label>
            <input id="species-subculture-interval" type="number" title="Enter how many days between subculture transfers" bind:value={form.default_subculture_interval_days} />
          </div>
          <div class="form-group">
            <label for="species-notes" title="Optional notes about this species">Notes</label>
            <input id="species-notes" type="text" title="Enter any additional notes about this species" bind:value={form.notes} />
          </div>
        </div>
        <p class="form-hint">
          Saving also files this species under its genus in the taxonomy tree, creating the genus
          taxon if this is the first species in it.
        </p>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary" disabled={saving} title="Save this new species to the registry">
            {saving ? 'Saving…' : 'Add Species'}
          </button>
        </div>
      </form>
    </div>
  {/if}

  <DataState
    {loading}
    {error}
    empty={!loading && !error && species.length === 0}
    emptyIcon="🌱"
    emptyTitle="No species yet"
    emptyMessage="Register your first species — every specimen belongs to one, so this is the place to start."
    emptyActionLabel={canManage ? '+ Add Species' : ''}
    onemptyaction={canManage ? () => (showForm = true) : undefined}
    onretry={load}
  >
    <div class="card">
      <p class="table-hint">Select a species to manage its strains or open it in the Taxonomy Navigator.</p>
      <table>
        <thead>
          <tr>
            <th title="Short unique identifier for this species">Code</th>
            <th title="Taxonomic genus of the species">Genus</th>
            <th title="Specific epithet of the species">Species</th>
            <th title="Common or vernacular name of the species">Common Name</th>
            <th title="Default number of days between subculture transfers">Subculture Interval</th>
            <th title="Whether this species appears in the Taxonomy Navigator">Taxonomy</th>
            <th title="Additional notes about the species">Notes</th>
          </tr>
        </thead>
        <tbody>
          {#each species as s (s.id)}
            <tr
              class="species-row"
              class:selected={selectedId === s.id}
              tabindex="0"
              role="button"
              aria-pressed={selectedId === s.id}
              aria-label="Select {speciesLabel(s)}"
              title="Select {speciesLabel(s)} to manage its strains or open it in the Taxonomy Navigator"
              onclick={() => toggleSelect(s.id)}
              onkeydown={(e) => handleRowKeydown(e, s.id)}
            >
              <td><strong>{s.species_code}</strong></td>
              <td><em>{s.genus}</em></td>
              <td><em>{s.species_name}</em></td>
              <td>{s.common_name || '—'}</td>
              <td>{s.default_subculture_interval_days ? `${s.default_subculture_interval_days} days` : '—'}</td>
              <td>
                {#if s.taxon_path}
                  <span class="badge badge-green" title="Classified — visible in the Taxonomy Navigator">Classified</span>
                {:else}
                  <span class="badge badge-yellow" title="No genus taxon — this species does not appear in the Taxonomy Navigator yet">Unclassified</span>
                {/if}
              </td>
              <td>{s.notes || '—'}</td>
            </tr>
            {#if selectedId === s.id}
              <tr class="action-row">
                <td colspan="7">
                  <div class="row-actions">
                    <span class="row-actions-label">{speciesLabel(s)}</span>
                    <button
                      class="btn btn-sm"
                      onclick={(e) => { e.stopPropagation(); openStrains(s); }}
                      title="Add, edit, and verify the strains and cultivars of this species"
                    >
                      Manage strains
                    </button>
                    <button
                      class="btn btn-sm"
                      disabled={jumping}
                      onclick={(e) => { e.stopPropagation(); openInTaxonomy(s.id); }}
                      title="Open the Taxonomy Navigator with this species selected"
                    >
                      {jumping ? 'Opening…' : 'Open in Taxonomy →'}
                    </button>
                  </div>
                </td>
              </tr>
            {/if}
          {/each}
        </tbody>
      </table>
    </div>
  </DataState>

  <!-- Strain manager overlay for the selected species -->
  {#if strainSpeciesId}
    <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
    <div
      class="strain-overlay"
      onclick={(e) => { if (e.target === e.currentTarget) strainSpeciesId = null; }}
    >
      <div class="strain-inner" role="dialog" aria-modal="true" aria-label="Strains of {strainSpeciesName}">
        <div class="strain-head">
          <button class="btn btn-sm" onclick={() => (strainSpeciesId = null)} title="Close the strain manager">← Close</button>
          <h3>{strainSpeciesName}</h3>
        </div>
        <StrainManager speciesId={strainSpeciesId} speciesName={strainSpeciesName} />
      </div>
    </div>
  {/if}
</div>

<style>
  .repair-banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4, 16px);
    flex-wrap: wrap;
    padding: 12px 16px;
    margin-bottom: 16px;
    border: 1px solid var(--color-warning-border, #fcd34d);
    background: var(--color-warning-bg, #fffbeb);
    border-radius: var(--radius-md, 8px);
    font-size: 13px;
    line-height: 1.5;
  }
  .repair-banner span {
    display: block;
    color: var(--color-text-muted, #6b7280);
    margin-top: 2px;
  }
  :global(.dark) .repair-banner {
    background: #422006;
    border-color: #854d0e;
  }
  :global(.dark) .repair-banner span { color: #d6d3d1; }

  .form-hint {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin-bottom: 12px;
  }
  .table-hint {
    font-size: 12px;
    color: var(--color-text-muted, #6b7280);
    margin-bottom: 10px;
  }

  .species-row { cursor: pointer; }
  .species-row.selected > :global(td) {
    background: var(--color-accent-soft, #eff6ff);
  }
  :global(.dark) .species-row.selected > :global(td) {
    background: #1e3a5f;
  }

  .action-row > :global(td) {
    background: var(--color-surface-raised, #f8fafc);
    padding-top: 8px;
    padding-bottom: 8px;
  }
  :global(.dark) .action-row > :global(td) { background: #0f172a; }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .row-actions-label {
    font-weight: 600;
    font-style: italic;
    margin-right: 4px;
  }

  .strain-overlay {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.55);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding: 24px 16px;
    z-index: 900;
    overflow-y: auto;
  }
  .strain-inner {
    background: var(--color-surface, #fff);
    border-radius: var(--radius-lg, 10px);
    padding: 20px;
    width: min(1000px, 100%);
  }
  :global(.dark) .strain-inner { background: #1e293b; }
  .strain-head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }
  .strain-head h3 { font-size: 16px; font-weight: 700; }
</style>
