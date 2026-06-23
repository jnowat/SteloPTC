<script lang="ts">
  import { onMount } from 'svelte';
  import { createSpecimen, listSpecies, listMedia, listStages, listPropagationMethods, listStrainsBySpecies } from '../api';
  import { addNotification, addErrorWithContext } from '../stores/app';
  import { effectiveHealth } from '../utils';
  import Tooltip from './Tooltip.svelte';

  let { onclose, onsave }: { onclose: () => void; onsave: () => void } = $props();

  let species = $state<any[]>([]);
  let mediaBatches = $state<any[]>([]);
  let loading = $state(false);

  // Strain selector
  let strains = $state<any[]>([]);
  let strainsLoading = $state(false);
  let selectedStrainId = $state('');

  // Health status slider (0=Dead … 4=Healthy, -1=Unknown/Awaiting)
  let healthUnknown = $state(localStorage.getItem('spec_lastHealthUnknown') === 'true');
  let healthValue = $state(parseInt(localStorage.getItem('spec_lastHealth') || '4'));
  const healthLabels = ['Dead', 'Poor', 'Fair', 'Good', 'Healthy'];
  const healthColors = ['#dc2626', '#d97706', '#ca8a04', '#65a30d', '#16a34a'];


  // Location parts
  let locRoom = $state(localStorage.getItem('spec_lastRoom') || '');
  let locRack = $state(localStorage.getItem('spec_lastRack') || '');
  let locShelf = $state(localStorage.getItem('spec_lastShelf') || '');
  let locTray = $state(localStorage.getItem('spec_lastTray') || '');

  let form = $state({
    species_id: localStorage.getItem('spec_lastSpecies') || '',
    stage: localStorage.getItem('spec_lastStage') || 'explant',
    initiation_date: new Date().toISOString().split('T')[0],
    provenance: '',
    source_plant: '',
    propagation_method: localStorage.getItem('spec_lastPropMethod') || '',
    media_batch_id: localStorage.getItem('spec_lastMediaBatch') || '',
    employee_id: '',
    notes: '',
  });

  let stages = $state<any[]>([]);
  let propagationMethods = $state<any[]>([]);

  const rooms = ['1', '2', '3', '4', '5'];
  const racks = ['A', 'B', 'C', 'D'];
  const shelves = ['1', '2', '3', '4', '5'];
  const trays = ['A', 'B', 'C', 'D', 'E', 'F'];

  onMount(() => {
    listSpecies().then(s => species = s).catch(() => {});
    listMedia().then(m => mediaBatches = m).catch(() => {});
    listStages().then(s => stages = s).catch((e: any) => addNotification(e.message, 'error'));
    listPropagationMethods().then(m => propagationMethods = m).catch((e: any) => addNotification(e.message, 'error'));
  });

  // Lazy-load strains when species changes
  $effect(() => {
    const spId = form.species_id;
    if (!spId) { strains = []; selectedStrainId = ''; return; }
    strainsLoading = true;
    listStrainsBySpecies(spId)
      .then(s => { strains = s; })
      .catch(() => { strains = []; })
      .finally(() => { strainsLoading = false; });
    selectedStrainId = '';
  });

  let selectedStrain = $derived(strains.find(s => s.id === selectedStrainId) ?? null);

  function composeLocation(): string {
    const parts: string[] = [];
    if (locRoom) parts.push(`Room ${locRoom}`);
    if (locRack) parts.push(`Rack ${locRack}`);
    if (locShelf) parts.push(`Shelf ${locShelf}`);
    if (locTray) parts.push(`Tray ${locTray}`);
    return parts.join(' / ');
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!form.species_id) {
      addNotification('Please select a species', 'warning');
      return;
    }
    loading = true;

    // Persist last-used values for auto-populate
    localStorage.setItem('spec_lastRoom', locRoom);
    localStorage.setItem('spec_lastRack', locRack);
    localStorage.setItem('spec_lastShelf', locShelf);
    localStorage.setItem('spec_lastTray', locTray);
    localStorage.setItem('spec_lastHealth', String(healthValue));
    localStorage.setItem('spec_lastHealthUnknown', String(healthUnknown));
    localStorage.setItem('spec_lastSpecies', form.species_id);
    localStorage.setItem('spec_lastStage', form.stage);
    localStorage.setItem('spec_lastPropMethod', form.propagation_method);
    localStorage.setItem('spec_lastMediaBatch', form.media_batch_id);

    const location = composeLocation();
    const mediaBatch = mediaBatches.find(m => m.id === form.media_batch_id);
    let notes = form.notes;
    if (mediaBatch) {
      const prefix = `Initial media: ${mediaBatch.batch_id} – ${mediaBatch.name}`;
      notes = notes ? `${prefix}\n${notes}` : prefix;
    }

    try {
      await createSpecimen({
        species_id: form.species_id,
        strain_id: selectedStrainId || undefined,
        stage: form.stage,
        initiation_date: form.initiation_date,
        provenance: form.provenance || undefined,
        source_plant: form.source_plant || undefined,
        location: location || undefined,
        propagation_method: form.propagation_method || undefined,
        health_status: effectiveHealth(healthValue, healthUnknown),
        employee_id: form.employee_id || undefined,
        notes: notes || undefined,
      });
      addNotification('Specimen created', 'success');
      onsave();
    } catch (err: any) {
      addErrorWithContext(
        'Failed to Create Specimen',
        err.message,
        'specimens.create',
        {
          species_id: form.species_id,
          stage: form.stage,
          initiation_date: form.initiation_date,
          propagation_method: form.propagation_method,
          health_status: effectiveHealth(healthValue, healthUnknown),
          location: composeLocation(),
          provenance: form.provenance,
          source_plant: form.source_plant,
          employee_id: form.employee_id,
          notes: form.notes,
        }
      );
    } finally {
      loading = false;
    }
  }
