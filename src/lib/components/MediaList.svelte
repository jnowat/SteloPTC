<script lang="ts">
  import { onMount } from 'svelte';
  import { listMedia, createMediaBatch, updateMediaBatch, deleteMediaBatch, listInventory } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let media = $state<any[]>([]);
  let inventoryItems = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let editingBatch = $state<any | null>(null);  // null = create, object = edit

  // ── Create form ─────────────────────────────────────────────────────────────
  let form = $state({
    name: '',
    preparation_date: new Date().toISOString().split('T')[0],
    expiration_date: '',
    basal_salts: localStorage.getItem('media_lastBasalSalts') || 'MS',
    basal_salts_concentration: localStorage.getItem('media_lastConc') || '1.0',
    sucrose_g_per_l: '30',
    agar_g_per_l: '8',
    ph_before_autoclave: '5.7',
    volume_prepared_ml: '',
    sterilization_method: 'autoclave',
    notes: '',
    vessels_prepared: '',
  });

  // Basal salts auto-calc fields
  let basalWeightG = $state('');      // grams of powder weighed out
  let basalWaterMl = $state('');      // total water volume (mL)
  let premadeSolution = $state(false); // toggle: pre-made vs weigh-out

  // Auto-calculate concentration when weight and volume are set
  let autoConcentration = $derived((() => {
    const w = parseFloat(basalWeightG);
    const v = parseFloat(basalWaterMl);
    if (w > 0 && v > 0) return (w / (v / 1000)).toFixed(3);
    return '';
  })());

  $effect(() => {
    if (!premadeSolution && autoConcentration) {
      form.basal_salts_concentration = autoConcentration;
    }
  });

  // ── Edit form ────────────────────────────────────────────────────────────────
  let editForm = $state({
    name: '',
    expiration_date: '',
    volume_remaining_ml: '',
    storage_conditions: '',
    qc_notes: '',
    needs_review: false,
    notes: '',
  });

  // ── Reagents ─────────────────────────────────────────────────────────────────
  let reagentRows = $state<{ item_id: string; lot_number: string; amount: string; unit: string }[]>([]);

  onMount(() => { load(); loadInventory(); });

  async function load() {
    loading = true;
    try { media = await listMedia(); }
    catch (e: any) { addNotification(e.message, 'error'); }
    finally { loading = false; }
  }

  async function loadInventory() {
    try { inventoryItems = await listInventory(); }
    catch (_e) {}
  }

  function openCreate() {
    editingBatch = null;
    basalWeightG = '';
    basalWaterMl = '';
    premadeSolution = false;
    reagentRows = [];
    form = {
      name: '',
      preparation_date: new Date().toISOString().split('T')[0],
      expiration_date: '',
      basal_salts: localStorage.getItem('media_lastBasalSalts') || 'MS',
      basal_salts_concentration: localStorage.getItem('media_lastConc') || '1.0',
      sucrose_g_per_l: '30',
      agar_g_per_l: '8',
      ph_before_autoclave: '5.7',
      volume_prepared_ml: '',
      sterilization_method: 'autoclave',
      notes: '',
      vessels_prepared: '',
    };
    showForm = true;
  }

  function openEdit(batch: any) {
    editingBatch = batch;
    editForm = {
      name: batch.name || '',
      expiration_date: batch.expiration_date || '',
      volume_remaining_ml: batch.volume_remaining_ml != null ? String(batch.volume_remaining_ml) : '',
      storage_conditions: batch.storage_conditions || '',
      qc_notes: batch.qc_notes || '',
      needs_review: batch.needs_review || false,
      notes: batch.notes || '',
    };
    showForm = true;
  }

  function cancelForm() {
    showForm = false;
    editingBatch = null;
  }

  // Reagent helpers
  function addReagentRow() {
    reagentRows = [...reagentRows, { item_id: '', lot_number: '', amount: '', unit: 'g/L' }];
  }
  function removeReagentRow(i: number) {
    reagentRows = reagentRows.filter((_, idx) => idx !== i);
  }
  function onReagentItemChange(i: number, item_id: string) {
    const inv = inventoryItems.find(it => it.id === item_id);
    reagentRows = reagentRows.map((r, idx) =>
      idx === i ? { ...r, item_id, lot_number: inv?.lot_number || '', unit: inv?.unit || 'g/L' } : r
    );
  }
  function buildReagentNotes(): string {
    const lines = reagentRows
      .filter(r => r.item_id && r.amount)
      .map(r => {
        const inv = inventoryItems.find(it => it.id === r.item_id);
        const name = inv?.name || r.item_id;
        const lot = r.lot_number ? ` [Lot: ${r.lot_number}]` : '';
        return `  ${name}${lot}: ${r.amount} ${r.unit}`;
      });
    return lines.length > 0 ? `Reagents:\n${lines.join('\n')}` : '';
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    localStorage.setItem('media_lastBasalSalts', form.basal_salts);
    localStorage.setItem('media_lastConc', form.basal_salts_concentration);

    const prefixParts: string[] = [];
    if (form.vessels_prepared) prefixParts.push(`Vessels prepared: ${form.vessels_prepared}`);
    const reagentNotes = buildReagentNotes();
    if (reagentNotes) prefixParts.push(reagentNotes);
    const combinedNotes = [...prefixParts, form.notes].filter(Boolean).join('\n');

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
        notes: combinedNotes || undefined,
      });
      addNotification('Media batch created', 'success');
      showForm = false;
      reagentRows = [];
      load();
    } catch (e: any) { addNotification(e.message, 'error'); }
  }

  async function handleUpdate(e: Event) {
    e.preventDefault();
    if (!editingBatch) return;
    try {
      await updateMediaBatch({
        id: editingBatch.id,
        name: editForm.name || undefined,
        expiration_date: editForm.expiration_date || undefined,
        volume_remaining_ml: editForm.volume_remaining_ml ? parseFloat(editForm.volume_remaining_ml) : undefined,
        storage_conditions: editForm.storage_conditions || undefined,
        qc_notes: editForm.qc_notes || undefined,
        needs_review: editForm.needs_review,
        notes: editForm.notes || undefined,
      });
      addNotification('Media batch updated', 'success');
      showForm = false;
      editingBatch = null;
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

  function parseVessels(notes: string | null): string {
    if (!notes) return '—';
    const m = notes.match(/Vessels prepared:\s*(\d+)/);
    return m ? m[1] : '—';
  }

  function canEdit(): boolean {
    const r = $currentUser?.role;
    return r === 'admin' || r === 'supervisor';
  }
</script>

<div>
  <div class="page-header">
    <h1>Media Logs</h1>
    {#if $currentUser?.role !== 'guest'}
      <button class="btn btn-primary" onclick={openCreate}>+ New Media Batch</button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      {#if editingBatch}
        <!-- ── Edit form ── -->
        <form onsubmit={handleUpdate}>
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
            <h3>Edit Batch: {editingBatch.batch_id}</h3>
            <button type="button" class="btn btn-sm" onclick={cancelForm}>Cancel</button>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label>Name</label>
              <input type="text" bind:value={editForm.name} />
            </div>
            <div class="form-group">
              <label>Expiration Date</label>
              <input type="date" bind:value={editForm.expiration_date} />
            </div>
          </div>

          <div class="compact-row">
            <div class="form-group compact-field">
              <label>Volume Remaining (mL)</label>
              <input type="number" step="0.1" bind:value={editForm.volume_remaining_ml} />
            </div>
            <div class="form-group compact-field-wide">
              <label>Storage Conditions</label>
              <input type="text" bind:value={editForm.storage_conditions} placeholder="e.g., 4°C dark" />
            </div>
          </div>

          <div class="form-group">
            <label>QC Notes</label>
            <input type="text" bind:value={editForm.qc_notes} placeholder="Quality control observations..." />
          </div>

          <div class="form-group">
            <label>Notes</label>
            <textarea bind:value={editForm.notes} rows="3"></textarea>
          </div>

          <div style="display:flex;align-items:center;gap:16px;justify-content:space-between;">
            <label style="display:inline-flex;align-items:center;gap:6px;font-size:13px;cursor:pointer;">
              <input type="checkbox" bind:checked={editForm.needs_review} style="width:auto;" />
              Flag for supervisor review
            </label>
            <button type="submit" class="btn btn-primary">Save Changes</button>
          </div>
        </form>

      {:else}
        <!-- ── Create form ── -->
        <form onsubmit={handleCreate}>
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
            <h3>Create New Media Batch</h3>
            <button type="button" class="btn btn-sm" onclick={cancelForm}>Cancel</button>
          </div>

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

          <!-- Basal salts -->
          <div class="form-row">
            <div class="form-group">
              <label>Basal Salts</label>
              <select bind:value={form.basal_salts}>
                <option value="MS">Murashige &amp; Skoog (MS)</option>
                <option value="1/2 MS">Half-strength MS</option>
                <option value="WPM">Woody Plant Medium</option>
                <option value="B5">Gamborg's B5</option>
                <option value="N6">Chu's N6</option>
                <option value="LS">Linsmaier &amp; Skoog</option>
                <option value="White">White's Medium</option>
                <option value="DKW">Driver &amp; Kuniyuki</option>
                <option value="custom">Custom</option>
              </select>
            </div>
            <div class="form-group">
              <label>Expiration Date</label>
              <input type="date" bind:value={form.expiration_date} />
            </div>
          </div>

          <!-- Concentration auto-calc -->
          <div class="conc-section">
            <div class="conc-toggle">
              <label style="display:inline-flex;align-items:center;gap:6px;cursor:pointer;">
                <input type="checkbox" bind:checked={premadeSolution} style="width:auto;" />
                Pre-made solution (enter concentration directly)
              </label>
            </div>

            {#if premadeSolution}
              <div class="form-group" style="max-width:200px;">
                <label>Basal Salts Concentration (g/L)</label>
                <input type="number" step="0.001" bind:value={form.basal_salts_concentration} placeholder="e.g., 4.4" />
              </div>
            {:else}
              <div class="compact-row">
                <div class="form-group compact-field">
                  <label>Basal Salts Added (g)</label>
                  <input type="number" step="0.001" bind:value={basalWeightG} placeholder="e.g., 4.4" />
                </div>
                <div class="form-group compact-field">
                  <label>Water Volume (mL)</label>
                  <input type="number" step="1" bind:value={basalWaterMl} placeholder="e.g., 1000" />
                </div>
                <div class="form-group compact-field">
                  <label>Concentration (g/L)</label>
                  <input
                    type="number" step="0.001"
                    bind:value={form.basal_salts_concentration}
                    placeholder={autoConcentration || '—'}
                    title={autoConcentration ? `Auto-calculated: ${autoConcentration} g/L` : 'Enter weight and volume to auto-calculate'}
                  />
                </div>
              </div>
              {#if autoConcentration}
                <div class="calc-hint">Auto-calculated: <strong>{autoConcentration} g/L</strong></div>
              {/if}
            {/if}
          </div>

          <!-- Numeric fields row -->
          <div class="compact-row">
            <div class="form-group compact-field">
              <label>Sucrose (g/L)</label>
              <input type="number" step="0.1" bind:value={form.sucrose_g_per_l} />
            </div>
            <div class="form-group compact-field">
              <label>Agar (g/L)</label>
              <input type="number" step="0.1" bind:value={form.agar_g_per_l} />
            </div>
            <div class="form-group compact-field">
              <label>pH (pre-autoclave)</label>
              <input type="number" step="0.01" bind:value={form.ph_before_autoclave} />
            </div>
            <div class="form-group compact-field">
              <label>Volume Prepared (mL)</label>
              <input type="number" bind:value={form.volume_prepared_ml} />
            </div>
            <div class="form-group compact-field">
              <label>Vessels/Jars Prepared</label>
              <input type="number" step="1" bind:value={form.vessels_prepared} placeholder="e.g., 24" />
            </div>
            <div class="form-group compact-sterilization">
              <label>Sterilization</label>
              <select bind:value={form.sterilization_method}>
                <option value="autoclave">Autoclave</option>
                <option value="filter">Filter Sterilization</option>
                <option value="uv">UV</option>
                <option value="other">Other</option>
              </select>
            </div>
          </div>

          <!-- Reagents -->
          <div class="form-group" style="margin-top:8px;">
            <label>Stock Reagents Used</label>
            {#if reagentRows.length > 0}
              <div class="reagent-table">
                <div class="reagent-header">
                  <span>Reagent</span><span>Lot #</span><span>Amount</span><span>Unit</span><span></span>
                </div>
                {#each reagentRows as row, i}
                  <div class="reagent-row">
                    <select bind:value={row.item_id} onchange={() => onReagentItemChange(i, row.item_id)}>
                      <option value="">Select reagent...</option>
                      {#each inventoryItems as inv}
                        <option value={inv.id}>{inv.name}</option>
                      {/each}
                    </select>
                    <input type="text" bind:value={row.lot_number} placeholder="Lot #" style="max-width:110px;" />
                    <input type="number" step="any" bind:value={row.amount} placeholder="0" style="max-width:90px;" />
                    <input type="text" bind:value={row.unit} placeholder="g/L" style="max-width:70px;" />
                    <button type="button" class="btn btn-sm btn-danger" onclick={() => removeReagentRow(i)}>✕</button>
                  </div>
                {/each}
              </div>
            {/if}
            <button type="button" class="btn btn-sm" onclick={addReagentRow} style="margin-top:6px;">
              + Add Reagent
            </button>
          </div>

          <div class="form-group">
            <label>Notes</label>
            <textarea bind:value={form.notes} rows="2" placeholder="QC notes, observations..."></textarea>
          </div>

          <div style="text-align:right;">
            <button type="submit" class="btn btn-primary">Create Batch</button>
          </div>
        </form>
      {/if}
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
            <th>Conc.</th>
            <th>pH</th>
            <th>Sucrose</th>
            <th>Agar</th>
            <th>Prepared</th>
            <th>Expires</th>
            <th>Vol.</th>
            <th>Jars</th>
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
              <td>{m.basal_salts_concentration != null ? `${m.basal_salts_concentration} g/L` : '—'}</td>
              <td>{m.ph_before_autoclave || '—'}</td>
              <td>{m.sucrose_g_per_l ? `${m.sucrose_g_per_l}` : '—'}</td>
              <td>{m.agar_g_per_l ? `${m.agar_g_per_l}` : '—'}</td>
              <td>{m.preparation_date}</td>
              <td>
                {#if m.expiration_date}
                  <span class:expired={isExpired(m.expiration_date)} class:expiring={isExpiringSoon(m.expiration_date)}>
                    {m.expiration_date}
                  </span>
                {:else}—{/if}
              </td>
              <td>{m.volume_remaining_ml != null ? `${m.volume_remaining_ml} mL` : '—'}</td>
              <td>{parseVessels(m.notes)}</td>
              <td>
                {#if m.needs_review}
                  <span class="badge badge-yellow">Review</span>
                {:else if isExpired(m.expiration_date)}
                  <span class="badge badge-red">Expired</span>
                {:else}
                  <span class="badge badge-green">OK</span>
                {/if}
              </td>
              <td>
                <div style="display:flex;gap:4px;">
                  {#if canEdit()}
                    <button class="btn btn-sm" onclick={() => openEdit(m)}>Edit</button>
                  {/if}
                  {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                    <button class="btn btn-sm btn-danger" onclick={() => handleDelete(m.id)}>Del</button>
                  {/if}
                </div>
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

  .compact-row {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    margin-bottom: 0;
    align-items: flex-end;
  }
  .compact-field { flex: 0 0 130px; }
  .compact-field-wide { flex: 1 1 200px; }
  .compact-sterilization { flex: 0 0 180px; }

  .conc-section {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 16px;
  }
  :global(.dark) .conc-section { background: #0f172a; border-color: #334155; }
  .conc-toggle { margin-bottom: 10px; }
  .calc-hint {
    font-size: 12px;
    color: #2563eb;
    margin-top: 4px;
    font-style: italic;
  }

  .reagent-table {
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    overflow: hidden;
  }
  .reagent-header {
    display: grid;
    grid-template-columns: 2fr 1fr 0.8fr 0.6fr 36px;
    gap: 8px;
    padding: 6px 10px;
    background: #f8fafc;
    font-size: 11px;
    font-weight: 700;
    color: #6b7280;
    border-bottom: 1px solid #e2e8f0;
  }
  .reagent-row {
    display: grid;
    grid-template-columns: 2fr 1fr 0.8fr 0.6fr 36px;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid #f1f5f9;
    align-items: center;
  }
  .reagent-row:last-child { border-bottom: none; }
</style>
