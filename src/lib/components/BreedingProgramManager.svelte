<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listBreedingPrograms,
    getBreedingProgram,
    createBreedingProgram,
    listBreedingRecordsForProgram,
    addBreedingRecord,
    getGenerationalSummary,
    type BreedingProgram,
    type BreedingRecord,
    type GenerationalSummary,
  } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import DataState from './DataState.svelte';

  const today = new Date().toISOString().split('T')[0];
  const canWrite = $derived($currentUser?.role !== 'guest');

  // Program list
  let programs = $state<BreedingProgram[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Selected program detail
  let selectedProgram = $state<BreedingProgram | null>(null);
  let records = $state<BreedingRecord[]>([]);
  let summary = $state<GenerationalSummary[]>([]);
  let detailLoading = $state(false);

  // Create program form
  let showCreateForm = $state(false);
  let createForm = $state({
    name: '',
    goal: '',
    start_date: today,
    target_traits: '',
    founder_strain_ids: '',
    notes: '',
  });
  let createSaving = $state(false);

  // Add record form
  let showAddRecordForm = $state(false);
  let recordForm = $state({
    strain_id: '',
    generation_number: '1',
    selection_notes: '',
    fitness_score: '',
    selection_date: today,
    notes: '',
  });
  let recordSaving = $state(false);

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
    try {
      programs = await listBreedingPrograms();
    } catch (e: any) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function selectProgram(program: BreedingProgram) {
    selectedProgram = program;
    detailLoading = true;
    records = [];
    summary = [];
    try {
      [records, summary] = await Promise.all([
        listBreedingRecordsForProgram(program.id),
        getGenerationalSummary(program.id),
      ]);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      detailLoading = false;
    }
  }

  async function handleCreateProgram() {
    if (!createForm.name.trim()) {
      addNotification('Program name is required', 'warning');
      return;
    }
    createSaving = true;
    try {
      const created = await createBreedingProgram({
        name: createForm.name.trim(),
        goal: createForm.goal.trim() || undefined,
        start_date: createForm.start_date || undefined,
        target_traits: createForm.target_traits.trim() || undefined,
        founder_strain_ids: createForm.founder_strain_ids.trim() || undefined,
        notes: createForm.notes.trim() || undefined,
      });
      programs = [created, ...programs];
      showCreateForm = false;
      createForm = { name: '', goal: '', start_date: today, target_traits: '', founder_strain_ids: '', notes: '' };
      addNotification('Breeding program created', 'success');
      await selectProgram(created);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      createSaving = false;
    }
  }

  async function handleAddRecord() {
    if (!selectedProgram) return;
    if (!recordForm.strain_id.trim()) {
      addNotification('Strain ID is required', 'warning');
      return;
    }
    const gen = parseInt(recordForm.generation_number, 10);
    if (isNaN(gen) || gen < 1) {
      addNotification('Generation number must be a positive integer', 'warning');
      return;
    }
    recordSaving = true;
    try {
      await addBreedingRecord({
        program_id: selectedProgram.id,
        strain_id: recordForm.strain_id.trim(),
        generation_number: gen,
        selection_notes: recordForm.selection_notes.trim() || undefined,
        fitness_score: recordForm.fitness_score ? parseFloat(recordForm.fitness_score) : undefined,
        selection_date: recordForm.selection_date || undefined,
        notes: recordForm.notes.trim() || undefined,
      });
      showAddRecordForm = false;
      recordForm = { strain_id: '', generation_number: '1', selection_notes: '', fitness_score: '', selection_date: today, notes: '' };
      addNotification('Selection record added', 'success');
      // Refresh detail data
      [records, summary] = await Promise.all([
        listBreedingRecordsForProgram(selectedProgram.id),
        getGenerationalSummary(selectedProgram.id),
      ]);
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      recordSaving = false;
    }
  }
</script>

<div class="breeding-manager">
  <div class="bm-header">
    <h2 class="bm-title">Breeding Programs</h2>
    {#if canWrite}
      <button class="btn-primary" onclick={() => (showCreateForm = !showCreateForm)}>
        {showCreateForm ? 'Cancel' : '+ New Program'}
      </button>
    {/if}
  </div>

  {#if showCreateForm}
    <div class="bm-form-card">
      <h3 class="bm-form-title">New Breeding Program</h3>
      <div class="bm-form-grid">
        <label class="bm-label">
          Program Name *
          <input class="bm-input" type="text" bind:value={createForm.name} placeholder="e.g. F1 Fragrance Selection" />
        </label>
        <label class="bm-label">
          Start Date
          <input class="bm-input" type="date" bind:value={createForm.start_date} />
        </label>
        <label class="bm-label bm-span2">
          Goal
          <input class="bm-input" type="text" bind:value={createForm.goal} placeholder="e.g. Improve disease resistance" />
        </label>
        <label class="bm-label bm-span2">
          Target Traits
          <input class="bm-input" type="text" bind:value={createForm.target_traits} placeholder='e.g. ["fragrance","yield"]' />
        </label>
        <label class="bm-label bm-span2">
          Founder Strain IDs (JSON array or comma-separated)
          <input class="bm-input" type="text" bind:value={createForm.founder_strain_ids} placeholder='["strain-uuid-1","strain-uuid-2"]' />
        </label>
        <label class="bm-label bm-span2">
          Notes
          <textarea class="bm-input bm-textarea" bind:value={createForm.notes} placeholder="Optional notes"></textarea>
        </label>
      </div>
      <div class="bm-form-actions">
        <button class="btn-primary" onclick={handleCreateProgram} disabled={createSaving}>
          {createSaving ? 'Creating…' : 'Create Program'}
        </button>
        <button class="btn-secondary" onclick={() => (showCreateForm = false)}>Cancel</button>
      </div>
    </div>
  {/if}

  <div class="bm-layout">
    <!-- Program list -->
    <div class="bm-list">
      <DataState {loading} {error} empty={!loading && programs.length === 0}
        emptyTitle="No Breeding Programs"
        emptyMessage="Create a breeding program to start tracking multi-generational selection."
        onretry={load}>
        {#each programs as prog (prog.id)}
          <button
            class="bm-program-row {selectedProgram?.id === prog.id ? 'selected' : ''}"
            onclick={() => selectProgram(prog)}
          >
            <span class="bm-prog-name">{prog.name}</span>
            {#if prog.goal}
              <span class="bm-prog-goal">{prog.goal}</span>
            {/if}
            <span class="bm-prog-date">{prog.start_date ?? 'No date'}</span>
          </button>
        {/each}
      </DataState>
    </div>

    <!-- Program detail -->
    <div class="bm-detail">
      {#if selectedProgram}
        <div class="bm-detail-header">
          <div>
            <h3 class="bm-detail-title">{selectedProgram.name}</h3>
            {#if selectedProgram.goal}
              <p class="bm-detail-goal">{selectedProgram.goal}</p>
            {/if}
            {#if selectedProgram.start_date}
              <span class="bm-chip">Started {selectedProgram.start_date}</span>
            {/if}
            {#if selectedProgram.target_traits}
              <span class="bm-chip">Traits: {selectedProgram.target_traits}</span>
            {/if}
          </div>
          {#if canWrite}
            <button class="btn-primary" onclick={() => (showAddRecordForm = !showAddRecordForm)}>
              {showAddRecordForm ? 'Cancel' : '+ Add Selection'}
            </button>
          {/if}
        </div>

        {#if showAddRecordForm}
          <div class="bm-form-card">
            <h4 class="bm-form-title">Add Selection Record</h4>
            <div class="bm-form-grid">
              <label class="bm-label">
                Strain ID *
                <input class="bm-input" type="text" bind:value={recordForm.strain_id} placeholder="UUID of the selected strain" />
              </label>
              <label class="bm-label">
                Generation #
                <input class="bm-input" type="number" min="1" bind:value={recordForm.generation_number} />
              </label>
              <label class="bm-label">
                Fitness Score
                <input class="bm-input" type="number" step="0.1" bind:value={recordForm.fitness_score} placeholder="0.0 – 10.0" />
              </label>
              <label class="bm-label">
                Selection Date
                <input class="bm-input" type="date" bind:value={recordForm.selection_date} />
              </label>
              <label class="bm-label bm-span2">
                Selection Notes
                <textarea class="bm-input bm-textarea" bind:value={recordForm.selection_notes} placeholder="Why was this strain selected?"></textarea>
              </label>
            </div>
            <div class="bm-form-actions">
              <button class="btn-primary" onclick={handleAddRecord} disabled={recordSaving}>
                {recordSaving ? 'Saving…' : 'Add Record'}
              </button>
              <button class="btn-secondary" onclick={() => (showAddRecordForm = false)}>Cancel</button>
            </div>
          </div>
        {/if}

        {#if detailLoading}
          <p class="bm-loading">Loading…</p>
        {:else}
          <!-- Generational summary table -->
          {#if summary.length > 0}
            <section class="bm-section">
              <h4 class="bm-section-title">Generational Performance</h4>
              <table class="bm-table">
                <thead>
                  <tr>
                    <th>Generation</th>
                    <th>Selections</th>
                    <th>Avg Fitness</th>
                  </tr>
                </thead>
                <tbody>
                  {#each summary as row (row.generation_number)}
                    <tr>
                      <td>F{row.generation_number}</td>
                      <td>{row.record_count}</td>
                      <td>{row.avg_fitness != null ? row.avg_fitness.toFixed(2) : '—'}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </section>
          {/if}

          <!-- Selection records -->
          <section class="bm-section">
            <h4 class="bm-section-title">Selection Records ({records.length})</h4>
            {#if records.length === 0}
              <p class="bm-empty-hint">No selection records yet. Add the first selection above.</p>
            {:else}
              <div class="bm-records">
                {#each records as rec (rec.id)}
                  <div class="bm-record-card">
                    <div class="bm-record-header">
                      <span class="bm-gen-badge">F{rec.generation_number}</span>
                      <span class="bm-strain-id">{rec.strain_id}</span>
                      {#if rec.fitness_score != null}
                        <span class="bm-fitness-badge">Fitness {rec.fitness_score.toFixed(1)}</span>
                      {/if}
                      {#if rec.selection_date}
                        <span class="bm-date">{rec.selection_date}</span>
                      {/if}
                    </div>
                    {#if rec.selection_notes}
                      <p class="bm-record-notes">{rec.selection_notes}</p>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </section>
        {/if}
      {:else}
        <div class="bm-no-selection">
          <p>Select a breeding program from the list to view details.</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .breeding-manager {
    padding: 1.5rem;
    max-width: 1200px;
    margin: 0 auto;
  }

  .bm-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.25rem;
  }

  .bm-title {
    font-size: 1.4rem;
    font-weight: 700;
    color: var(--color-text-primary, #111);
  }

  .bm-form-card {
    background: var(--color-surface, #f8f9fa);
    border: 1px solid var(--color-border, #e0e0e0);
    border-radius: 8px;
    padding: 1.25rem;
    margin-bottom: 1.25rem;
  }

  .bm-form-title {
    font-size: 1rem;
    font-weight: 600;
    margin-bottom: 1rem;
    color: var(--color-text-primary, #111);
  }

  .bm-form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .bm-label {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--color-text-secondary, #555);
  }

  .bm-span2 { grid-column: span 2; }

  .bm-input {
    padding: 0.45rem 0.6rem;
    border: 1px solid var(--color-border, #ccc);
    border-radius: 5px;
    font-size: 0.85rem;
    background: var(--color-bg, #fff);
    color: var(--color-text-primary, #111);
  }

  .bm-textarea { min-height: 70px; resize: vertical; }

  .bm-form-actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .bm-layout {
    display: grid;
    grid-template-columns: 280px 1fr;
    gap: 1.25rem;
  }

  .bm-list {
    border: 1px solid var(--color-border, #e0e0e0);
    border-radius: 8px;
    overflow: hidden;
  }

  .bm-program-row {
    display: block;
    width: 100%;
    text-align: left;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    border-bottom: 1px solid var(--color-border, #e0e0e0);
    cursor: pointer;
    transition: background 0.15s;
  }

  .bm-program-row:last-child { border-bottom: none; }

  .bm-program-row:hover { background: var(--color-surface-hover, #f0f4f8); }

  .bm-program-row.selected {
    background: var(--color-accent-light, #e8f0fe);
    border-left: 3px solid var(--color-accent, #1a73e8);
  }

  .bm-prog-name {
    display: block;
    font-weight: 600;
    font-size: 0.9rem;
    color: var(--color-text-primary, #111);
  }

  .bm-prog-goal {
    display: block;
    font-size: 0.78rem;
    color: var(--color-text-secondary, #666);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .bm-prog-date {
    display: block;
    font-size: 0.72rem;
    color: var(--color-text-tertiary, #888);
    margin-top: 0.15rem;
  }

  .bm-detail {
    border: 1px solid var(--color-border, #e0e0e0);
    border-radius: 8px;
    padding: 1.25rem;
  }

  .bm-detail-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  .bm-detail-title {
    font-size: 1.15rem;
    font-weight: 700;
    color: var(--color-text-primary, #111);
    margin-bottom: 0.25rem;
  }

  .bm-detail-goal {
    font-size: 0.85rem;
    color: var(--color-text-secondary, #555);
    margin-bottom: 0.4rem;
  }

  .bm-chip {
    display: inline-block;
    padding: 0.2rem 0.6rem;
    background: var(--color-surface, #f0f4f8);
    border: 1px solid var(--color-border, #d0d7e3);
    border-radius: 12px;
    font-size: 0.75rem;
    color: var(--color-text-secondary, #555);
    margin-right: 0.4rem;
    margin-bottom: 0.3rem;
  }

  .bm-section { margin-top: 1.25rem; }

  .bm-section-title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-text-primary, #111);
    margin-bottom: 0.6rem;
  }

  .bm-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  .bm-table th, .bm-table td {
    padding: 0.5rem 0.75rem;
    text-align: left;
    border-bottom: 1px solid var(--color-border, #e0e0e0);
  }

  .bm-table th {
    font-weight: 600;
    color: var(--color-text-secondary, #555);
    background: var(--color-surface, #f8f9fa);
  }

  .bm-records { display: flex; flex-direction: column; gap: 0.6rem; }

  .bm-record-card {
    border: 1px solid var(--color-border, #e0e0e0);
    border-radius: 6px;
    padding: 0.65rem 0.85rem;
  }

  .bm-record-header {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
    margin-bottom: 0.3rem;
  }

  .bm-gen-badge {
    background: var(--color-accent-light, #e8f0fe);
    color: var(--color-accent, #1a73e8);
    border-radius: 10px;
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .bm-strain-id {
    font-family: monospace;
    font-size: 0.78rem;
    color: var(--color-text-secondary, #555);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .bm-fitness-badge {
    background: var(--color-success-light, #e6f4ea);
    color: var(--color-success, #1e7e34);
    border-radius: 10px;
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .bm-date { font-size: 0.75rem; color: var(--color-text-tertiary, #888); }

  .bm-record-notes {
    font-size: 0.82rem;
    color: var(--color-text-secondary, #555);
    margin: 0;
  }

  .bm-no-selection {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 200px;
    color: var(--color-text-tertiary, #999);
    font-size: 0.9rem;
  }

  .bm-loading {
    color: var(--color-text-tertiary, #999);
    font-size: 0.9rem;
    padding: 1rem 0;
  }

  .bm-empty-hint {
    color: var(--color-text-tertiary, #999);
    font-size: 0.85rem;
    font-style: italic;
  }

  .btn-primary {
    padding: 0.5rem 1rem;
    background: var(--color-accent, #1a73e8);
    color: #fff;
    border: none;
    border-radius: 6px;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-primary:disabled { opacity: 0.6; cursor: not-allowed; }

  .btn-secondary {
    padding: 0.5rem 1rem;
    background: transparent;
    color: var(--color-text-secondary, #555);
    border: 1px solid var(--color-border, #ccc);
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  @media (max-width: 768px) {
    .bm-layout { grid-template-columns: 1fr; }
    .bm-form-grid { grid-template-columns: 1fr; }
    .bm-span2 { grid-column: span 1; }
  }
</style>