</script>

<form onsubmit={handleSubmit}>
  <h3 style="margin-bottom:16px;">New Specimen</h3>

  <div class="form-row">
    <div class="form-group">
      <label for="species">Species * <Tooltip text="The plant species this specimen belongs to (required)" /></label>
      <select id="species" bind:value={form.species_id} required title="Select the species for this specimen — determines the species code and taxonomy">
        <option value="">Select species...</option>
        {#each species as sp}
          <option value={sp.id}>{sp.species_code} - {sp.genus} {sp.species_name}</option>
        {/each}
      </select>
    </div>
    <div class="form-group">
      <label for="stage">Stage * <Tooltip text="Current development stage: Explant (initial tissue), Callus, Suspension, Shoot, Plantlet, etc." /></label>
      <select id="stage" bind:value={form.stage} title="Development stage: explant (initial tissue), callus (undifferentiated mass), suspension (liquid culture), shoot (organized shoot growth), plantlet (complete small plant), etc.">
        {#if stages.length === 0}
          <option value={form.stage}>{form.stage || 'Loading...'}</option>
        {:else}
          {#each stages.filter(s => !s.is_terminal) as s}
            <option value={s.code}>{s.label}</option>
          {/each}
        {/if}
      </select>
    </div>
  </div>

  <!-- Strain selector (lazy-loads after species is selected) -->
  {#if form.species_id}
    <div class="form-group">
      <label for="strain">Strain <Tooltip text="Optionally assign this specimen to a known strain. Strains are managed in the Taxonomy view." /></label>
      {#if strainsLoading}
        <select id="strain" disabled><option>Loading strains…</option></select>
      {:else}
        <select id="strain" bind:value={selectedStrainId} title="Assign this specimen to a strain — leave blank if strain is unknown">
          <option value="">No strain assigned</option>
          {#each strains as s}
            <option value={s.id}>
              {s.code} — {s.name}
              {#if s.status === 'unverified'}(Unverified){:else if s.status === 'claimed'}(Claimed){:else if s.status === 'confirmed_manual'}(⚠ Manual ID){:else if s.status === 'confirmed_genomic'}(✓ Genomic){/if}
            </option>
          {/each}
        </select>
      {/if}
      {#if selectedStrain}
        <div class="strain-status-hint strain-status-hint-{selectedStrain.status}">
          {#if selectedStrain.status === 'unverified'}
            <span class="strain-hint-icon">ℹ</span>
            This strain's identity has not been asserted yet. Consider updating its status to Claimed if you believe this is the correct strain.
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <div class="form-row">
    <div class="form-group">
      <label for="init_date">Initiation Date * <Tooltip text="The date this specimen culture was first established in vitro" /></label>
      <input id="init_date" type="date" bind:value={form.initiation_date} required title="Date this specimen was first placed into culture" />
    </div>
    <div class="form-group">
      <label for="prop_method">Propagation Method <Tooltip text="Technique used to multiply this specimen: micropropagation, somatic embryogenesis, organogenesis, meristem culture, etc." /></label>
      <select id="prop_method" bind:value={form.propagation_method} title="Select the propagation technique: micropropagation, somatic embryogenesis, organogenesis, meristem culture, etc.">
        <option value="">Select...</option>
        {#each propagationMethods as m}
          <option value={m.code}>{m.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Location as structured dropdowns -->
  <div class="form-group">
    <label>Location <Tooltip text="Physical storage location within the facility. Select Room, Rack, Shelf, and Tray to compose the full address (e.g., Room 2 / Rack B / Shelf 3 / Tray C)" /></label>
    <div class="location-row">
      <div class="loc-group">
        <span class="loc-label" title="Growth room number where the specimen is stored">Room</span>
        <select bind:value={locRoom} title="Select the growth room number where this specimen is stored">
          <option value="">—</option>
          {#each rooms as r}
            <option value={r}>{r}</option>
          {/each}
        </select>
      </div>
      <div class="loc-group">
        <span class="loc-label" title="Rack letter within the room where the specimen is stored">Rack</span>
        <select bind:value={locRack} title="Select the rack (A–D) within the room where this specimen is stored">
          <option value="">—</option>
          {#each racks as r}
            <option value={r}>{r}</option>
          {/each}
        </select>
      </div>
      <div class="loc-group">
        <span class="loc-label" title="Shelf number on the rack where the specimen is stored">Shelf</span>
        <select bind:value={locShelf} title="Select the shelf number (1–5) on the rack where this specimen is stored">
          <option value="">—</option>
          {#each shelves as s}
            <option value={s}>{s}</option>
          {/each}
        </select>
      </div>
      <div class="loc-group">
        <span class="loc-label" title="Tray position on the shelf where the specimen is stored">Tray</span>
        <select bind:value={locTray} title="Select the tray position (A–F) on the shelf where this specimen is stored">
          <option value="">—</option>
          {#each trays as t}
            <option value={t}>{t}</option>
          {/each}
        </select>
      </div>
    </div>
    {#if locRoom || locRack || locShelf || locTray}
      <div class="location-preview">{composeLocation()}</div>
    {/if}
  </div>

  <!-- Health Status slider -->
  <div class="form-group">
    <label>Health Status <Tooltip text="Current health condition: 0 = Dead, 1 = Poor, 2 = Fair, 3 = Good, 4 = Healthy. Check 'Unknown' if not yet assessed." /></label>
    <div class="health-slider-wrap">
      <label class="unknown-toggle" title="Check this if health has not yet been assessed — records health as Unknown (-1)">
        <input type="checkbox" bind:checked={healthUnknown} style="width:auto;" title="Mark health status as unknown or awaiting assessment" />
        Unknown / Awaiting Assessment
      </label>
      {#if healthUnknown}
        <div class="health-display" style="color:#7c3aed;">? – Unknown / Awaiting Assessment</div>
      {:else}
        <input
          type="range"
          id="health-slider"
          min="0"
          max="4"
          step="1"
          bind:value={healthValue}
          class="health-slider"
          style="--track-color: {healthColors[healthValue]};"
          aria-label="Health status"
          aria-valuemin="0"
          aria-valuemax="4"
          aria-valuenow={healthValue}
          aria-valuetext="{healthValue} – {healthLabels[healthValue]}"
          title="Rate specimen health from 0 (Dead) to 4 (Healthy) — current: {healthValue} ({healthLabels[healthValue]})"
        />
        <div class="health-ticks">
          {#each healthLabels as lbl, i}
            <span class="health-tick" class:active={healthValue === i} style={healthValue === i ? `color:${healthColors[i]};` : ''}>
              {i} {lbl}
            </span>
          {/each}
        </div>
        <div class="health-display" style="color:{healthColors[healthValue]};">
          {healthValue} – {healthLabels[healthValue]}
        </div>
      {/if}
    </div>
  </div>

  <!-- Media Batch -->
  <div class="form-group">
    <label for="media_batch">Initial Media Batch <Tooltip text="The nutrient media batch used for this specimen's initial culture. The batch ID will be recorded in the specimen notes for traceability." /></label>
    <select id="media_batch" bind:value={form.media_batch_id} title="Select the media batch used for this specimen's initial culture — batch ID will be recorded in the notes">
      <option value="">None / Select later...</option>
      {#each mediaBatches as mb}
        <option value={mb.id}>{mb.batch_id} – {mb.name}</option>
      {/each}
    </select>
  </div>

  <div class="form-row">
    <div class="form-group">
      <label for="provenance">Provenance / Origin <Tooltip text="Geographic or institutional origin of the plant material (e.g., USDA germplasm collection, field site, donor institution)" /></label>
      <input id="provenance" type="text" bind:value={form.provenance} placeholder="e.g., USDA germplasm collection" title="Enter the source or origin of the plant material (e.g., germplasm collection, field site, donor institution)" />
    </div>
    <div class="form-group">
      <label for="source_plant">Source Plant <Tooltip text="Identifier of the parent or mother plant this explant was taken from (e.g., Mother plant #12)" /></label>
      <input id="source_plant" type="text" bind:value={form.source_plant} placeholder="e.g., Mother plant #12" title="Enter the identifier or label of the parent plant this specimen was derived from" />
    </div>
  </div>

  <div class="form-group">
    <label for="employee_id">Employee ID / Badge # <Tooltip text="Your employee ID or badge number for traceability — recorded with the specimen for audit purposes" /></label>
    <input id="employee_id" type="text" bind:value={form.employee_id} placeholder="e.g., EMP-042" title="Enter your employee ID or badge number for traceability and audit purposes" />
  </div>

  <div class="form-group">
    <label for="notes">Notes <Tooltip text="Free-text field for initial observations, culture conditions, contamination notes, or any other relevant information" /></label>
    <textarea id="notes" bind:value={form.notes} rows="3" placeholder="Initial observations, conditions, etc." title="Add initial observations, culture conditions, contamination notes, or any other relevant information"></textarea>
  </div>

  <div style="display:flex;gap:8px;justify-content:flex-end;">
    <button type="button" class="btn" onclick={onclose} title="Discard this form and return to the specimen list">Cancel</button>
    <button type="submit" class="btn btn-primary" disabled={loading} title="Save this new specimen record to the database">
      {loading ? 'Creating...' : 'Create Specimen'}
    </button>
  </div>
</form>

<style>
  .location-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .loc-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .loc-label {
    font-size: 11px;
    font-weight: 600;
    color: #6b7280;
    letter-spacing: 0.4px;
  }
  .location-preview {
    margin-top: 6px;
    font-size: 12px;
    color: #2563eb;
    font-style: italic;
  }

  .health-slider-wrap {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .unknown-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: #7c3aed;
    cursor: pointer;
    text-transform: none;
    letter-spacing: 0;
    font-weight: 500;
  }
  .health-slider {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(to right, #dc2626, #d97706, #ca8a04, #65a30d, #16a34a);
    outline: none;
    border: none !important;
    padding: 0 !important;
    cursor: pointer;
  }
  .health-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--track-color, #16a34a);
    border: 2px solid white;
    box-shadow: 0 1px 4px rgba(0,0,0,0.3);
    cursor: pointer;
  }
  .health-slider::-moz-range-thumb {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--track-color, #16a34a);
    border: 2px solid white;
    box-shadow: 0 1px 4px rgba(0,0,0,0.3);
    cursor: pointer;
  }
  .health-ticks {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: #9ca3af;
  }
  .health-tick.active {
    font-weight: 700;
  }
  .health-display {
    font-size: 13px;
    font-weight: 700;
    margin-top: 2px;
  }

  .strain-status-hint {
    margin-top: 6px;
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 12px;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    line-height: 1.5;
  }
  .strain-status-hint-unverified {
    background: #f8fafc;
    color: #475569;
    border: 1px solid #e2e8f0;
  }
  .strain-hint-icon { flex-shrink: 0; opacity: 0.6; }
</style>
