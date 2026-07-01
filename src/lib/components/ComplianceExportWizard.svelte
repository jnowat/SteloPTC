<script lang="ts">
  import { onMount, tick } from 'svelte';
  import {
    getSigningPublicKey,
    exportFdaPart11Bundle,
    exportUsdaPermit,
    exportCitesDossier,
    listSpecimens,
  } from '../api';
  import { addNotification } from '../stores/app';

  let { onclose }: { onclose: () => void } = $props();

  type RegType = 'fda_part11' | 'usda' | 'cites';

  let modalEl: HTMLDivElement;
  let currentStep = $state(1);
  const TOTAL_STEPS = 5;

  // Step 1 — regulation type
  let regType = $state<RegType | null>(null);

  // Step 2 — scope
  let fromDate = $state('');
  let toDate = $state('');

  let specimens = $state<any[]>([]);
  let loadingSpecimens = $state(false);
  let specimenLoadError = $state<string | null>(null);
  let specimenSearch = $state('');

  let usdaSelectedIds = $state<Set<string>>(new Set());
  let authorizedScientist = $state('');

  let citesRootId = $state('');
  let citesAppendix = $state('');

  // Step 4 — signing key (Part 11 only)
  let signingKey = $state<string | null>(null);
  let loadingKey = $state(false);
  let keyError = $state<string | null>(null);
  let keyCopied = $state(false);

  // Step 5 — generate
  let generating = $state(false);
  let generateError = $state<string | null>(null);
  let result = $state<{ ok: boolean; file_path: string; size_bytes: number } | null>(null);

  const regOptions: Array<{ value: RegType; label: string; description: string }> = [
    {
      value: 'fda_part11',
      label: 'FDA 21 CFR Part 11',
      description: 'Signed audit-trail attestation package for electronic-records compliance inspections.',
    },
    {
      value: 'usda',
      label: 'USDA APHIS (PPQ Form 526)',
      description: 'Pre-filled plant movement permit application (plant tissue culture labs only).',
    },
    {
      value: 'cites',
      label: 'CITES Species Provenance Dossier',
      description: 'Chain-of-custody dossier for internationally protected species.',
    },
  ];

  const filteredSpecimens = $derived(
    specimenSearch.trim()
      ? specimens.filter((s: any) =>
          `${s.accession_number ?? ''} ${s.species_code ?? ''} ${s.genus ?? ''} ${s.species_name ?? ''}`
            .toLowerCase()
            .includes(specimenSearch.trim().toLowerCase()),
        )
      : specimens,
  );

  const usdaSelectedSpecimens = $derived(specimens.filter((s: any) => usdaSelectedIds.has(s.id)));
  const citesRootSpecimen = $derived(specimens.find((s: any) => s.id === citesRootId) ?? null);

  onMount(async () => {
    await tick();
    modalEl?.querySelector<HTMLElement>('.cew-close-btn')?.focus();
  });

  async function ensureSpecimensLoaded() {
    if (specimens.length > 0 || loadingSpecimens) return;
    loadingSpecimens = true;
    specimenLoadError = null;
    try {
      const result = await listSpecimens(1, 500);
      specimens = result.items ?? [];
    } catch (e: any) {
      specimenLoadError = e.message;
    } finally {
      loadingSpecimens = false;
    }
  }

  function specimenLabel(s: any): string {
    const species = s.species_code ? s.species_code : `${s.genus ?? ''} ${s.species_name ?? ''}`.trim();
    return `${s.accession_number}${species ? ` — ${species}` : ''}`;
  }

  function toggleUsdaSpecimen(id: string) {
    const next = new Set(usdaSelectedIds);
    if (next.has(id)) next.delete(id); else next.add(id);
    usdaSelectedIds = next;
  }

  async function loadSigningKey() {
    if (signingKey || loadingKey) return;
    loadingKey = true;
    keyError = null;
    try {
      signingKey = await getSigningPublicKey();
    } catch (e: any) {
      keyError = e.message;
    } finally {
      loadingKey = false;
    }
  }

  async function copyFullKey() {
    if (!signingKey) return;
    try {
      await navigator.clipboard.writeText(signingKey);
      keyCopied = true;
      setTimeout(() => { keyCopied = false; }, 1500);
    } catch { /* clipboard unavailable */ }
  }

  function truncKey(key: string): string {
    if (key.length <= 20) return key;
    return `${key.slice(0, 8)}…${key.slice(-8)}`;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    const units = ['KB', 'MB', 'GB'];
    let value = bytes / 1024;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024;
      unitIndex++;
    }
    return `${value.toFixed(1)} ${units[unitIndex]}`;
  }

  function canAdvance(): boolean {
    switch (currentStep) {
      case 1:
        return regType !== null;
      case 2:
        if (regType === 'fda_part11') return !!fromDate && !!toDate && fromDate <= toDate;
        if (regType === 'usda') return usdaSelectedIds.size > 0 && !!authorizedScientist.trim();
        if (regType === 'cites') return !!citesRootId && !!citesAppendix;
        return false;
      case 3:
        return true;
      case 4:
        // Only relevant for Part 11 — must have successfully loaded the key
        if (regType === 'fda_part11') return !!signingKey;
        return true;
      case 5:
        return true;
      default:
        return false;
    }
  }

  async function handleNext() {
    if (!canAdvance()) return;
    // Steps map differently for non-Part11 flows (they skip the signing-key step)
    if (currentStep === 2) {
      currentStep = 3;
      return;
    }
    if (currentStep === 3) {
      if (regType === 'fda_part11') {
        currentStep = 4;
        await loadSigningKey();
      } else {
        currentStep = 5;
      }
      return;
    }
    currentStep++;
  }

  function handleBack() {
    if (currentStep === 5 && regType !== 'fda_part11') {
      currentStep = 3;
      return;
    }
    currentStep--;
  }

  async function handleGenerate() {
    if (!regType) return;
    generating = true;
    generateError = null;
    result = null;
    try {
      if (regType === 'fda_part11') {
        result = await exportFdaPart11Bundle(fromDate, toDate, 'SteloPTC Lab');
      } else if (regType === 'usda') {
        result = await exportUsdaPermit(Array.from(usdaSelectedIds), authorizedScientist.trim());
      } else if (regType === 'cites') {
        result = await exportCitesDossier(citesRootId, citesAppendix);
      }
      if (result) {
        addNotification(`Export saved to: ${result.file_path}`, 'success');
      }
    } catch (e: any) {
      generateError = e.message;
      addNotification(e.message, 'error');
    } finally {
      generating = false;
    }
  }

  function resetAndClose() {
    currentStep = 1;
    regType = null;
    fromDate = '';
    toDate = '';
    usdaSelectedIds = new Set();
    authorizedScientist = '';
    citesRootId = '';
    citesAppendix = '';
    specimenSearch = '';
    signingKey = null;
    keyError = null;
    generateError = null;
    result = null;
    onclose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') { resetAndClose(); return; }
    if (e.key === 'Tab' && modalEl) {
      const focusable = Array.from(modalEl.querySelectorAll<HTMLElement>(
        'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
      ));
      if (focusable.length === 0) { e.preventDefault(); return; }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey) {
        if (document.activeElement === first) { e.preventDefault(); last.focus(); }
      } else {
        if (document.activeElement === last) { e.preventDefault(); first.focus(); }
      }
    }
  }

  // Load specimens as soon as we reach the scope step for USDA/CITES
  $effect(() => {
    if (currentStep === 2 && (regType === 'usda' || regType === 'cites')) {
      ensureSpecimensLoaded();
    }
  });

  const regLabel = $derived(regOptions.find(r => r.value === regType)?.label ?? '');
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="cew-panel" role="dialog" aria-modal="true" aria-labelledby="cew-title" bind:this={modalEl}>
  <div class="cew-header">
    <div>
      <h3 id="cew-title" class="cew-title">Regulatory Export Wizard</h3>
      <div class="cew-steps">Step {currentStep} of {TOTAL_STEPS}{regLabel ? ` — ${regLabel}` : ''}</div>
    </div>
    <button class="cew-close-btn" onclick={resetAndClose} aria-label="Close regulatory export wizard" title="Cancel and close this wizard">&#10005;</button>
  </div>

  <div class="cew-progress">
    {#each Array.from({ length: TOTAL_STEPS }, (_, i) => i + 1) as n}
      <div class="cew-dot" class:done={n < currentStep} class:active={n === currentStep}></div>
    {/each}
  </div>

  <div class="cew-body">
    <!-- Step 1: Regulation type -->
    {#if currentStep === 1}
      <h4 class="cew-step-title">1. Select Regulation Type</h4>
      <div class="cew-reg-options">
        {#each regOptions as opt}
          <label class="cew-reg-option" class:selected={regType === opt.value}>
            <input type="radio" bind:group={regType} value={opt.value} name="cew-regtype" />
            <div class="cew-reg-content">
              <div class="cew-reg-label">{opt.label}</div>
              <div class="cew-reg-desc">{opt.description}</div>
            </div>
          </label>
        {/each}
      </div>

    <!-- Step 2: Scope -->
    {:else if currentStep === 2}
      <h4 class="cew-step-title">2. Select Scope</h4>

      {#if regType === 'fda_part11'}
        <p class="cew-hint">Choose the date range of audit history to include in the signed attestation bundle.</p>
        <div class="form-row">
          <div class="form-group">
            <label for="cew-from">From Date *</label>
            <input id="cew-from" type="date" bind:value={fromDate} />
          </div>
          <div class="form-group">
            <label for="cew-to">To Date *</label>
            <input id="cew-to" type="date" bind:value={toDate} />
          </div>
        </div>
        {#if fromDate && toDate && fromDate > toDate}
          <p class="cew-error-text">"From" date must be on or before "To" date.</p>
        {/if}

      {:else if regType === 'usda'}
        <p class="cew-hint">Select the specimens to include on the plant movement permit application, and name the authorized scientist.</p>
        <div class="form-group">
          <label for="cew-scientist">Authorized Scientist *</label>
          <input id="cew-scientist" type="text" bind:value={authorizedScientist} placeholder="Full name of the scientist authorizing this permit" />
        </div>
        <div class="form-group">
          <label for="cew-specimen-search">Search Specimens</label>
          <input id="cew-specimen-search" type="text" bind:value={specimenSearch} placeholder="Filter by accession number or species…" />
        </div>
        {#if loadingSpecimens}
          <p class="cew-hint">Loading specimens…</p>
        {:else if specimenLoadError}
          <p class="cew-error-text">{specimenLoadError}</p>
        {:else if filteredSpecimens.length === 0}
          <p class="cew-hint">No specimens found.</p>
        {:else}
          <div class="cew-specimen-list">
            {#each filteredSpecimens as s}
              <label class="cew-specimen-row">
                <input type="checkbox" checked={usdaSelectedIds.has(s.id)} onclick={() => toggleUsdaSpecimen(s.id)} />
                <span>{specimenLabel(s)}</span>
              </label>
            {/each}
          </div>
          <p class="cew-hint">{usdaSelectedIds.size} specimen{usdaSelectedIds.size === 1 ? '' : 's'} selected.</p>
        {/if}

      {:else if regType === 'cites'}
        <p class="cew-hint">Select the root specimen whose full chain of custody will be exported, and confirm the CITES Appendix.</p>
        <div class="form-group">
          <label for="cew-specimen-search">Search Specimens</label>
          <input id="cew-specimen-search" type="text" bind:value={specimenSearch} placeholder="Filter by accession number or species…" />
        </div>
        {#if loadingSpecimens}
          <p class="cew-hint">Loading specimens…</p>
        {:else if specimenLoadError}
          <p class="cew-error-text">{specimenLoadError}</p>
        {:else if filteredSpecimens.length === 0}
          <p class="cew-hint">No specimens found.</p>
        {:else}
          <div class="form-group">
            <label for="cew-root-specimen">Root Specimen *</label>
            <select id="cew-root-specimen" bind:value={citesRootId}>
              <option value="">Select specimen…</option>
              {#each filteredSpecimens as s}
                <option value={s.id}>{specimenLabel(s)}</option>
              {/each}
            </select>
          </div>
        {/if}
        <div class="form-group">
          <label for="cew-appendix">CITES Appendix *</label>
          <select id="cew-appendix" bind:value={citesAppendix}>
            <option value="">Select appendix…</option>
            <option value="Appendix I">Appendix I</option>
            <option value="Appendix II">Appendix II</option>
            <option value="Appendix III">Appendix III</option>
          </select>
        </div>
        <p class="cew-note">
          Note: SteloPTC does not maintain a live CITES species database. Please confirm the correct
          appendix for this species yourself from an official reference (e.g. the CITES Appendices
          published by the CITES Secretariat) before proceeding.
        </p>
      {/if}

    <!-- Step 3: Preview -->
    {:else if currentStep === 3}
      <h4 class="cew-step-title">3. Preview</h4>
      <p class="cew-hint">Review the selections below. This is a read-only summary of what you picked — no data has been generated yet.</p>

      {#if regType === 'fda_part11'}
        <div class="cew-review-grid">
          <span class="cew-rg-label">Regulation</span><span>FDA 21 CFR Part 11</span>
          <span class="cew-rg-label">From Date</span><span>{fromDate}</span>
          <span class="cew-rg-label">To Date</span><span>{toDate}</span>
        </div>
      {:else if regType === 'usda'}
        <div class="cew-review-grid">
          <span class="cew-rg-label">Regulation</span><span>USDA APHIS (PPQ Form 526)</span>
          <span class="cew-rg-label">Authorized Scientist</span><span>{authorizedScientist}</span>
          <span class="cew-rg-label">Specimens ({usdaSelectedSpecimens.length})</span>
          <span>
            {#if usdaSelectedSpecimens.length > 0}
              <ul class="cew-review-list">
                {#each usdaSelectedSpecimens as s}
                  <li>{s.accession_number}</li>
                {/each}
              </ul>
            {:else}
              —
            {/if}
          </span>
        </div>
      {:else if regType === 'cites'}
        <div class="cew-review-grid">
          <span class="cew-rg-label">Regulation</span><span>CITES Species Provenance Dossier</span>
          <span class="cew-rg-label">Root Specimen</span><span>{citesRootSpecimen ? specimenLabel(citesRootSpecimen) : '—'}</span>
          <span class="cew-rg-label">CITES Appendix</span><span>{citesAppendix}</span>
        </div>
      {/if}

    <!-- Step 4: Signing key (Part 11 only) -->
    {:else if currentStep === 4}
      <h4 class="cew-step-title">4. Signing Key</h4>
      {#if loadingKey}
        <p class="cew-hint">Loading signing key…</p>
      {:else if keyError}
        <p class="cew-error-text">{keyError}</p>
        <button class="btn btn-sm" onclick={loadSigningKey}>Retry</button>
      {:else if signingKey}
        <div class="cew-key-box">
          <code class="cew-key-value">{truncKey(signingKey)}</code>
          <button class="btn btn-sm" onclick={copyFullKey}>{keyCopied ? '✓ Copied' : 'Copy full key'}</button>
        </div>
        <p class="cew-note">
          This key is generated once and reused for all Part 11 exports; the public key is bundled in
          every export so inspectors can verify signatures against it directly.
        </p>
      {/if}

    <!-- Step 5: Confirm and generate -->
    {:else if currentStep === 5}
      <h4 class="cew-step-title">5. Confirm and Generate</h4>

      {#if !result}
        <p class="cew-hint">Ready to generate the export. This may take a moment for larger date ranges or specimen sets.</p>
        <button class="btn btn-primary" onclick={handleGenerate} disabled={generating}>
          {generating ? 'Generating…' : 'Generate Export'}
        </button>
        {#if generateError}
          <p class="cew-error-text" style="margin-top:10px;">{generateError}</p>
        {/if}
      {:else}
        <div class="cew-success-box">
          <div class="cew-success-icon">&#10003;</div>
          <div>
            <p class="cew-success-title">Export generated successfully</p>
            <p class="cew-success-detail">Saved to: <code>{result.file_path}</code></p>
            <p class="cew-success-detail">Size: {formatSize(result.size_bytes)}</p>
          </div>
        </div>
      {/if}
    {/if}
  </div>

  <div class="cew-footer">
    <button class="btn btn-sm" onclick={() => currentStep > 1 ? handleBack() : resetAndClose()}>
      {currentStep === 1 ? 'Cancel' : '← Back'}
    </button>
    {#if currentStep < TOTAL_STEPS}
      <button class="btn btn-sm btn-primary" onclick={handleNext} disabled={!canAdvance()}>Next →</button>
    {:else}
      <button class="btn btn-sm" onclick={resetAndClose}>{result ? 'Done' : 'Cancel'}</button>
    {/if}
  </div>
</div>

<style>
  .cew-panel {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
    margin-bottom: var(--space-4);
    overflow: hidden;
  }

  .cew-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: var(--space-5) var(--space-5) 0;
  }

  .cew-title {
    font-size: var(--font-size-xl);
    font-weight: 700;
    color: var(--color-text-strong);
    margin-bottom: 2px;
  }

  .cew-steps {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
  }

  .cew-close-btn {
    background: none;
    border: none;
    font-size: 16px;
    color: var(--color-text-faint);
    cursor: pointer;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
    min-height: 36px;
    min-width: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .cew-close-btn:hover { background: var(--color-fill-track); color: var(--color-text); }

  .cew-progress {
    display: flex;
    gap: 6px;
    padding: var(--space-3) var(--space-5);
  }
  .cew-dot {
    flex: 1;
    height: 4px;
    border-radius: 2px;
    background: var(--color-fill-track);
    transition: background 0.2s;
  }
  .cew-dot.done { background: var(--color-success); }
  .cew-dot.active { background: var(--color-accent); }

  .cew-body {
    padding: 0 var(--space-5);
    min-height: 200px;
  }

  .cew-footer {
    display: flex;
    justify-content: space-between;
    padding: var(--space-4) var(--space-5) var(--space-5);
    margin-top: var(--space-4);
  }

  .cew-step-title {
    font-size: var(--font-size-lg);
    font-weight: 600;
    margin-bottom: var(--space-4);
    color: var(--color-text-strong);
  }

  .cew-hint {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    margin-bottom: var(--space-3);
  }

  .cew-note {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    background: color-mix(in srgb, var(--color-warn) 10%, var(--color-surface));
    border: 1px solid color-mix(in srgb, var(--color-warn) 30%, transparent);
    border-radius: var(--radius-md);
    padding: var(--space-3);
    margin-top: var(--space-3);
    line-height: 1.5;
  }

  .cew-error-text {
    font-size: var(--font-size-sm);
    color: var(--color-danger);
    margin: var(--space-2) 0;
  }

  .cew-reg-options {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .cew-reg-option {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-3) var(--space-4);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .cew-reg-option:hover { background: var(--color-fill-track); }
  .cew-reg-option.selected {
    border-color: var(--color-accent);
    background: color-mix(in srgb, var(--color-accent) 6%, var(--color-surface));
  }
  .cew-reg-option input[type="radio"] { margin-top: 3px; flex-shrink: 0; }

  .cew-reg-label {
    font-weight: 600;
    font-size: var(--font-size-md);
    color: var(--color-text-strong);
  }
  .cew-reg-desc {
    font-size: var(--font-size-sm);
    color: var(--color-text-muted);
    margin-top: 2px;
  }

  .cew-specimen-list {
    max-height: 220px;
    overflow-y: auto;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-2);
  }

  .cew-specimen-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    font-size: var(--font-size-base);
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .cew-specimen-row:hover { background: var(--color-fill-track); }

  .cew-review-grid {
    display: grid;
    grid-template-columns: 160px 1fr;
    gap: var(--space-2) var(--space-3);
    font-size: var(--font-size-base);
  }
  .cew-rg-label { font-weight: 600; color: var(--color-text-muted); }

  .cew-review-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .cew-key-box {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    background: var(--color-fill-track);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    padding: var(--space-3);
  }
  .cew-key-value {
    font-family: 'SF Mono', 'Fira Code', Consolas, monospace;
    font-size: var(--font-size-base);
    color: var(--color-text);
    word-break: break-all;
  }

  .cew-success-box {
    display: flex;
    align-items: flex-start;
    gap: var(--space-3);
    background: color-mix(in srgb, var(--color-success) 8%, var(--color-surface));
    border: 1px solid color-mix(in srgb, var(--color-success) 30%, transparent);
    border-radius: var(--radius-md);
    padding: var(--space-4);
  }
  .cew-success-icon {
    font-size: 22px;
    color: var(--color-success);
    line-height: 1;
  }
  .cew-success-title {
    font-weight: 700;
    color: var(--color-text-strong);
    margin-bottom: var(--space-2);
  }
  .cew-success-detail {
    font-size: var(--font-size-sm);
    color: var(--color-text);
    margin-top: 2px;
    word-break: break-all;
  }
  .cew-success-detail code {
    font-family: 'SF Mono', 'Fira Code', Consolas, monospace;
    font-size: var(--font-size-sm);
  }
</style>
