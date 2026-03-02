<script lang="ts">
  import { onMount } from 'svelte';
  import { listSpecies, createSpecies } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let species = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state({ genus: '', species_name: '', common_name: '', species_code: '', default_subculture_interval_days: '28', notes: '' });

  onMount(() => { load(); });

  async function load() {
    loading = true;
    try { species = await listSpecies(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    try {
      await createSpecies({
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
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }
</script>

<div>
  <div class="page-header">
    <h1>Species Registry</h1>
    {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
      <button class="btn btn-primary" title={showForm ? 'Cancel and close the form' : 'Open form to register a new species'} onclick={() => showForm = !showForm}>
        {showForm ? 'Cancel' : '+ Add Species'}
      </button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">Add Species</h3>
        <div class="form-row">
          <div class="form-group">
            <label title="Taxonomic genus of the plant species">Genus *</label>
            <input type="text" title="Enter the genus name, e.g. Citrus" bind:value={form.genus} required placeholder="e.g., Citrus" />
          </div>
          <div class="form-group">
            <label title="Specific epithet (species part of the binomial name)">Species Name *</label>
            <input type="text" title="Enter the species epithet, e.g. sinensis" bind:value={form.species_name} required placeholder="e.g., sinensis" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label title="Vernacular or common name for this species">Common Name</label>
            <input type="text" title="Enter the common name, e.g. Sweet Orange" bind:value={form.common_name} placeholder="e.g., Sweet Orange" />
          </div>
          <div class="form-group">
            <label title="Short unique code used to identify this species in the system">Species Code *</label>
            <input type="text" title="Enter a short unique code for this species, e.g. CIT-SIN" bind:value={form.species_code} required placeholder="e.g., CIT-SIN" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label title="Default number of days between subculture transfers for this species">Default Subculture Interval (days)</label>
            <input type="number" title="Enter how many days between subculture transfers" bind:value={form.default_subculture_interval_days} />
          </div>
          <div class="form-group">
            <label title="Optional notes about this species">Notes</label>
            <input type="text" title="Enter any additional notes about this species" bind:value={form.notes} />
          </div>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary" title="Save this new species to the registry">Add Species</button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else}
    <div class="card">
      <table>
        <thead>
          <tr>
            <th title="Short unique identifier for this species">Code</th>
            <th title="Taxonomic genus of the species">Genus</th>
            <th title="Specific epithet of the species">Species</th>
            <th title="Common or vernacular name of the species">Common Name</th>
            <th title="Default number of days between subculture transfers">Subculture Interval</th>
            <th title="Additional notes about the species">Notes</th>
          </tr>
        </thead>
        <tbody>
          {#each species as s}
            <tr>
              <td><strong>{s.species_code}</strong></td>
              <td><em>{s.genus}</em></td>
              <td><em>{s.species_name}</em></td>
              <td>{s.common_name || '—'}</td>
              <td>{s.default_subculture_interval_days ? `${s.default_subculture_interval_days} days` : '—'}</td>
              <td>{s.notes || '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>
