<script lang="ts">
  import { createSpecimen, listSpecies } from '../api';
  import { addNotification } from '../stores/app';

  let { onclose, onsave }: { onclose: () => void; onsave: () => void } = $props();

  let species = $state<any[]>([]);
  let loading = $state(false);
  let form = $state({
    species_id: '',
    stage: 'explant',
    initiation_date: new Date().toISOString().split('T')[0],
    provenance: '',
    source_plant: '',
    location: '',
    propagation_method: '',
    health_status: 'healthy',
    notes: '',
  });

  const stages = ['explant', 'callus', 'suspension', 'protoplast', 'shoot', 'root', 'embryogenic', 'plantlet', 'acclimatized', 'stock'];
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

  $effect(() => {
    listSpecies().then(s => species = s).catch(() => {});
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!form.species_id) {
      addNotification('Please select a species', 'warning');
      return;
    }
    loading = true;
    try {
      await createSpecimen({
        species_id: form.species_id,
        stage: form.stage,
        initiation_date: form.initiation_date,
        provenance: form.provenance || undefined,
        source_plant: form.source_plant || undefined,
        location: form.location || undefined,
        propagation_method: form.propagation_method || undefined,
        health_status: form.health_status || undefined,
        notes: form.notes || undefined,
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
          <option value={s}>{s}</option>
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

  <div class="form-row">
    <div class="form-group">
      <label for="location">Location</label>
      <input id="location" type="text" bind:value={form.location} placeholder="e.g., Growth Room A, Shelf 3" />
    </div>
    <div class="form-group">
      <label for="health">Health Status</label>
      <input id="health" type="text" bind:value={form.health_status} placeholder="healthy" />
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
