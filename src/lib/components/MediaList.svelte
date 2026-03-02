<script lang="ts">
  import { onMount } from 'svelte';
  import { listMedia, createMediaBatch, updateMediaBatch, deleteMediaBatch, listInventory } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';

  type ReagentRow = {
    item_id: string;
    lot_number: string;
    amount_used: string;       // physical amount removed from stock (in item's native unit)
    final_concentration: string; // auto-calculated or manually entered
    final_conc_unit: string;   // unit for final concentration in media
  };

  const CONC_UNITS = ['nM', 'µM', 'mM', 'M', 'ng/mL', 'µg/mL', 'mg/mL', 'mg/L', 'g/L', '%'];

  let media = $state<any[]>([]);
  let inventoryItems = $state<any[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let editingBatch = $state<any | null>(null);  // null = create, object = edit

  // ── Create form ─────────────────────────────────────────────────────────────
  let batchForm = $state({
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
    employee_id: '',
  });

  // Keep old `form` alias for template compatibility (batchForm is canonical).
  // Use $derived so Svelte 5 tracks it reactively instead of a one-time assignment.
  let form = $derived(batchForm);

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
      batchForm.basal_salts_concentration = autoConcentration;
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
  let reagentRows = $state<ReagentRow[]>([]);

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
    batchForm = {
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
      employee_id: '',
    };
    form = batchForm;
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

  // ── Reagent helpers ───────────────────────────────────────────────────────────
  function addReagentRow() {
    reagentRows = [...reagentRows, {
      item_id: '',
      lot_number: '',
      amount_used: '',
      final_concentration: '',
      final_conc_unit: 'mg/L',
    }];
  }

  function removeReagentRow(i: number) {
    reagentRows = reagentRows.filter((_, idx) => idx !== i);
  }

  function getInventoryItem(item_id: string): any | undefined {
    return inventoryItems.find(it => it.id === item_id);
  }

  function onReagentItemChange(i: number, item_id: string) {
    const inv = getInventoryItem(item_id);
    reagentRows = reagentRows.map((r, idx) => {
      if (idx !== i) return r;
      return {
        ...r,
        item_id,
        lot_number: inv?.lot_number || '',
        amount_used: '',
        final_concentration: '',
        final_conc_unit: 'mg/L',
      };
    });
  }

  function onAmountUsedChange(i: number, value: string) {
    reagentRows = reagentRows.map((r, idx) => {
      if (idx !== i) return r;
      const inv = getInventoryItem(r.item_id);
      const isSOLID = inv?.physical_state === 'SOLID';
      let final_concentration = r.final_concentration;
      let final_conc_unit = r.final_conc_unit;

      if (isSOLID && value) {
        const amountNum = parseFloat(value);
        const volumeMl = parseFloat(batchForm.volume_prepared_ml);
        if (!isNaN(amountNum) && volumeMl > 0) {
          const unit = inv?.unit || '';
          if (unit === 'g') {
            // (g * 1000) / mL = mg/L
            final_concentration = ((amountNum * 1000) / volumeMl).toFixed(4);
          } else if (unit === 'mg') {
            // mg / mL = mg/L
            final_concentration = (amountNum / volumeMl).toFixed(4);
          } else {
            final_concentration = '';
          }
          final_conc_unit = 'mg/L';
        } else {
          final_concentration = '';
        }
      }

      return { ...r, amount_used: value, final_concentration, final_conc_unit };
    });
  }

  function isRowSolid(row: ReagentRow): boolean {
    const inv = getInventoryItem(row.item_id);
    return inv?.physical_state === 'SOLID';
  }

  function rowStockWarning(row: ReagentRow): string {
    if (!row.item_id || !row.amount_used) return '';
    const inv = getInventoryItem(row.item_id);
    if (!inv) return '';
    const used = parseFloat(row.amount_used);
    const stock = inv.current_stock;
    if (!isNaN(used) && stock != null && stock < used) {
      return `Amount exceeds current stock (${stock} ${inv.unit || ''})`;
    }
    return '';
  }

  function buildReagentNotes(): string {
    const lines = reagentRows
      .filter(r => r.item_id && r.amount_used)
      .map(r => {
        const inv = getInventoryItem(r.item_id);
        const name = inv?.name || r.item_id;
        const lot = r.lot_number ? ` [Lot: ${r.lot_number}]` : '';
        const amtStr = `${r.amount_used} ${inv?.unit || ''}`.trim();
        const concStr = r.final_concentration ? ` → ${r.final_concentration} ${r.final_conc_unit}` : '';
        return `  ${name}${lot}: ${amtStr}${concStr}`;
      });
    return lines.length > 0 ? `Reagents:\n${lines.join('\n')}` : '';
  }

  function buildHormonesArray() {
    return reagentRows
      .filter(r => r.item_id)
      .map(r => {
        const inv = getInventoryItem(r.item_id);
        return {
          item_id: r.item_id,
          lot_number: r.lot_number || undefined,
          final_concentration: r.final_concentration ? parseFloat(r.final_concentration) : undefined,
          final_conc_unit: r.final_conc_unit || undefined,
          amount_used: r.amount_used ? parseFloat(r.amount_used) : undefined,
          amount_unit: inv?.unit || undefined,
        };
      });
  }

  async function handleCreate(e: Event) {
    e.preventDefault();
    localStorage.setItem('media_lastBasalSalts', batchForm.basal_salts);
    localStorage.setItem('media_lastConc', batchForm.basal_salts_concentration);

    const prefixParts: string[] = [];
    if (batchForm.vessels_prepared) prefixParts.push(`Vessels prepared: ${batchForm.vessels_prepared}`);
    const reagentNotes = buildReagentNotes();
    if (reagentNotes) prefixParts.push(reagentNotes);
    const combinedNotes = [...prefixParts, batchForm.notes].filter(Boolean).join('\n');

    const hormones = buildHormonesArray();
    const updatedReagentCount = hormones.filter(h => h.amount_used != null).length;

    try {
      await createMediaBatch({
        name: batchForm.name,
        preparation_date: batchForm.preparation_date,
        expiration_date: batchForm.expiration_date || undefined,
        basal_salts: batchForm.basal_salts || undefined,
        basal_salts_concentration: batchForm.basal_salts_concentration ? parseFloat(batchForm.basal_salts_concentration) : undefined,
        sucrose_g_per_l: batchForm.sucrose_g_per_l ? parseFloat(batchForm.sucrose_g_per_l) : undefined,
        agar_g_per_l: batchForm.agar_g_per_l ? parseFloat(batchForm.agar_g_per_l) : undefined,
        ph_before_autoclave: batchForm.ph_before_autoclave ? parseFloat(batchForm.ph_before_autoclave) : undefined,
        volume_prepared_ml: batchForm.volume_prepared_ml ? parseFloat(batchForm.volume_prepared_ml) : undefined,
        sterilization_method: batchForm.sterilization_method || undefined,
        notes: combinedNotes || undefined,
        employee_id: batchForm.employee_id || undefined,
        hormones: hormones.length > 0 ? hormones : undefined,
      });

      let successMsg = 'Media batch created';
      if (updatedReagentCount > 0) {
        successMsg += `. Inventory stock auto-updated for ${updatedReagentCount} reagent(s).`;
      }
      addNotification(successMsg, 'success');

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
      <button
        class="btn btn-primary"
        onclick={openCreate}
        title="Open the form to log a new media preparation batch"
      >+ New Media Batch</button>
    {/if}
  </div>

  {#if showForm}
    <div class="card" style="margin-bottom:16px;">
      {#if editingBatch}
        <!-- ── Edit form ── -->
        <form onsubmit={handleUpdate}>
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
            <h3>Edit Batch: {editingBatch.batch_id}</h3>
            <button
              type="button"
              class="btn btn-sm"
              onclick={cancelForm}
              title="Discard changes and close the edit form"
            >Cancel</button>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label title="Human-readable name for this media batch (e.g., MS Full + BAP)">Name</label>
              <input
                type="text"
                bind:value={editForm.name}
                title="Human-readable name for this media batch (e.g., MS Full + BAP)"
              />
            </div>
            <div class="form-group">
              <label title="Date after which this media batch should no longer be used">Expiration Date</label>
              <input
                type="date"
                bind:value={editForm.expiration_date}
                title="Date after which this media batch should no longer be used"
              />
            </div>
          </div>

          <div class="compact-row">
            <div class="form-group compact-field">
              <label title="Current volume of media remaining in stock (mL)">Volume Remaining (mL)</label>
              <input
                type="number"
                step="0.1"
                bind:value={editForm.volume_remaining_ml}
                title="Current volume of media remaining in stock (mL)"
              />
            </div>
            <div class="form-group compact-field-wide">
              <label title="How and where this batch is being stored (e.g., 4°C in the dark)">Storage Conditions</label>
              <input
                type="text"
                bind:value={editForm.storage_conditions}
                placeholder="e.g., 4°C dark"
                title="How and where this batch is being stored (e.g., 4°C in the dark)"
              />
            </div>
          </div>

          <div class="form-group">
            <label title="Quality control observations — contamination checks, colour, clarity, gelation">QC Notes</label>
            <input
              type="text"
              bind:value={editForm.qc_notes}
              placeholder="Quality control observations..."
              title="Quality control observations — contamination checks, colour, clarity, gelation"
            />
          </div>

          <div class="form-group">
            <label title="General notes about this batch — amendments, deviations from protocol, etc.">Notes</label>
            <textarea
              bind:value={editForm.notes}
              rows="3"
              title="General notes about this batch — amendments, deviations from protocol, etc."
            ></textarea>
          </div>

          <div style="display:flex;align-items:center;gap:16px;justify-content:space-between;">
            <label
              style="display:inline-flex;align-items:center;gap:6px;font-size:13px;cursor:pointer;"
              title="Check this box to flag the batch for supervisor review before use"
            >
              <input
                type="checkbox"
                bind:checked={editForm.needs_review}
                style="width:auto;"
                title="Flag this batch so a supervisor is alerted to review it before use"
              />
              Flag for supervisor review
            </label>
            <button
              type="submit"
              class="btn btn-primary"
              title="Save all changes made to this media batch record"
            >Save Changes</button>
          </div>
        </form>

      {:else}
        <!-- ── Create form ── -->
        <form onsubmit={handleCreate}>
          <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:16px;">
            <h3>Create New Media Batch</h3>
            <button
              type="button"
              class="btn btn-sm"
              onclick={cancelForm}
              title="Discard this form and return to the media list"
            >Cancel</button>
          </div>

          <div class="form-row">
            <div class="form-group">
              <label title="Descriptive name for this batch — include basal salts type and key supplements">Name *</label>
              <input
                type="text"
                bind:value={batchForm.name}
                placeholder="e.g., MS Full Strength + BAP"
                required
                title="Descriptive name for this batch — include basal salts type and key supplements"
              />
            </div>
            <div class="form-group">
              <label title="Date on which this media batch was prepared and autoclaved">Preparation Date *</label>
              <input
                type="date"
                bind:value={batchForm.preparation_date}
                required
                title="Date on which this media batch was prepared and autoclaved"
              />
            </div>
          </div>

          <!-- Employee ID -->
          <div class="form-row">
            <div class="form-group" style="max-width:260px;">
              <label title="ID or badge number of the technician who prepared this batch — used for traceability">Employee ID / Badge #</label>
              <input
                type="text"
                bind:value={batchForm.employee_id}
                placeholder="e.g., EMP-042"
                title="ID or badge number of the technician who prepared this batch — used for traceability"
              />
            </div>
          </div>

          <!-- Basal salts -->
          <div class="form-row">
            <div class="form-group">
              <label title="Type of basal medium formulation (MS=Murashige &amp; Skoog, WPM=Woody Plant Medium, B5=Gamborg's B5, etc.)">Basal Salts</label>
              <select
                bind:value={batchForm.basal_salts}
                title="Type of basal medium formulation (MS=Murashige &amp; Skoog, WPM=Woody Plant Medium, B5=Gamborg's B5, etc.)"
              >
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
              <label title="Date after which this media batch should no longer be used">Expiration Date</label>
              <input
                type="date"
                bind:value={batchForm.expiration_date}
                title="Date after which this media batch should no longer be used"
              />
            </div>
          </div>

          <!-- Concentration auto-calc -->
          <div class="conc-section">
            <div class="conc-toggle">
              <label
                style="display:inline-flex;align-items:center;gap:6px;cursor:pointer;"
                title="Toggle to use a pre-made stock solution instead of weighing out powder directly"
              >
                <input
                  type="checkbox"
                  bind:checked={premadeSolution}
                  style="width:auto;"
                  title="Toggle to use a stock solution instead of weighing powder directly — when checked, enter the known concentration; when unchecked, enter weight and volume to auto-calculate"
                />
                Pre-made solution (enter concentration directly)
              </label>
            </div>

            {#if premadeSolution}
              <div class="form-group" style="max-width:200px;">
                <label title="Known concentration of the pre-made basal salts stock solution in g/L">Basal Salts Concentration (g/L)</label>
                <input
                  type="number"
                  step="0.001"
                  bind:value={batchForm.basal_salts_concentration}
                  placeholder="e.g., 4.4"
                  title="Known concentration of the pre-made basal salts stock solution in g/L (e.g., 4.4 g/L for full-strength MS)"
                />
              </div>
            {:else}
              <div class="compact-row">
                <div class="form-group compact-field">
                  <label title="Mass of basal salts powder weighed out for this batch (grams)">Basal Salts Added (g)</label>
                  <input
                    type="number"
                    step="0.001"
                    bind:value={basalWeightG}
                    placeholder="e.g., 4.4"
                    title="Mass of basal salts powder weighed out for this batch (grams) — used to auto-calculate concentration"
                  />
                </div>
                <div class="form-group compact-field">
                  <label title="Total water volume used to dissolve the basal salts (mL) — used to auto-calculate concentration">Water Volume (mL)</label>
                  <input
                    type="number"
                    step="1"
                    bind:value={basalWaterMl}
                    placeholder="e.g., 1000"
                    title="Total water volume used to dissolve the basal salts (mL) — used to auto-calculate concentration"
                  />
                </div>
                <div class="form-group compact-field">
                  <label title="Resulting basal salts concentration in g/L — auto-calculated from weight and volume, or enter manually">Concentration (g/L)</label>
                  <input
                    type="number" step="0.001"
                    bind:value={batchForm.basal_salts_concentration}
                    placeholder={autoConcentration || '—'}
                    title={autoConcentration ? `Auto-calculated: ${autoConcentration} g/L` : 'Enter weight and volume to auto-calculate, or type a value manually'}
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
              <label title="Carbon source concentration in g/L (typical 20–30 g/L for most plant tissue culture media)">Sucrose (g/L)</label>
              <input
                type="number"
                step="0.1"
                bind:value={batchForm.sucrose_g_per_l}
                title="Carbon source (sucrose) concentration in g/L — typical range 20–30 g/L; provides energy for non-photosynthetic explants"
              />
            </div>
            <div class="form-group compact-field">
              <label title="Agar concentration in g/L — controls gel firmness (6–8 g/L solid; 0 for liquid/suspension media)">Agar (g/L)</label>
              <input
                type="number"
                step="0.1"
                bind:value={batchForm.agar_g_per_l}
                title="Agar concentration in g/L — typical 6–8 g/L for solid media; set to 0 for liquid or suspension cultures"
              />
            </div>
            <div class="form-group compact-field">
              <label title="pH measured before autoclave sterilization — typically adjusted to 5.5–6.0 for most plant tissue culture media">pH (pre-autoclave)</label>
              <input
                type="number"
                step="0.01"
                bind:value={batchForm.ph_before_autoclave}
                title="pH measured before autoclave sterilization — typical range 5.5–6.0; adjust with KOH or HCl before autoclaving"
              />
            </div>
            <div class="form-group compact-field">
              <label title="Total volume of media prepared in this batch (mL) — used to calculate reagent concentrations">Volume Prepared (mL)</label>
              <input
                type="number"
                bind:value={batchForm.volume_prepared_ml}
                title="Total volume of media prepared in this batch (mL) — used to auto-calculate reagent final concentrations"
              />
            </div>
            <div class="form-group compact-field">
              <label title="Number of culture vessels (jars, plates, tubes) filled from this batch">Vessels/Jars Prepared</label>
              <input
                type="number"
                step="1"
                bind:value={batchForm.vessels_prepared}
                placeholder="e.g., 24"
                title="Number of culture vessels (jars, Magenta boxes, Petri dishes, tubes) filled from this batch"
              />
            </div>
            <div class="form-group compact-sterilization">
              <label title="Method used to sterilize this media batch">Sterilization</label>
              <select
                bind:value={batchForm.sterilization_method}
                title="Sterilization method — autoclave (121°C, 15 psi, 20 min) is standard; filter sterilization (0.22 µm) for heat-labile additives"
              >
                <option value="autoclave">Autoclave</option>
                <option value="filter">Filter Sterilization</option>
                <option value="uv">UV</option>
                <option value="other">Other</option>
              </select>
            </div>
          </div>

          <!-- Reagents -->
          <div class="form-group" style="margin-top:8px;">
            <label title="Hormones, growth regulators, vitamins, or other stock reagents added to this media batch">Stock Reagents Used</label>
            {#if reagentRows.length > 0}
              <div class="reagent-table">
                <div class="reagent-header">
                  <span title="Inventory item used as a reagent in this media batch">Reagent</span>
                  <span title="Physical state of the reagent — SOLID (powder) or LIQUID (solution)">State</span>
                  <span title="Lot number of the reagent used — for traceability and QC">Lot #</span>
                  <span title="Physical amount of reagent taken from stock (in the item's native unit, e.g., g or mL)">Amount Used</span>
                  <span title="Final concentration of this reagent in the prepared media — auto-calculated for solids, manual for liquids">Final Conc.</span>
                  <span title="Unit for the final concentration of this reagent in the media (e.g., mg/L, µM)">Unit</span>
                  <span></span>
                </div>
                {#each reagentRows as row, i}
                  {@const inv = getInventoryItem(row.item_id)}
                  {@const solid = inv?.physical_state === 'SOLID'}
                  {@const warning = rowStockWarning(row)}
                  <div class="reagent-row-wrap">
                    <div class="reagent-row">
                      <!-- Reagent selector -->
                      <select
                        bind:value={row.item_id}
                        onchange={() => onReagentItemChange(i, row.item_id)}
                        title="Select the inventory reagent added to this media batch — stock will be deducted automatically on save"
                      >
                        <option value="">Select reagent...</option>
                        {#each inventoryItems as inv}
                          <option value={inv.id}>{inv.name}</option>
                        {/each}
                      </select>

                      <!-- Physical state badge -->
                      <div class="state-cell">
                        {#if row.item_id && inv}
                          <span
                            class="badge {solid ? 'badge-blue' : 'badge-teal'}"
                            title={solid ? 'Solid — dry powder; concentration is auto-calculated from mass and volume' : 'Liquid — prepared solution; enter the volume added and concentration manually'}
                          >
                            {inv.physical_state || '—'}
                          </span>
                        {:else}
                          <span class="badge-empty" title="Select a reagent to see its physical state (Solid or Liquid)">—</span>
                        {/if}
                      </div>

                      <!-- Lot number -->
                      <input
                        type="text"
                        bind:value={row.lot_number}
                        placeholder="Lot #"
                        style="max-width:110px;"
                        title="Lot number of the reagent container used — auto-filled from inventory record; edit if using a different lot"
                      />

                      <!-- Amount used -->
                      <div class="amount-cell">
                        {#if solid}
                          <input
                            type="number"
                            step="any"
                            value={row.amount_used}
                            oninput={(e) => onAmountUsedChange(i, (e.target as HTMLInputElement).value)}
                            placeholder="0"
                            style="max-width:90px;"
                            title="Mass of solid reagent weighed out for this batch — this amount will be deducted from inventory stock on save"
                          />
                          {#if inv?.unit}
                            <span class="unit-label" title="Unit for the amount of this reagent used (as recorded in inventory)">{inv.unit}</span>
                          {/if}
                        {:else if row.item_id}
                          <!-- Liquid: volume added -->
                          <input
                            type="number"
                            step="any"
                            value={row.amount_used}
                            oninput={(e) => onAmountUsedChange(i, (e.target as HTMLInputElement).value)}
                            placeholder="0"
                            style="max-width:90px;"
                            title="Volume of liquid stock solution added to this media batch — this amount will be deducted from inventory stock on save"
                          />
                          {#if inv?.unit}
                            <span class="unit-label" title="Unit for the volume of liquid reagent used (as recorded in inventory)">{inv.unit}</span>
                          {/if}
                        {:else}
                          <input type="number" step="any" placeholder="0" style="max-width:90px;" disabled title="Select a reagent first to enter the amount used" />
                        {/if}
                      </div>

                      <!-- Final concentration -->
                      <div class="conc-cell">
                        {#if solid}
                          <!-- Auto-calculated; show read-only -->
                          <input
                            type="number"
                            step="any"
                            bind:value={row.final_concentration}
                            placeholder="auto"
                            style="max-width:100px;"
                            title="Auto-calculated from amount used and volume prepared — formula: (mass in mg) / (volume in L) = mg/L; edit to override"
                          />
                        {:else if row.item_id}
                          <!-- Liquid: manual entry -->
                          <input
                            type="number"
                            step="any"
                            bind:value={row.final_concentration}
                            placeholder="0"
                            style="max-width:100px;"
                            title="Final concentration of this liquid reagent in the prepared media — calculate based on stock concentration and dilution factor"
                          />
                        {:else}
                          <input type="number" step="any" placeholder="0" style="max-width:100px;" disabled title="Select a reagent first to enter the final concentration" />
                        {/if}
                      </div>

                      <!-- Concentration unit -->
                      <div class="unit-cell">
                        {#if solid}
                          <!-- Fixed mg/L for solids by default, still allow override -->
                          <select
                            bind:value={row.final_conc_unit}
                            style="max-width:90px;"
                            title="Unit for the final concentration of this solid reagent in the media — defaults to mg/L (auto-calculated); change if needed"
                          >
                            {#each CONC_UNITS as u}
                              <option value={u}>{u}</option>
                            {/each}
                          </select>
                        {:else if row.item_id}
                          <select
                            bind:value={row.final_conc_unit}
                            style="max-width:90px;"
                            title="Unit for the final concentration of this liquid reagent in the media (e.g., µM for hormones, mg/L for vitamins)"
                          >
                            {#each CONC_UNITS as u}
                              <option value={u}>{u}</option>
                            {/each}
                          </select>
                        {:else}
                          <select style="max-width:90px;" disabled title="Select a reagent first to choose a concentration unit">
                            <option>mg/L</option>
                          </select>
                        {/if}
                      </div>

                      <!-- Remove -->
                      <button
                        type="button"
                        class="btn btn-sm btn-danger"
                        onclick={() => removeReagentRow(i)}
                        title="Remove this reagent from the batch — no stock deduction will be made for this row"
                      >✕</button>
                    </div>

                    <!-- Per-row stock warning -->
                    {#if warning}
                      <div class="stock-warning" title="Warning: the amount entered exceeds the current inventory stock level for this reagent">⚠ {warning}</div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
            <button
              type="button"
              class="btn btn-sm"
              onclick={addReagentRow}
              style="margin-top:6px;"
              title="Add a new row to record an additional reagent used in this media batch"
            >
              + Add Reagent
            </button>
          </div>

          <div class="form-group">
            <label title="Free-text notes about this batch — QC observations, deviations, contamination risk, etc.">Notes</label>
            <textarea
              bind:value={batchForm.notes}
              rows="2"
              placeholder="QC notes, observations..."
              title="Free-text notes about this batch — QC observations, deviations from protocol, contamination risk, etc."
            ></textarea>
          </div>

          <div style="text-align:right;">
            <button
              type="submit"
              class="btn btn-primary"
              title="Save this media batch record and auto-deduct reagent amounts from inventory stock"
            >Create Batch</button>
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
            <th title="Unique system-generated identifier for this media batch">Batch ID</th>
            <th title="Descriptive name of the media formulation (e.g., MS Full + BAP 0.5 mg/L)">Name</th>
            <th title="Base basal salts formulation used (e.g., MS = Murashige &amp; Skoog, WPM = Woody Plant Medium)">Base</th>
            <th title="Concentration of basal salts in g/L (e.g., 4.4 g/L for full-strength MS)">Conc.</th>
            <th title="pH measured before autoclave sterilization — typical range 5.5–6.0">pH</th>
            <th title="Sucrose concentration in g/L — carbon source for non-photosynthetic explants (typical 20–30 g/L)">Sucrose</th>
            <th title="Agar concentration in g/L — controls gel firmness (6–8 g/L solid media; 0 for liquid)">Agar</th>
            <th title="Date this media batch was prepared and autoclaved">Prepared</th>
            <th title="Date after which this media batch should no longer be used">Expires</th>
            <th title="Volume of media remaining in this batch (mL)">Vol.</th>
            <th title="Number of culture vessels (jars, Magenta boxes, Petri dishes) filled from this batch">Jars</th>
            <th title="Current status of this media batch: OK (usable), Expired (past expiration date), or Review (flagged for supervisor review)">Status</th>
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
                  <span
                    class:expired={isExpired(m.expiration_date)}
                    class:expiring={isExpiringSoon(m.expiration_date)}
                    title={isExpired(m.expiration_date) ? 'This batch has passed its expiration date and should not be used' : isExpiringSoon(m.expiration_date) ? 'This batch expires within 7 days — use soon or discard' : `Expires on ${m.expiration_date}`}
                  >
                    {m.expiration_date}
                  </span>
                {:else}—{/if}
              </td>
              <td title="Volume of media remaining in this batch (mL)">{m.volume_remaining_ml != null ? `${m.volume_remaining_ml} mL` : '—'}</td>
              <td title="Number of culture vessels filled from this batch (parsed from batch notes)">{parseVessels(m.notes)}</td>
              <td>
                {#if m.needs_review}
                  <span class="badge badge-yellow" title="This batch has been flagged for supervisor review before use">Review</span>
                {:else if isExpired(m.expiration_date)}
                  <span class="badge badge-red" title="This media batch has passed its expiration date and should not be used">Expired</span>
                {:else}
                  <span class="badge badge-green" title="This media batch is within its expiration date and has not been flagged for review">OK</span>
                {/if}
              </td>
              <td>
                <div style="display:flex;gap:4px;">
                  {#if canEdit()}
                    <button
                      class="btn btn-sm"
                      onclick={() => openEdit(m)}
                      title="Edit this media batch — update volume remaining, storage conditions, QC notes, or flag for review"
                    >Edit</button>
                  {/if}
                  {#if $currentUser?.role === 'admin' || $currentUser?.role === 'supervisor'}
                    <button
                      class="btn btn-sm btn-danger"
                      onclick={() => handleDelete(m.id)}
                      title="Permanently delete this media batch record — this action cannot be undone"
                    >Del</button>
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

  /* Reagent table */
  .reagent-table {
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    overflow: hidden;
  }
  :global(.dark) .reagent-table { border-color: #334155; }

  .reagent-header {
    display: grid;
    grid-template-columns: 2fr 0.6fr 0.9fr 1fr 1fr 0.8fr 36px;
    gap: 8px;
    padding: 6px 10px;
    background: #f8fafc;
    font-size: 11px;
    font-weight: 700;
    color: #6b7280;
    border-bottom: 1px solid #e2e8f0;
  }
  :global(.dark) .reagent-header {
    background: #1e293b;
    border-color: #334155;
    color: #94a3b8;
  }

  .reagent-row-wrap {
    border-bottom: 1px solid #f1f5f9;
  }
  .reagent-row-wrap:last-child { border-bottom: none; }

  .reagent-row {
    display: grid;
    grid-template-columns: 2fr 0.6fr 0.9fr 1fr 1fr 0.8fr 36px;
    gap: 8px;
    padding: 6px 10px;
    align-items: center;
  }
  :global(.dark) .reagent-row-wrap { border-color: #1e293b; }

  .state-cell {
    display: flex;
    align-items: center;
  }
  .badge-empty {
    font-size: 12px;
    color: #9ca3af;
  }

  .amount-cell,
  .conc-cell,
  .unit-cell {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .unit-label {
    font-size: 12px;
    color: #6b7280;
    white-space: nowrap;
  }

  /* State badges */
  .badge-blue {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 9999px;
    font-size: 10px;
    font-weight: 700;
    background: #dbeafe;
    color: #1d4ed8;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  :global(.dark) .badge-blue { background: #1e3a5f; color: #93c5fd; }

  .badge-teal {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 9999px;
    font-size: 10px;
    font-weight: 700;
    background: #ccfbf1;
    color: #0f766e;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  :global(.dark) .badge-teal { background: #134e4a; color: #5eead4; }

  /* Stock warning */
  .stock-warning {
    padding: 3px 10px 5px;
    font-size: 11px;
    color: #92400e;
    background: #fffbeb;
    border-top: 1px solid #fde68a;
  }
  :global(.dark) .stock-warning {
    background: #2d1f06;
    color: #fcd34d;
    border-color: #78350f;
  }
</style>
