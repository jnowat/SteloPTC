<script lang="ts">
  import { onMount } from 'svelte';
  import { listSpecies, listStrainsBySpecies, createHybridizationEvent } from '../api';
  import { addNotification, addErrorWithContext } from '../stores/app';

  let { speciesId = '', speciesName = '', onclose, oncreated }:
    { speciesId?: string; speciesName?: string; onclose: () => void; oncreated: () => void } = $props();

  let step = $state(1);
  const TOTAL_STEPS = 8;

  let allSpecies = $state<any[]>([]);
  // svelte-ignore state_referenced_locally
  let selectedSpeciesId = $state(speciesId);
  let strainsA = $state<any[]>([]);
  let strainsB = $state<any[]>([]);
  let loadingStrains = $state(false);

  // Step 2 — Parent A
  let parentAId = $state('');
  let parentARole = $state<'maternal' | 'paternal' | 'parent'>('parent');

  // Step 3 — Parent B
  let parentBId = $state('');
  let crossSpeciesError = $state('');

  // Step 4 — Hybrid name/code/type
  let hybridName = $state('');
  let hybridCode = $state('');
  let hybridType = $state('hybrid');

  // Step 5 — Optional parent specimens
  let parentASpecimenNote = $state('');
  let parentBSpecimenNote = $state('');

  // Step 6 — Cross date and method
  let crossDate = $state(new Date().toISOString().split('T')[0]);
  let crossMethod = $state('');

  let submitting = $state(false);

  let parentA = $derived(strainsA.find(s => s.id === parentAId) ?? null);
  let parentB = $derived(strainsB.find(s => s.id === parentBId) ?? null);

  const crossMethods = [
    'Hand pollination',
    'Open pollination',
    'Somatic hybridization (protoplast fusion)',
    'Embryo rescue',
    'Graft hybridization',
    'Other',
  ];

  onMount(async () => {
    allSpecies = await listSpecies().catch(() => []);
    if (selectedSpeciesId) {
      await loadStrains(selectedSpeciesId);
    }
  });

  async function loadStrains(spId: string) {
    if (!spId) { strainsA = []; strainsB = []; return; }
    loadingStrains = true;
    try {
      const s = await listStrainsBySpecies(spId);
      strainsA = s;
      strainsB = s;
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loadingStrains = false;
    }
  }

  async function handleSpeciesChange() {
    parentAId = '';
    parentBId = '';
    crossSpeciesError = '';
    await loadStrains(selectedSpeciesId);
  }

  function validateParentB() {
    crossSpeciesError = '';
    if (!parentBId || !parentAId) return;
    const b = strainsB.find(s => s.id === parentBId);
    if (!b) return;
    if (b.species_id !== selectedSpeciesId) {
      crossSpeciesError = 'Cross-species selection is not permitted. Both parents must belong to the same species.';
      parentBId = '';
    }
    if (parentBId === parentAId) {
      crossSpeciesError = 'Parent B must be different from Parent A.';
      parentBId = '';
    }
  }

  function canAdvance(): boolean {
    switch (step) {
      case 1: return !!selectedSpeciesId;
      case 2: return !!parentAId && !!parentARole;
      case 3: return !!parentBId && !crossSpeciesError;
      case 4: return !!hybridName.trim() && !!hybridCode.trim();
      case 5: return true;
      case 6: return !!crossDate;
      case 7: return true;
      case 8: return true;
      default: return false;
    }
  }

  function buildNotes(): string {
    const parts: string[] = [];
    parts.push(`Cross date: ${crossDate}`);
    if (crossMethod) parts.push(`Method: ${crossMethod}`);
    if (parentARole !== 'parent') parts.push(`Parent A role: ${parentARole}`);
    if (parentASpecimenNote) parts.push(`Parent A specimen: ${parentASpecimenNote}`);
    if (parentBSpecimenNote) parts.push(`Parent B specimen: ${parentBSpecimenNote}`);
    return parts.join('\n');
  }

  async function handleConfirm() {
    if (!parentAId || !parentBId || !hybridName.trim() || !hybridCode.trim()) return;
    submitting = true;
    try {
      await createHybridizationEvent({
        parent_a_id: parentAId,
        parent_b_id: parentBId,
        name: hybridName.trim(),
        code: hybridCode.trim(),
        notes: buildNotes() || undefined,
      });
      oncreated();
    } catch (e: any) {
      addErrorWithContext('Failed to Create Hybrid', e.message, 'strains.hybridize', {
        parent_a_id: parentAId,
        parent_b_id: parentBId,
        name: hybridName,
        code: hybridCode,
      });
    } finally {
      submitting = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }

  function speciesLabel(spId: string): string {
    const sp = allSpecies.find(s => s.id === spId);
    return sp ? `${sp.species_code} — ${sp.genus} ${sp.species_name}` : spId;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="hw-backdrop" onclick={onclose}>
  <div class="hw-box" role="dialog" aria-modal="true" aria-label="New Hybrid Strain Wizard">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div onclick={(e) => e.stopPropagation()}>
      <div class="hw-header">
        <div>
          <h2 class="hw-title">New Hybrid Strain</h2>
          <div class="hw-steps">Step {step} of {TOTAL_STEPS}</div>
        </div>
        <button class="hw-close" onclick={onclose} aria-label="Close wizard">&#10005;</button>
      </div>

      <div class="hw-progress">
        {#each Array.from({ length: TOTAL_STEPS }, (_, i) => i + 1) as n}
          <div class="hw-dot" class:done={n < step} class:active={n === step}></div>
        {/each}
      </div>

      <div class="hw-body">
        <!-- Step 1: Select species -->
        {#if step === 1}
          <h3 class="step-title">1. Select Species</h3>
          <div class="form-group">
            <label for="hw-species">Species *</label>
            <select id="hw-species" bind:value={selectedSpeciesId} onchange={handleSpeciesChange}>
              <option value="">Select species…</option>
              {#each allSpecies as sp}
                <option value={sp.id}>{sp.species_code} — {sp.genus} {sp.species_name}</option>
              {/each}
            </select>
          </div>
          {#if selectedSpeciesId}
            <p class="step-hint">Species selected: <strong>{speciesLabel(selectedSpeciesId)}</strong></p>
          {/if}

        <!-- Step 2: Parent A + role -->
        {:else if step === 2}
          <h3 class="step-title">2. Select Parent A</h3>
          {#if loadingStrains}
            <div class="step-hint">Loading strains…</div>
          {:else if strainsA.length === 0}
            <div class="step-hint empty">No strains available for this species. Create strains first.</div>
          {:else}
            <div class="form-group">
              <label for="hw-pa">Parent A Strain *</label>
              <select id="hw-pa" bind:value={parentAId}>
                <option value="">Select strain…</option>
                {#each strainsA as s}
                  <option value={s.id}>{s.code} — {s.name}</option>
                {/each}
              </select>
            </div>
            <div class="form-group">
              <fieldset style="border:none;padding:0;margin:0;">
              <legend style="font-size:12px;font-weight:600;color:#6b7280;margin-bottom:4px;">Role of Parent A</legend>
              <div class="role-group">
                {#each (['maternal', 'paternal', 'parent'] as const) as role}
                  <label class="role-option">
                    <input type="radio" bind:group={parentARole} value={role} />
                    {role.charAt(0).toUpperCase() + role.slice(1)}
                  </label>
                {/each}
              </div>
              </fieldset>
            </div>
          {/if}

        <!-- Step 3: Parent B (same species only) -->
        {:else if step === 3}
          <h3 class="step-title">3. Select Parent B</h3>
          {#if crossSpeciesError}
            <div class="cross-species-error">{crossSpeciesError}</div>
          {/if}
          <div class="form-group">
            <label for="hw-pb">Parent B Strain *</label>
            <select id="hw-pb" bind:value={parentBId} onchange={validateParentB}>
              <option value="">Select strain…</option>
              {#each strainsB.filter(s => s.id !== parentAId) as s}
                <option value={s.id}>{s.code} — {s.name}</option>
              {/each}
            </select>
          </div>
          <p class="step-hint">Only strains from the same species ({speciesLabel(selectedSpeciesId)}) are shown. Cross-species hybridization is not permitted.</p>

        <!-- Step 4: Hybrid name, code, type -->
        {:else if step === 4}
          <h3 class="step-title">4. Hybrid Strain Identity</h3>
          <div class="form-group">
            <label for="hw-name">Strain Name *</label>
            <input id="hw-name" type="text" bind:value={hybridName} placeholder="e.g., Clone A-3 × Clone B-1" />
          </div>
          <div class="form-group">
            <label for="hw-code">Strain Code *</label>
            <input id="hw-code" type="text" bind:value={hybridCode} placeholder="e.g., HYB-AB3B1" />
          </div>
          <div class="form-group">
            <label for="hw-type">Strain Type</label>
            <select id="hw-type" bind:value={hybridType}>
              <option value="hybrid">Hybrid</option>
              <option value="f1_hybrid">F1 Hybrid</option>
              <option value="f2_hybrid">F2 Hybrid</option>
            </select>
          </div>

        <!-- Step 5: Optional parent specimens -->
        {:else if step === 5}
          <h3 class="step-title">5. Parent Specimens (Optional)</h3>
          <p class="step-hint">Record the specific specimens used in this cross for traceability. This information will be included in the event notes.</p>
          <div class="form-group">
            <label for="hw-psa">Parent A Specimen (Accession / ID)</label>
            <input id="hw-psa" type="text" bind:value={parentASpecimenNote} placeholder="e.g., 2025-01-15-SPEC-001" />
          </div>
          <div class="form-group">
            <label for="hw-psb">Parent B Specimen (Accession / ID)</label>
            <input id="hw-psb" type="text" bind:value={parentBSpecimenNote} placeholder="e.g., 2025-03-22-SPEC-007" />
          </div>

        <!-- Step 6: Cross date and method -->
        {:else if step === 6}
          <h3 class="step-title">6. Cross Details</h3>
          <div class="form-group">
            <label for="hw-date">Cross Date *</label>
            <input id="hw-date" type="date" bind:value={crossDate} />
          </div>
          <div class="form-group">
            <label for="hw-method">Cross Method</label>
            <select id="hw-method" bind:value={crossMethod}>
              <option value="">Select method…</option>
              {#each crossMethods as m}
                <option value={m}>{m}</option>
              {/each}
            </select>
          </div>

        <!-- Step 7: Pedigree preview -->
        {:else if step === 7}
          <h3 class="step-title">7. Pedigree Preview</h3>
          <div class="pedigree">
            <div class="pedigree-row">
              <div class="pedigree-node parent-node">
                <div class="pn-label">{parentA?.code ?? '—'}</div>
                <div class="pn-name">{parentA?.name ?? '—'}</div>
                <div class="pn-role">{parentARole}</div>
              </div>
              <div class="pedigree-node parent-node">
                <div class="pn-label">{parentB?.code ?? '—'}</div>
                <div class="pn-name">{parentB?.name ?? '—'}</div>
                <div class="pn-role">
                  {parentARole === 'maternal' ? 'paternal' : parentARole === 'paternal' ? 'maternal' : 'parent'}
                </div>
              </div>
            </div>
            <div class="pedigree-connectors">
              <div class="pedigree-line-left"></div>
              <div class="pedigree-join"></div>
              <div class="pedigree-line-right"></div>
            </div>
            <div class="pedigree-arrow">&#8595;</div>
            <div class="pedigree-row">
              <div class="pedigree-node hybrid-node">
                <div class="pn-label">{hybridCode || '—'}</div>
                <div class="pn-name">{hybridName || '—'}</div>
                <div class="pn-role">New Hybrid</div>
              </div>
            </div>
          </div>
          <div class="pedigree-meta">
            <div><strong>Cross date:</strong> {crossDate}</div>
            {#if crossMethod}<div><strong>Method:</strong> {crossMethod}</div>{/if}
            {#if parentASpecimenNote}<div><strong>Parent A specimen:</strong> {parentASpecimenNote}</div>{/if}
            {#if parentBSpecimenNote}<div><strong>Parent B specimen:</strong> {parentBSpecimenNote}</div>{/if}
          </div>

        <!-- Step 8: Final review -->
        {:else if step === 8}
          <h3 class="step-title">8. Review & Confirm</h3>
          <div class="review-grid">
            <span class="rg-label">Species</span><span>{speciesLabel(selectedSpeciesId)}</span>
            <span class="rg-label">Parent A</span><span>{parentA?.code} — {parentA?.name} ({parentARole})</span>
            <span class="rg-label">Parent B</span><span>{parentB?.code} — {parentB?.name}</span>
            <span class="rg-label">Hybrid Name</span><span>{hybridName}</span>
            <span class="rg-label">Hybrid Code</span><span><code>{hybridCode}</code></span>
            <span class="rg-label">Type</span><span>{hybridType}</span>
            <span class="rg-label">Cross Date</span><span>{crossDate}</span>
            {#if crossMethod}<span class="rg-label">Method</span><span>{crossMethod}</span>{/if}
          </div>
          <p class="step-hint">
            Clicking Confirm will atomically create the hybrid strain, record the hybridization event, and append audit entries to both parent chains.
          </p>
        {/if}
      </div>

      <div class="hw-footer">
        <button class="btn btn-sm" onclick={() => step > 1 ? step-- : onclose()}>
          {step === 1 ? 'Cancel' : '← Back'}
        </button>
        {#if step < TOTAL_STEPS}
          <button class="btn btn-sm btn-primary" onclick={() => step++} disabled={!canAdvance()}>Next →</button>
        {:else}
          <button class="btn btn-sm btn-primary" onclick={handleConfirm} disabled={submitting || !canAdvance()}>
            {submitting ? 'Creating…' : 'Confirm & Create'}
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .hw-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1100;
    background: rgba(0,0,0,0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
  }
  .hw-box {
    background: white;
    border-radius: 12px;
    width: 100%;
    max-width: 560px;
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 24px 64px rgba(0,0,0,0.35);
  }
  .hw-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 24px 24px 0;
  }
  .hw-title { font-size: 20px; font-weight: 700; margin-bottom: 2px; }
  .hw-steps { font-size: 12px; color: #6b7280; }
  .hw-close {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: #6b7280;
    padding: 4px 8px;
    border-radius: 4px;
    min-height: 0;
  }
  .hw-close:hover { background: #f3f4f6; }

  .hw-progress {
    display: flex;
    gap: 6px;
    padding: 12px 24px;
  }
  .hw-dot {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: #e2e8f0;
    transition: background 0.2s;
  }
  .hw-dot.done { background: #10b981; }
  .hw-dot.active { background: #2563eb; }

  .hw-body { padding: 0 24px; min-height: 240px; }
  .hw-footer {
    display: flex;
    justify-content: space-between;
    padding: 20px 24px;
    border-top: 1px solid #e2e8f0;
    margin-top: 20px;
  }

  .step-title { font-size: 15px; font-weight: 600; margin-bottom: 16px; color: #1e293b; }
  .step-hint { font-size: 12px; color: #6b7280; margin-top: 8px; }
  .step-hint.empty { color: #dc2626; }

  .cross-species-error {
    background: #fef2f2;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 10px 12px;
    color: #991b1b;
    font-size: 13px;
    margin-bottom: 12px;
  }

  .role-group { display: flex; gap: 12px; }
  .role-option {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 13px;
    color: #374151;
    cursor: pointer;
    font-weight: 500;
    text-transform: none;
    letter-spacing: 0;
  }

  /* Pedigree preview */
  .pedigree {
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0;
    margin-bottom: 16px;
  }
  .pedigree-row {
    display: flex;
    gap: 32px;
    justify-content: center;
  }
  .pedigree-node {
    background: white;
    border: 2px solid #e2e8f0;
    border-radius: 8px;
    padding: 10px 16px;
    min-width: 130px;
    text-align: center;
  }
  .parent-node { border-color: #93c5fd; }
  .hybrid-node { border-color: #6ee7b7; background: #f0fdf4; }
  .pn-label { font-weight: 700; font-size: 13px; color: #1e293b; }
  .pn-name { font-size: 11px; color: #6b7280; margin-top: 2px; }
  .pn-role { font-size: 10px; color: #9ca3af; margin-top: 4px; font-style: italic; }

  .pedigree-connectors {
    display: flex;
    width: 296px;
    height: 16px;
    margin-top: -1px;
  }
  .pedigree-line-left {
    flex: 1;
    border-bottom: 2px solid #94a3b8;
    border-left: 2px solid #94a3b8;
    border-radius: 0 0 0 4px;
  }
  .pedigree-join {
    width: 2px;
    background: #94a3b8;
  }
  .pedigree-line-right {
    flex: 1;
    border-bottom: 2px solid #94a3b8;
    border-right: 2px solid #94a3b8;
    border-radius: 0 0 4px 0;
  }
  .pedigree-arrow {
    font-size: 20px;
    color: #94a3b8;
    line-height: 1;
    margin: 2px 0;
  }

  .pedigree-meta {
    font-size: 12px;
    color: #6b7280;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .review-grid {
    display: grid;
    grid-template-columns: 140px 1fr;
    gap: 6px 12px;
    font-size: 13px;
    margin-bottom: 16px;
  }
  .rg-label { font-weight: 600; color: #6b7280; }

  :global(.dark) .hw-box { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .hw-close { color: #94a3b8; }
  :global(.dark) .hw-close:hover { background: #334155; }
  :global(.dark) .hw-footer { border-top-color: #334155; }
  :global(.dark) .step-title { color: #f1f5f9; }
  :global(.dark) .pedigree { background: #0f172a; border-color: #334155; }
  :global(.dark) .pedigree-node { background: #1e293b; border-color: #475569; color: #e2e8f0; }
  :global(.dark) .hybrid-node { background: #064e3b; border-color: #059669; }
</style>
