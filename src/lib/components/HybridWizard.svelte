<script lang="ts">
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import { listSpecies, listStrainsBySpecies, createHybridizationEvent, suggestGenerationLabel } from '../api';
  import { addNotification, addErrorWithContext } from '../stores/app';
  import { currentUser } from '../stores/auth';

  let { speciesId = '', speciesName = '', onclose, oncreated }:
    { speciesId?: string; speciesName?: string; onclose: () => void; oncreated: () => void } = $props();

  let step = $state(1);
  const TOTAL_STEPS = 9;

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
  // Admin cross-species override state
  let showCrossSpeciesOverride = $state(false);
  let adminOverrideReason = $state('');
  let adminOverrideConfirmed = $state(false);

  // Step 4 — Hybrid name/code/type
  let hybridName = $state('');
  let hybridCode = $state('');
  let hybridType = $state('hybrid');

  // Step 4.5 / Step 5 — Generation label
  let generationLabel = $state('');
  let suggestedLabel = $state<string | null>(null);
  let isSuggestedBackcross = $state(false);
  let loadingSuggestion = $state(false);

  // Step 6 — Optional parent specimens
  let parentASpecimenNote = $state('');
  let parentBSpecimenNote = $state('');

  // Step 7 — Cross date and method
  let crossDate = $state(new Date().toISOString().split('T')[0]);
  let crossMethod = $state('');

  let submitting = $state(false);

  let parentA = $derived(strainsA.find(s => s.id === parentAId) ?? null);
  let parentB = $derived(strainsB.find(s => s.id === parentBId) ?? null);
  let isAdmin = $derived(get(currentUser)?.role === 'admin');

  const crossMethods = [
    'Hand pollination',
    'Open pollination',
    'Somatic hybridization (protoplast fusion)',
    'Embryo rescue',
    'Graft hybridization',
    'Other',
  ];

  const knownLabels = ['F1', 'F2', 'F3', 'F4', 'BC1F1', 'BC1F2', 'BC2F1', 'BC2F2'];

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
    showCrossSpeciesOverride = false;
    adminOverrideConfirmed = false;
    await loadStrains(selectedSpeciesId);
  }

  function validateParentB() {
    crossSpeciesError = '';
    showCrossSpeciesOverride = false;
    adminOverrideConfirmed = false;
    if (!parentBId || !parentAId) return;
    const b = strainsB.find(s => s.id === parentBId);
    if (!b) return;
    if (b.species_id !== selectedSpeciesId) {
      crossSpeciesError = 'Cross-species selection is not permitted. Both parents must belong to the same species.';
      if (!isAdmin) {
        parentBId = '';
      } else {
        showCrossSpeciesOverride = true;
      }
      return;
    }
    if (parentBId === parentAId) {
      crossSpeciesError = 'Parent B must be different from Parent A.';
      parentBId = '';
    }
  }

  async function fetchSuggestion() {
    if (!parentAId || !parentBId) return;
    loadingSuggestion = true;
    try {
      const resp = await suggestGenerationLabel(parentAId, parentBId);
      suggestedLabel = resp.suggested_label;
      isSuggestedBackcross = resp.is_backcross;
      if (resp.suggested_label && !generationLabel) {
        generationLabel = resp.suggested_label;
      }
    } catch {
      suggestedLabel = null;
    } finally {
      loadingSuggestion = false;
    }
  }

  function canAdvance(): boolean {
    switch (step) {
      case 1: return !!selectedSpeciesId;
      case 2: return !!parentAId && !!parentARole;
      case 3: {
        if (!parentBId) return false;
        if (crossSpeciesError && !adminOverrideConfirmed) return false;
        return true;
      }
      case 4: return !!hybridName.trim() && !!hybridCode.trim();
      case 5: return true;
      case 6: return true;
      case 7: return !!crossDate;
      case 8: return true;
      case 9: return true;
      default: return false;
    }
  }

  function handleStepAdvance() {
    if (step === 3 && parentAId && parentBId) {
      fetchSuggestion();
    }
    step++;
  }

  function buildNotes(): string {
    const parts: string[] = [];
    parts.push(`Cross date: ${crossDate}`);
    if (crossMethod) parts.push(`Method: ${crossMethod}`);
    if (parentARole !== 'parent') parts.push(`Parent A role: ${parentARole}`);
    if (parentASpecimenNote) parts.push(`Parent A specimen: ${parentASpecimenNote}`);
    if (parentBSpecimenNote) parts.push(`Parent B specimen: ${parentBSpecimenNote}`);
    if (isSuggestedBackcross) parts.push('Backcross relationship detected');
    return parts.join('\n');
  }

  async function handleConfirm() {
    if (!parentAId || !parentBId || !hybridName.trim() || !hybridCode.trim()) return;
    submitting = true;
    try {
      const isCrossSpecies = crossSpeciesError && adminOverrideConfirmed;
      await createHybridizationEvent({
        parent_a_id: parentAId,
        parent_b_id: parentBId,
        name: hybridName.trim(),
        code: hybridCode.trim(),
        notes: buildNotes() || undefined,
        generation_label: generationLabel.trim() || undefined,
        admin_override_cross_species: isCrossSpecies ? true : undefined,
        admin_override_reason: isCrossSpecies ? adminOverrideReason : undefined,
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

        <!-- Step 3: Parent B with cross-species guard -->
        {:else if step === 3}
          <h3 class="step-title">3. Select Parent B</h3>
          {#if crossSpeciesError}
            <div class="cross-species-error">
              <strong>Cross-Species Selection Blocked</strong>
              <p style="margin:4px 0 0;">{crossSpeciesError}</p>
            </div>
          {/if}
          {#if showCrossSpeciesOverride}
            <div class="admin-override-panel">
              <div class="override-warning-header">
                <span class="override-icon">&#9888;</span>
                <strong>Admin Override — Cross-Species Hybridization</strong>
              </div>
              <p class="override-warning-text">
                This creates a <strong>permanent, non-removable</strong> audit warning. The resulting
                hybrid will display a visible cross-species warning banner on all future views.
                This action cannot be undone.
              </p>
              <div class="form-group" style="margin-top:12px;">
                <label for="hw-override-reason">Scientific justification for override *</label>
                <textarea
                  id="hw-override-reason"
                  bind:value={adminOverrideReason}
                  rows="3"
                  placeholder="Provide a detailed scientific reason for this cross-species hybridization…"
                ></textarea>
              </div>
              <label class="override-confirm-label">
                <input
                  type="checkbox"
                  bind:checked={adminOverrideConfirmed}
                  disabled={!adminOverrideReason.trim()}
                />
                I understand this is a cross-species hybridization and consent to the permanent audit warning.
              </label>
            </div>
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
          {#if !crossSpeciesError}
            <p class="step-hint">Only strains from the same species ({speciesLabel(selectedSpeciesId)}) are permitted. Cross-species hybridization requires admin authorisation.</p>
          {/if}

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

        <!-- Step 5: Generation label -->
        {:else if step === 5}
          <h3 class="step-title">5. Generation Label <span class="step-optional">(Optional)</span></h3>
          {#if loadingSuggestion}
            <div class="suggestion-loading">Analysing parent lineages…</div>
          {:else if suggestedLabel}
            <div class="suggestion-box" class:backcross={isSuggestedBackcross}>
              <div class="suggestion-icon">{isSuggestedBackcross ? '↩' : '✓'}</div>
              <div class="suggestion-content">
                <strong>Suggested: {suggestedLabel}</strong>
                {#if isSuggestedBackcross}
                  <p class="suggestion-note">Backcross relationship detected — one parent is an ancestor of the other.</p>
                {:else}
                  <p class="suggestion-note">Inferred from parent generation labels.</p>
                {/if}
              </div>
            </div>
          {/if}
          <div class="form-group">
            <label for="hw-gen-sel">Quick-select label</label>
            <select id="hw-gen-sel" bind:value={generationLabel}>
              <option value="">None / Unknown</option>
              {#each knownLabels as lbl}
                <option value={lbl}>{lbl}{lbl === suggestedLabel ? ' (suggested)' : ''}</option>
              {/each}
            </select>
          </div>
          <div class="form-group">
            <label for="hw-gen-text">Or enter a custom label</label>
            <input
              id="hw-gen-text"
              type="text"
              bind:value={generationLabel}
              placeholder="e.g., BC3F2, F5…"
            />
          </div>
          <p class="step-hint">
            F1 = first filial generation from two distinct parents · F2 = progeny of two F1 plants ·
            BC = backcross to an ancestor line. Leaving this blank is fine if the generation is unknown.
          </p>

        <!-- Step 6: Optional parent specimens -->
        {:else if step === 6}
          <h3 class="step-title">6. Parent Specimens <span class="step-optional">(Optional)</span></h3>
          <p class="step-hint">Record the specific specimens used in this cross for traceability. This information will be included in the event notes.</p>
          <div class="form-group">
            <label for="hw-psa">Parent A Specimen (Accession / ID)</label>
            <input id="hw-psa" type="text" bind:value={parentASpecimenNote} placeholder="e.g., 2025-01-15-SPEC-001" />
          </div>
          <div class="form-group">
            <label for="hw-psb">Parent B Specimen (Accession / ID)</label>
            <input id="hw-psb" type="text" bind:value={parentBSpecimenNote} placeholder="e.g., 2025-03-22-SPEC-007" />
          </div>

        <!-- Step 7: Cross date and method -->
        {:else if step === 7}
          <h3 class="step-title">7. Cross Details</h3>
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

        <!-- Step 8: Pedigree preview -->
        {:else if step === 8}
          <h3 class="step-title">8. Pedigree Preview</h3>
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
                {#if generationLabel}<div class="pn-role">{generationLabel}</div>{/if}
              </div>
            </div>
          </div>
          {#if isSuggestedBackcross}
            <div class="backcross-notice">&#8617; Backcross relationship detected between these parents.</div>
          {/if}
          <div class="pedigree-meta">
            <div><strong>Cross date:</strong> {crossDate}</div>
            {#if crossMethod}<div><strong>Method:</strong> {crossMethod}</div>{/if}
            {#if generationLabel}<div><strong>Generation:</strong> {generationLabel}</div>{/if}
            {#if parentASpecimenNote}<div><strong>Parent A specimen:</strong> {parentASpecimenNote}</div>{/if}
            {#if parentBSpecimenNote}<div><strong>Parent B specimen:</strong> {parentBSpecimenNote}</div>{/if}
          </div>

        <!-- Step 9: Final review -->
        {:else if step === 9}
          <h3 class="step-title">9. Review & Confirm</h3>
          {#if crossSpeciesError && adminOverrideConfirmed}
            <div class="review-cross-species-warning">
              &#9888; Cross-species override active — a permanent audit warning will be recorded.
            </div>
          {/if}
          <div class="review-grid">
            <span class="rg-label">Species</span><span>{speciesLabel(selectedSpeciesId)}</span>
            <span class="rg-label">Parent A</span><span>{parentA?.code} — {parentA?.name} ({parentARole})</span>
            <span class="rg-label">Parent B</span><span>{parentB?.code} — {parentB?.name}</span>
            <span class="rg-label">Hybrid Name</span><span>{hybridName}</span>
            <span class="rg-label">Hybrid Code</span><span><code>{hybridCode}</code></span>
            <span class="rg-label">Type</span><span>{hybridType}</span>
            {#if generationLabel}<span class="rg-label">Generation</span><span>{generationLabel}{isSuggestedBackcross ? ' (backcross)' : ''}</span>{/if}
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
          <button class="btn btn-sm btn-primary" onclick={handleStepAdvance} disabled={!canAdvance()}>Next →</button>
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

  .step-optional { font-size: 12px; font-weight: 400; color: #9ca3af; margin-left: 4px; }

  /* Admin cross-species override panel */
  .admin-override-panel {
    background: #fefce8;
    border: 1px solid #fbbf24;
    border-radius: 8px;
    padding: 14px 16px;
    margin-bottom: 14px;
  }
  .override-warning-header {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: #92400e;
    margin-bottom: 8px;
  }
  .override-icon { font-size: 16px; }
  .override-warning-text { font-size: 12px; color: #78350f; margin: 0 0 4px; line-height: 1.5; }
  .override-confirm-label {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    font-size: 12px;
    color: #92400e;
    cursor: pointer;
    font-weight: 500;
    text-transform: none;
    letter-spacing: 0;
    margin-top: 8px;
  }
  .override-confirm-label input { margin-top: 2px; flex-shrink: 0; }

  /* Generation label step */
  .suggestion-loading { font-size: 12px; color: #6b7280; margin-bottom: 12px; font-style: italic; }
  .suggestion-box {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background: #f0fdf4;
    border: 1px solid #86efac;
    border-radius: 8px;
    padding: 12px 14px;
    margin-bottom: 14px;
  }
  .suggestion-box.backcross { background: #fffbeb; border-color: #fcd34d; }
  .suggestion-icon { font-size: 18px; line-height: 1; color: #16a34a; }
  .suggestion-box.backcross .suggestion-icon { color: #d97706; }
  .suggestion-content { font-size: 13px; color: #166534; }
  .suggestion-box.backcross .suggestion-content { color: #92400e; }
  .suggestion-note { font-size: 11px; margin: 3px 0 0; opacity: 0.8; }

  /* Backcross notice on pedigree step */
  .backcross-notice {
    background: #fffbeb;
    border: 1px solid #fcd34d;
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 12px;
    color: #92400e;
    margin-bottom: 12px;
  }

  /* Cross-species warning in review */
  .review-cross-species-warning {
    background: #fef2f2;
    border: 1px solid #fca5a5;
    border-radius: 6px;
    padding: 10px 12px;
    color: #991b1b;
    font-size: 12px;
    font-weight: 600;
    margin-bottom: 14px;
  }

  :global(.dark) .hw-box { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .hw-close { color: #94a3b8; }
  :global(.dark) .hw-close:hover { background: #334155; }
  :global(.dark) .hw-footer { border-top-color: #334155; }
  :global(.dark) .step-title { color: #f1f5f9; }
  :global(.dark) .pedigree { background: #0f172a; border-color: #334155; }
  :global(.dark) .pedigree-node { background: #1e293b; border-color: #475569; color: #e2e8f0; }
  :global(.dark) .hybrid-node { background: #064e3b; border-color: #059669; }
  :global(.dark) .admin-override-panel { background: #422006; border-color: #92400e; }
  :global(.dark) .override-warning-header { color: #fcd34d; }
  :global(.dark) .override-warning-text { color: #fde68a; }
  :global(.dark) .override-confirm-label { color: #fde68a; }
  :global(.dark) .suggestion-box { background: #064e3b; border-color: #059669; }
  :global(.dark) .suggestion-box.backcross { background: #422006; border-color: #92400e; }
  :global(.dark) .suggestion-content { color: #a7f3d0; }
  :global(.dark) .suggestion-box.backcross .suggestion-content { color: #fde68a; }
</style>
