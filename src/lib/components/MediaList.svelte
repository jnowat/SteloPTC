<script lang="ts">
  import { onMount } from 'svelte';
  import { listMedia, createMediaBatch, deleteMediaBatch } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let media = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state({
    name: '', preparation_date: new Date().toISOString().split('T')[0],
    expiration_date: '', basal_salts: 'MS', basal_salts_concentration: '1.0',
    sucrose_g_per_l: '30', agar_g_per_l: '8', ph_before_autoclave: '5.7',
    volume_prepared_ml: '', sterilization_method: 'autoclave', notes: '',
  });

  onMount(() => { load(); });

  async function load() {
    loading = true;
    try { media = await listMedia(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    try {
      await createMediaBatch({
        name: form.name,
        preparation_date: form.preparation_date,
        expiration_date: form.expiration_date || undefined,
        basal_salts: form.basal_salts || undefined,
        basal_salts_concentration: form.basal_salts_concentration ? parseFloat(form.basal_salts_concentration) : undefined,
        sucrose_g_per_l: form.sucrose_g_per_l ? parseFloat(form.sucrose_g_per_l) : undefined,
        agar_g_per_l: form.agar_g_per_l ? parseFloat(form.agar_g_per_l) : undefined,
        ph_before_autoclave: form.ph_before_autoclave ? parseFloat(form.ph_before_autoclave) : undefined,
        volume_prepared_ml: form.volume_prepared_ml ? parseFloat(form.volume_prepared_ml) : undefined,
        sterilization_method: form.sterilization_method || undefined,
        notes: form.notes || undefined,
      });
      addNotification('Media batch created', 'success');
      showForm = false;
      form = { name: '', preparation_date: new Date().toISOString().split('T')[0], expiration_date: '', basal_salts: 'MS', basal_salts_concentration: '1.0', sucrose_g_per_l: '30', agar_g_per_l: '8', ph_before_autoclave: '5.7', volume_prepared_ml: '', sterilization_method: 'autoclave', notes: '' };
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this media batch?')) return;
    try {
      await deleteMediaBatch(id);
      addNotification('Media batch deleted', 'success');
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  function isExpired(expDate: string | null): boolean {
    if (!expDate) return false;
    return new Date(expDate) < new Date();
  }

  function isExpiringSoon(expDate: string | null): boolean {
    if (!expDate) return false;
    const exp = new Date(expDate);
    const week = new Date();
    week.setDate(week.getDate() + 7);
    return exp > new Date() && exp <= week;
  }
</script>

<div>
  <div class="page-header">
    <h1>Media Logs</h1>
    {#if $currentUser?.role !== 'guest'}
      <button class="btn btn-primary" onclick={() => showForm = !showForm}>
        {showForm ? 'Cancel' : '+ New Media Batch'}
      </button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      <form onsubmit={handleCreate}>
        <h3 style="margin-bottom:16px;">New Media Batch</h3>
        <div class="form-row">
          <div class="form-group">
            <label>Name *</label>
            <input type="text" bind:value={form.name} placeholder="e.g., MS Full Strength + BAP" required />
          </div>
          <div class="form-group">
            <label>Preparation Date *</label>
            <input type="date" bind:value={form.preparation_date} required />
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label>Basal Salts</label>
            <select bind:value={form.basal_salts}>
              <option value="MS">Murashige & Skoog (MS)</option>
              <option value="1/2 MS">Half-strength MS</option>
              <option value="WPM">Woody Plant Medium</option>
              <option value="B5">Gamborg's B5</option>
              <option value="N6">Chu's N6</option>
              <option value="LS">Linsmaier & Skoog</option>
              <option value="White">White's Medium</option>
              <option value="DKW">Driver & Kuniyuki</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div class="form-group">
            <label>Concentration</label>
            <input type="number" step="0.1" bind:value={form.basal_salts_concentration} />
          </div>
          <div class="form-group">
            <label>Expiration Date</label>
            <input type="date" bind:value={form.expiration_date} />
          </div>
        </div>
        <div class="form-row-3">
          <div class="form-group">
            <label>Sucrose (g/L)</label>
            <input type="number" step="0.1" bind:value={form.sucrose_g_per_l} />
          </div>
          <div class="form-group">
            <label>Agar (g/L)</label>
            <input type="number" step="0.1" bind:value={form.agar_g_per_l} />
          </div>
          <div class="form-group">
            <label>pH (pre-autoclave)</label>
            <input type="number" step="0.01" bind:value={form.ph_before_autoclave} />
          </div>
        </div>
        <div class="form-row">
          <div class="form-group">
            <label>Volume Prepared (mL)</label>
            <input type="number" bind:value={form.volume_prepared_ml} />
          </div>
          <div class="form-group">
            <label>Sterilization</label>
            <select bind:value={form.sterilization_method}>
              <option value="autoclave">Autoclave</option>
              <option value="filter">Filter Sterilization</option>
              <option value="uv">UV</option>
              <option value="other">Other</option>
            </select>
          </div>
        </div>
        <div class="form-group">
          <label>Notes</label>
          <textarea bind:value={form.notes} rows="2"></textarea>
        </div>
        <div style="text-align:right;">
          <button type="submit" class="btn btn-primary">Create Batch</button>
        </div>
      </form>
    </div>
  {/if}

  {#if loading}
    <div class="empty-state">Loading...</div>
  {:else if media.length === 0}
    <div class="empty-state">No media batches yet</div>
  {:else}
    <div class="card" style="overflow-x:auto;">
      <table>
        <thead>
          <tr>
            <th>Batch ID</th>
            <th>Name</th>
            <th>Base</th>
            <th>pH</th>
            <th>Sucrose</th>
            <th>Agar</th>
            <th>Prepared</th>
            <th>Expires</th>
            <th>Volume</th>
            <th>Status</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          {#each media as m}
            <tr>
              <td><strong>{m.batch_id}</strong></td>
              <td>{m.name}</td>
              <td>{m.basal_salts || '—'}</td>
              <td>{m.ph_before_autoclave || '—'}</td>
              <td>{m.sucrose_g_per_l ? `${m.sucrose_g_per_l}g/L` : '—'}</td>
              <td>{m.agar_g_per_l ? `${m.agar_g_per_l}g/L` : '—'}</td>
              <td>{m.preparation_date}</td>
              <td>
                {#if m.expiration_date}
                  <span class:expired={isExpired(m.expiration_date)} class:expiring={isExpiringSoon(m.expiration_date)}>
                    {m.expiration_date}
                  </span>
                {:else}
                  —
                {/if}
              </td>
              <td>{m.volume_remaining_ml != null ? `${m.volume_remaining_ml}mL` : '—'}</td>
              <td>
                {#if m.needs_review}
                  <span class="badge badge-yellow">Needs Review</span>
                {:else if isExpired(m.expiration_date)}
                  <span class="badge badge-red">Expired</span>
                {:else}
                  <span class="badge badge-green">OK</span>
                {/if}
              </td>
              <td>
                {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                  <button class="btn btn-sm btn-danger" onclick={() => handleDelete(m.id)}>Delete</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .expired { color: #dc2626; font-weight: 600; }
  .expiring { color: #d97706; font-weight: 600; }
</style>
