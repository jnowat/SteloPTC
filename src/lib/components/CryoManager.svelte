<script lang="ts">
  import { onMount } from 'svelte';
  import {
    listFrozenVials, createFrozenVial, thawVial, discardFrozenVial,
    type FrozenVial, type ThawVialResult
  } from '../api';
  import { addNotification } from '../stores/app';
  import { currentUser } from '../stores/auth';
  import DataState from './DataState.svelte';

  const today = new Date().toISOString().split('T')[0];

  let vials = $state<FrozenVial[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Filter state
  let filterStatus = $state<string>('active');
  let filterFreezer = $state('');

  // New-vial form
  let showAddForm = $state(false);
  let addForm = $state({
    species_id: '',
    passage_number: '',
    cumulative_pdl: '',
    vial_count: '1',
    freeze_date: today,
    freeze_medium: '10% DMSO in complete medium',
    location_freezer: '',
    location_tower: '',
    location_box: '',
    location_position: '',
    notes: '',
  });
  let addSpeciesText = $state('');
  let addSaving = $state(false);

  // Thaw modal
  let thawingVial = $state<FrozenVial | null>(null);
  let thawForm = $state({
    thaw_date: today,
    vials_to_thaw: '1',
    location: '',
    notes: '',
    employee_id: '',
  });
  let thawResult = $state<ThawVialResult | null>(null);
  let thawSaving = $state(false);

  // Discard confirmation
  let discardingVial = $state<FrozenVial | null>(null);
  let discardNotes = $state('');
  let discardSaving = $state(false);

  onMount(() => { load(); });

  async function load() {
    loading = true;
    error = null;
    try {
      const params: Record<string, string | null> = {
        status: filterStatus || null,
        location_freezer: filterFreezer || null,
      };
      vials = await listFrozenVials(params);
    } catch (e: any) {
      error = e.message;
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function composeCryoLocation(freezer: string, tower: string, box: string, pos: string): string {
    const parts: string[] = [];
    if (freezer) parts.push(`Freezer ${freezer}`);
    if (tower) parts.push(`Tower ${tower}`);
    if (box) parts.push(`Box ${box}`);
    if (pos) parts.push(`Position ${pos}`);
    return parts.join(' / ');
  }

  function resetAddForm() {
    addForm = {
      species_id: '',
      passage_number: '',
      cumulative_pdl: '',
      vial_count: '1',
      freeze_date: today,
      freeze_medium: '10% DMSO in complete medium',
      location_freezer: localStorage.getItem('cryo_lastFreezer') || '',
      location_tower: localStorage.getItem('cryo_lastTower') || '',
      location_box: localStorage.getItem('cryo_lastBox') || '',
      location_position: '',
      notes: '',
    };
    addSpeciesText = '';
    showAddForm = false;
  }

  async function handleAdd(e: Event) {
    e.preventDefault();
    const count = parseInt(addForm.vial_count);
    if (!addForm.species_id || !addForm.freeze_date || !addForm.freeze_medium || count < 1) {
      addNotification('Please fill in all required fields and enter a valid vial count.', 'error');
      return;
    }
    addSaving = true;
    try {
      localStorage.setItem('cryo_lastFreezer', addForm.location_freezer);
      localStorage.setItem('cryo_lastTower', addForm.location_tower);
      localStorage.setItem('cryo_lastBox', addForm.location_box);
      await createFrozenVial({
        species_id: addForm.species_id,
        passage_number: parseInt(addForm.passage_number) || 0,
        cumulative_pdl: addForm.cumulative_pdl ? parseFloat(addForm.cumulative_pdl) : null,
        vial_count: count,
        freeze_date: addForm.freeze_date,
        freeze_medium: addForm.freeze_medium,
        location_freezer: addForm.location_freezer || null,
        location_tower: addForm.location_tower || null,
        location_box: addForm.location_box || null,
        location_position: addForm.location_position || null,
        notes: addForm.notes || null,
      });
      addNotification('Frozen vial lot recorded.', 'success');
      resetAddForm();
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      addSaving = false;
    }
  }

  function openThaw(vial: FrozenVial) {
    thawingVial = vial;
    thawForm = {
      thaw_date: today,
      vials_to_thaw: '1',
      location: '',
      notes: '',
      employee_id: '',
    };
    thawResult = null;
  }

  async function handleThaw(e: Event) {
    e.preventDefault();
    if (!thawingVial) return;
    const count = parseInt(thawForm.vials_to_thaw) || 1;
    thawSaving = true;
    try {
      const result = await thawVial({
        vial_id: thawingVial.id,
        thaw_date: thawForm.thaw_date,
        vials_to_thaw: count,
        location: thawForm.location || null,
        notes: thawForm.notes || null,
        employee_id: thawForm.employee_id || null,
      });
      thawResult = result;
      addNotification(`Thawed ${count} vial(s). New specimen: ${result.new_specimen_accession}`, 'success');
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      thawSaving = false;
    }
  }

  function openDiscard(vial: FrozenVial) {
    discardingVial = vial;
    discardNotes = '';
  }

  async function handleDiscard() {
    if (!discardingVial) return;
    discardSaving = true;
    try {
      await discardFrozenVial({ vial_id: discardingVial.id, notes: discardNotes || null });
      addNotification('Vial lot marked as discarded.', 'success');
      discardingVial = null;
      await load();
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      discardSaving = false;
    }
  }

  function statusBadgeClass(status: string): string {
    if (status === 'active') return 'badge-active';
    if (status === 'depleted') return 'badge-depleted';
    return 'badge-discarded';
  }


</script>

<div class="cryo-manager">
  <div class="page-header">
    <div class="header-left">
      <h2>Cryopreservation Inventory</h2>
      <p class="subtitle">Manage frozen vial lots in LN₂ and −80°C storage.</p>
    </div>
    <button class="btn btn-primary" onclick={() => { showAddForm = true; }}>
      + Record Vials
    </button>
  </div>

  <!-- Filter bar -->
  <div class="filter-bar">
    <label>
      Status
      <select bind:value={filterStatus} onchange={() => load()}>
        <option value="">All</option>
        <option value="active">Active</option>
        <option value="depleted">Depleted</option>
        <option value="discarded">Discarded</option>
      </select>
    </label>
    <label>
      Freezer
      <input
        type="text"
        placeholder="e.g. LN2-A"
        bind:value={filterFreezer}
        onchange={() => load()}
      />
    </label>
    <button class="btn btn-ghost" onclick={() => load()}>Refresh</button>
  </div>

  <!-- Vial list -->
  <DataState
    {loading}
    {error}
    empty={!loading && vials.length === 0}
    emptyTitle="No vials found"
    emptyMessage="Use 'Record Vials' to log a new frozen lot."
    onretry={load}
  >
    <div class="vial-table-wrap">
      <table class="vial-table">
        <thead>
          <tr>
            <th>Species / Cell Line</th>
            <th>Passage</th>
            <th>PDL</th>
            <th>Vials</th>
            <th>Freeze Date</th>
            <th>Freeze Medium</th>
            <th>Location</th>
            <th>Status</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          {#each vials as vial (vial.id)}
            <tr class="vial-row" class:row-depleted={vial.status === 'depleted'} class:row-discarded={vial.status === 'discarded'}>
              <td>
                <span class="species-code">{vial.species_code ?? vial.species_id}</span>
                {#if vial.species_name}
                  <span class="species-name">{vial.species_name}</span>
                {/if}
              </td>
              <td class="num-cell">P{vial.passage_number}</td>
              <td class="num-cell">{vial.cumulative_pdl != null ? vial.cumulative_pdl.toFixed(1) : '—'}</td>
              <td class="num-cell vial-count" class:count-low={vial.vial_count <= 2 && vial.status === 'active'}>
                {vial.vial_count}
              </td>
              <td>{vial.freeze_date}</td>
              <td class="medium-cell" title={vial.freeze_medium}>{vial.freeze_medium}</td>
              <td class="location-cell">{vial.location ?? '—'}</td>
              <td><span class="badge {statusBadgeClass(vial.status)}">{vial.status}</span></td>
              <td class="actions-cell">
                {#if vial.status === 'active'}
                  <button class="btn btn-sm btn-primary" onclick={() => openThaw(vial)}>Thaw</button>
                  <button class="btn btn-sm btn-danger" onclick={() => openDiscard(vial)}>Discard</button>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </DataState>
</div>

<!-- Add vial form modal -->
{#if showAddForm}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Record frozen vials">
    <div class="modal">
      <div class="modal-header">
        <h3>Record Frozen Vials</h3>
        <button class="modal-close" onclick={resetAddForm} aria-label="Close">&#10005;</button>
      </div>
      <form onsubmit={handleAdd}>
        <div class="form-grid">
          <label class="required">
            Species ID
            <input
              type="text"
              bind:value={addForm.species_id}
              placeholder="Species UUID"
              required
            />
          </label>

          <label class="required">
            Freeze Date
            <input type="date" bind:value={addForm.freeze_date} required />
          </label>

          <label class="required">
            Vial Count
            <input
              type="number"
              min="1"
              bind:value={addForm.vial_count}
              required
            />
          </label>

          <label>
            Passage Number
            <input type="number" min="0" bind:value={addForm.passage_number} placeholder="0" />
          </label>

          <label>
            Cumulative PDL
            <input type="number" step="0.1" bind:value={addForm.cumulative_pdl} placeholder="optional" />
          </label>

          <label class="full-width required">
            Freeze Medium
            <input
              type="text"
              bind:value={addForm.freeze_medium}
              placeholder="e.g. 10% DMSO in complete medium"
              required
            />
          </label>

          <div class="full-width">
            <span class="field-label">Location (Freezer / Tower / Box / Position)</span>
            <div class="location-row">
              <input
                type="text"
                placeholder="Freezer"
                bind:value={addForm.location_freezer}
                title="Freezer identifier, e.g. LN2-A or -80C-1"
              />
              <input
                type="text"
                placeholder="Tower"
                bind:value={addForm.location_tower}
                title="Tower or rack within the freezer"
              />
              <input
                type="text"
                placeholder="Box"
                bind:value={addForm.location_box}
                title="Box or cane within the tower"
              />
              <input
                type="text"
                placeholder="Position"
                bind:value={addForm.location_position}
                title="Well/cell position within the box (e.g. A1)"
              />
            </div>
            {#if addForm.location_freezer || addForm.location_tower || addForm.location_box || addForm.location_position}
              <div class="location-preview">
                {composeCryoLocation(addForm.location_freezer, addForm.location_tower, addForm.location_box, addForm.location_position)}
              </div>
            {/if}
          </div>

          <label class="full-width">
            Notes
            <textarea bind:value={addForm.notes} rows="2" placeholder="Optional notes"></textarea>
          </label>
        </div>

        <div class="modal-footer">
          <button type="button" class="btn btn-ghost" onclick={resetAddForm}>Cancel</button>
          <button type="submit" class="btn btn-primary" disabled={addSaving}>
            {addSaving ? 'Saving…' : 'Record Vials'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Thaw modal -->
{#if thawingVial}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Thaw vials">
    <div class="modal">
      <div class="modal-header">
        <h3>Thaw Vials — {thawingVial.species_code ?? thawingVial.species_id}</h3>
        <button class="modal-close" onclick={() => { thawingVial = null; thawResult = null; }} aria-label="Close">&#10005;</button>
      </div>

      {#if thawResult}
        <div class="thaw-success">
          <p class="success-msg">&#10003; Thaw recorded successfully.</p>
          <dl class="result-dl">
            <dt>New Specimen</dt>
            <dd><strong>{thawResult.new_specimen_accession}</strong></dd>
            <dt>Vials Remaining</dt>
            <dd>{thawResult.updated_vial.vial_count}</dd>
            <dt>Vial Status</dt>
            <dd><span class="badge {statusBadgeClass(thawResult.updated_vial.status)}">{thawResult.updated_vial.status}</span></dd>
          </dl>
          <div class="modal-footer">
            <button class="btn btn-primary" onclick={() => { thawingVial = null; thawResult = null; }}>Done</button>
          </div>
        </div>
      {:else}
        <div class="thaw-context">
          <p>Lot has <strong>{thawingVial.vial_count}</strong> vial(s) remaining.
             Passage <strong>P{thawingVial.passage_number}</strong>
             {#if thawingVial.cumulative_pdl != null}
               · PDL <strong>{thawingVial.cumulative_pdl.toFixed(1)}</strong>
             {/if}
          </p>
        </div>
        <form onsubmit={handleThaw}>
          <div class="form-grid">
            <label class="required">
              Thaw Date
              <input type="date" bind:value={thawForm.thaw_date} required />
            </label>

            <label class="required">
              Vials to Thaw
              <input
                type="number"
                min="1"
                max={thawingVial.vial_count}
                bind:value={thawForm.vials_to_thaw}
                required
              />
            </label>

            <label class="full-width">
              Location for New Specimen
              <input type="text" bind:value={thawForm.location} placeholder="e.g. Room 2 / Rack B / Shelf 3" />
            </label>

            <label>
              Employee ID
              <input type="text" bind:value={thawForm.employee_id} placeholder="optional" />
            </label>

            <label class="full-width">
              Notes
              <textarea bind:value={thawForm.notes} rows="2" placeholder="Optional notes"></textarea>
            </label>
          </div>
          <div class="modal-footer">
            <button type="button" class="btn btn-ghost" onclick={() => { thawingVial = null; }}>Cancel</button>
            <button type="submit" class="btn btn-primary" disabled={thawSaving}>
              {thawSaving ? 'Processing…' : 'Confirm Thaw'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}

<!-- Discard confirmation -->
{#if discardingVial}
  <div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Discard vials">
    <div class="modal modal-sm">
      <div class="modal-header">
        <h3>Discard Vial Lot</h3>
        <button class="modal-close" onclick={() => { discardingVial = null; }} aria-label="Close">&#10005;</button>
      </div>
      <p>Mark <strong>{discardingVial.species_code ?? discardingVial.species_id}</strong>
         ({discardingVial.vial_count} vials) as discarded? This cannot be undone.</p>
      <label>
        Reason / Notes
        <textarea bind:value={discardNotes} rows="2" placeholder="e.g. Contamination detected"></textarea>
      </label>
      <div class="modal-footer">
        <button class="btn btn-ghost" onclick={() => { discardingVial = null; }}>Cancel</button>
        <button class="btn btn-danger" onclick={handleDiscard} disabled={discardSaving}>
          {discardSaving ? 'Discarding…' : 'Discard Lot'}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .cryo-manager { padding: var(--space-4); max-width: 1200px; margin: 0 auto; }

  .page-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: var(--space-4);
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .page-header h2 { font-size: var(--font-size-xl); font-weight: 700; margin-bottom: 2px; }
  .subtitle { color: var(--color-text-muted); font-size: var(--font-size-sm); }

  .filter-bar {
    display: flex;
    gap: var(--space-3);
    align-items: flex-end;
    flex-wrap: wrap;
    margin-bottom: var(--space-4);
    padding: var(--space-3);
    background: var(--color-surface);
    border-radius: var(--radius-md);
    border: 1px solid var(--color-border);
  }
  .filter-bar label { display: flex; flex-direction: column; gap: 4px; font-size: var(--font-size-sm); }
  .filter-bar select, .filter-bar input { font-size: var(--font-size-sm); min-width: 120px; }

  .vial-table-wrap { overflow-x: auto; }
  .vial-table { width: 100%; border-collapse: collapse; font-size: var(--font-size-sm); }
  .vial-table th {
    text-align: left;
    padding: var(--space-2) var(--space-3);
    background: var(--color-surface);
    border-bottom: 2px solid var(--color-border);
    font-weight: 600;
    color: var(--color-text-muted);
    white-space: nowrap;
  }
  .vial-table td {
    padding: var(--space-2) var(--space-3);
    border-bottom: 1px solid var(--color-border);
    vertical-align: middle;
  }
  .vial-row:hover td { background: var(--color-surface-hover, rgba(0,0,0,0.03)); }
  .row-depleted td { opacity: 0.65; }
  .row-discarded td { opacity: 0.45; text-decoration: line-through; text-decoration-color: var(--color-text-muted); }

  .species-code { font-weight: 600; }
  .species-name { display: block; font-size: 0.75em; color: var(--color-text-muted); }
  .num-cell { text-align: right; font-variant-numeric: tabular-nums; }
  .vial-count { font-weight: 700; }
  .count-low { color: var(--color-warning, #d97706); }
  .medium-cell { max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .location-cell { font-size: 0.8em; color: var(--color-text-muted); }
  .actions-cell { white-space: nowrap; }
  .actions-cell .btn { margin-right: 4px; }

  .badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: var(--radius-full, 9999px);
    font-size: 0.75em;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .badge-active   { background: var(--color-success-subtle, #dcfce7); color: var(--color-success, #16a34a); }
  .badge-depleted { background: var(--color-warning-subtle, #fef9c3); color: var(--color-warning, #ca8a04); }
  .badge-discarded { background: var(--color-error-subtle, #fee2e2); color: var(--color-error, #dc2626); }

  /* Modal */
  .modal-backdrop {
    position: fixed; inset: 0;
    background: rgba(0,0,0,0.45);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000;
    padding: var(--space-4);
  }
  .modal {
    background: var(--color-background);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-xl, 0 20px 40px rgba(0,0,0,0.25));
    width: 100%;
    max-width: 600px;
    max-height: 90vh;
    overflow-y: auto;
    padding: var(--space-5);
  }
  .modal-sm { max-width: 440px; }
  .modal-header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: var(--space-4);
  }
  .modal-header h3 { font-size: var(--font-size-lg); font-weight: 700; }
  .modal-close {
    background: none; border: none; cursor: pointer;
    font-size: var(--font-size-lg); color: var(--color-text-muted);
    padding: 2px 6px; border-radius: var(--radius-sm);
  }
  .modal-close:hover { color: var(--color-text); background: var(--color-surface); }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .form-grid label, .form-grid .full-width { display: flex; flex-direction: column; gap: 4px; font-size: var(--font-size-sm); }
  .full-width { grid-column: 1 / -1; }
  label.required::after { content: ' *'; color: var(--color-error); }

  .field-label { font-size: var(--font-size-sm); font-weight: 500; margin-bottom: 4px; display: block; }
  .location-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: var(--space-2); }
  .location-preview {
    margin-top: var(--space-1);
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    font-style: italic;
  }

  .modal-footer {
    display: flex; justify-content: flex-end; gap: var(--space-2);
    padding-top: var(--space-3);
    border-top: 1px solid var(--color-border);
  }

  .thaw-context {
    background: var(--color-surface);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    margin-bottom: var(--space-4);
    font-size: var(--font-size-sm);
  }

  .thaw-success { padding: var(--space-2) 0; }
  .success-msg { color: var(--color-success, #16a34a); font-weight: 600; margin-bottom: var(--space-3); }
  .result-dl { display: grid; grid-template-columns: auto 1fr; gap: var(--space-1) var(--space-3); font-size: var(--font-size-sm); }
  .result-dl dt { color: var(--color-text-muted); font-weight: 500; }

  /* Buttons */
  .btn { padding: 6px 14px; border-radius: var(--radius-md); font-size: var(--font-size-sm); font-weight: 500; cursor: pointer; border: none; transition: opacity 0.15s; }
  .btn:disabled { opacity: 0.55; cursor: not-allowed; }
  .btn-primary { background: var(--color-primary); color: #fff; }
  .btn-primary:hover:not(:disabled) { opacity: 0.88; }
  .btn-danger { background: var(--color-error, #dc2626); color: #fff; }
  .btn-danger:hover:not(:disabled) { opacity: 0.88; }
  .btn-ghost { background: transparent; color: var(--color-text); border: 1px solid var(--color-border); }
  .btn-ghost:hover:not(:disabled) { background: var(--color-surface); }
  .btn-sm { padding: 3px 10px; font-size: 0.8em; }
</style>
