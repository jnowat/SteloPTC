<script lang="ts">
  import { listSpecies, createSpecies } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let species = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state({ genus: '', species_name: '', common_name: '', species_code: '', default_subculture_interval_days: '28', notes: '' });

  $effect(() => { load(); });

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
      <button class="btn btn-primary" onclick={() => showForm = !showForm}>
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
            <label>Genus *</label>
            <input type="text" bind:value={form.genus} required placeholder="e.g., Citrus" />
          </div>
          <div class="form-group">
            <label>Species Name *</label>
            <input type="text" bind:value={form.species_name} required placeholder="e.g., sinensis" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Common Name</label>
            <input type="text" bind:value={form.common_name} placeholder="e.g., Sweet Orange" />
          </div>
          <div class="form-group">
            <label>Species Code *</label>
            <input type="text" bind:value={form.species_code} required placeholder="e.g., CIT-SIN" />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Default Subculture Interval (days)</label>
            <input type="number" bind:value={form.default_subculture_interval_days} />
          </div>
          <div class="form-group">
            <label>Notes</label>
            <input type="text" bind:value={form.notes} />
          </div>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary">Add Species</button>
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
            <th>Code</th>
            <th>Genus</th>
            <th>Species</th>
            <th>Common Name</th>
            <th>Subculture Interval</th>
            <th>Notes</th>
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
