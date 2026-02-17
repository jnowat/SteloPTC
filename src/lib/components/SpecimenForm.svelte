<script lang="ts">
  import { onMount } from 'svelte';
  import { createSpecimen, listSpecies, listMedia } from '../api';
  import { addNotification } from '../stores/app';

  let { onclose, onsave }: { onclose: () => void; onsave: () => void } = $props();

  let species = $state<any[]>([]);
  let mediaBatches = $state<any[]>([]);
  let loading = $state(false);

  // Health status slider (0=Dead, 4=Healthy)
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
    notes: '',
  });

  const stages = [
    { value: 'explant', label: 'Explant' },
    { value: 'callus', label: 'Callus' },
    { value: 'suspension', label: 'Suspension' },
    { value: 'protoplast', label: 'Protoplast' },
    { value: 'shoot', label: 'Shoot' },
    { value: 'shoot_meristem', label: 'Shoot Meristem' },
    { value: 'root', label: 'Root' },
    { value: 'root_meristem', label: 'Root Meristem' },
    { value: 'embryogenic', label: 'Embryogenic' },
    { value: 'plantlet', label: 'Plantlet' },
    { value: 'acclimatized', label: 'Acclimatized' },
    { value: 'stock', label: 'Stock' },
  ];

  const propagationMethods = [
    { value: '', label: 'Select...' },
    { value: 'microprop', label: 'Micropropagation' },
    { value: 'somatic_embryogenesis', label: 'Somatic Embryogenesis' },
    { value: 'organogenesis', label: 'Organogenesis' },
    { value: 'meristem_culture', label: 'Meristem Culture' },
    { value: 'anther_culture', label: 'Anther Culture' },
    { value: 'protoplast_fusion', label: 'Protoplast Fusion' },
    { value: 'other', label: 'Other' },
  ];

  const rooms = ['1', '2', '3', '4', '5'];
  const racks = ['A', 'B', 'C', 'D'];
  const shelves = ['1', '2', '3', '4', '5'];
  const trays = ['A', 'B', 'C', 'D', 'E', 'F'];

  onMount(() => {
    listSpecies().then(s => species = s).catch(() => {});
    listMedia().then(m => mediaBatches = m).catch(() => {});
  });

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
        stage: form.stage,
        initiation_date: form.initiation_date,
        provenance: form.provenance || undefined,
        source_plant: form.source_plant || undefined,
        location: location || undefined,
        propagation_method: form.propagation_method || undefined,
        health_status: String(healthValue),
        notes: notes || undefined,
      });
      addNotification('Specimen created', 'success');
      onsave();
    } catch (err: any) {
      addNotification(err.message, 'error');
    } finally {
      loading = false;
    }
  }
</script>

<form onsubmit={handleSubmit}>
  <h3 style="margin-bottom:16px;">New Specimen</h3>

  <div class="form-row">
    <div class="form-group">
      <label for="species">Species *</label>
      <select id="species" bind:value={form.species_id} required>
        <option value="">Select species...</option>
        {#each species as sp}
          <option value={sp.id}>{sp.species_code} - {sp.genus} {sp.species_name}</option>
        {/each}
      </select>
    </div>
    <div class="form-group">
      <label for="stage">Stage *</label>
      <select id="stage" bind:value={form.stage}>
        {#each stages as s}
          <option value={s.value}>{s.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="form-row">
    <div class="form-group">
      <label for="init_date">Initiation Date *</label>
      <input id="init_date" type="date" bind:value={form.initiation_date} required />
    </div>
    <div class="form-group">
      <label for="prop_method">Propagation Method</label>
      <select id="prop_method" bind:value={form.propagation_method}>
        {#each propagationMethods as m}
          <option value={m.value}>{m.label}</option>
        {/each}
      </select>
    </div>
  </div>

  <!-- Location as structured dropdowns -->
  <div class="form-group">
    <label>Location</label>
    <div class="location-row">
      <div class="loc-group">
        <span class="loc-label">Room</span>
        <select bind:value={locRoom}>
          <option value="">—</option>
          {#each rooms as r}
            <option value={r}>{r}</option>
          {/each}
        </select>
      </div>
      <div class="loc-group">
        <span class="loc-label">Rack</span>
        <select bind:value={locRack}>
          <option value="">—</option>
          {#each racks as r}
            <option value={r}>{r}</option>
          {/each}
        </select>
      </div>
      <div class="loc-group">
        <span class="loc-label">Shelf</span>
        <select bind:value={locShelf}>
          <option value="">—</option>
          {#each shelves as s}
            <option value={s}>{s}</option>
          {/each}
        </select>
      </div>
      <div class="loc-group">
        <span class="loc-label">Tray</span>
        <select bind:value={locTray}>
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
    <label>Health Status</label>
    <div class="health-slider-wrap">
      <input
        type="range"
        min="0"
        max="4"
        step="1"
        bind:value={healthValue}
        class="health-slider"
        style="--track-color: {healthColors[healthValue]};"
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
    </div>
  </div>

  <!-- Media Batch -->
  <div class="form-group">
    <label for="media_batch">Initial Media Batch</label>
    <select id="media_batch" bind:value={form.media_batch_id}>
      <option value="">None / Select later...</option>
      {#each mediaBatches as mb}
        <option value={mb.id}>{mb.batch_id} – {mb.name}</option>
      {/each}
    </select>
  </div>

  <div class="form-row">
    <div class="form-group">
      <label for="provenance">Provenance / Origin</label>
      <input id="provenance" type="text" bind:value={form.provenance} placeholder="e.g., USDA germplasm collection" />
    </div>
    <div class="form-group">
      <label for="source_plant">Source Plant</label>
      <input id="source_plant" type="text" bind:value={form.source_plant} placeholder="e.g., Mother plant #12" />
    </div>
  </div>

  <div class="form-group">
    <label for="notes">Notes</label>
    <textarea id="notes" bind:value={form.notes} rows="3" placeholder="Initial observations, conditions, etc."></textarea>
  </div>

  <div style="display:flex;gap:8px;justify-content:flex-end;">
    <button type="button" class="btn" onclick={onclose}>Cancel</button>
    <button type="submit" class="btn btn-primary" disabled={loading}>
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
    text-transform: uppercase;
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
</style>
