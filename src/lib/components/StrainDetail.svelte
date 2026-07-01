<script lang="ts">
  import { onMount } from 'svelte';
  import { getStrain, getGenerationalStats, getStrainAncestry, RESTRICTED_MARKER, type GenerationalStats } from '../api';
  import { addNotification } from '../stores/app';

  let { strainId, onclose }: { strainId: string; onclose: () => void } = $props();

  let strain = $state<any>(null);
  let genStats = $state<GenerationalStats[]>([]);
  let ancestry = $state<any>(null);
  let loading = $state(true);
  let activeTab = $state<'overview' | 'generations' | 'pedigree'>('overview');

  onMount(async () => {
    await load();
  });

  async function load() {
    loading = true;
    try {
      const [s, stats] = await Promise.all([
        getStrain(strainId),
        getGenerationalStats(strainId).catch(() => []),
      ]);
      strain = s;
      genStats = stats;
      if (s?.is_hybrid) {
        ancestry = await getStrainAncestry(strainId, 3).catch(() => null);
      }
    } catch (e: any) {
      addNotification(e.message, 'error');
    } finally {
      loading = false;
    }
  }

  function statusLabel(s: string): string {
    return s === 'unverified' ? 'Unverified' :
           s === 'claimed' ? 'Claimed' :
           s === 'confirmed_manual' ? 'Confirmed (Manual)' :
           s === 'confirmed_genomic' ? 'Confirmed (Genomic)' : s;
  }

  let totalSpecimens = $derived(genStats.reduce((n, g) => n + g.specimen_count, 0));
  let totalHealthy = $derived(genStats.reduce((n, g) => n + g.healthy_count, 0));

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') onclose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="sd-backdrop" onclick={onclose}>
  <div class="sd-panel" role="dialog" aria-modal="true" aria-label="Strain Detail">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div onclick={(e) => e.stopPropagation()}>

      <div class="sd-header">
        <div class="sd-title-block">
          {#if loading}
            <h2 class="sd-title">Loading…</h2>
          {:else if strain}
            <h2 class="sd-title">{strain.name}</h2>
            <div class="sd-meta-row">
              <code class="sd-code">{strain.code}</code>
              <span class="sd-type">{strain.strain_type ?? '—'}</span>
              {#if strain.is_hybrid}
                <span class="sd-chip hybrid">Hybrid</span>
              {/if}
              {#if strain.is_archived}
                <span class="sd-chip archived">Archived</span>
              {/if}
            </div>
          {/if}
        </div>
        <button class="sd-close" onclick={onclose} aria-label="Close">&#10005;</button>
      </div>

      {#if !loading && strain?.is_cross_species}
        <div class="cross-species-banner" role="alert">
          <span class="cs-icon">&#9888;</span>
          <div>
            <strong>Cross-Species Hybrid — Permanent Warning</strong>
            <p>
              This strain was created via an authorised cross-species hybridisation event.
              This warning cannot be removed. Verify regulatory compliance before any propagation,
              distribution, or publication involving this strain.
            </p>
          </div>
        </div>
      {/if}

      {#if !loading && strain}
        <div class="sd-tabs">
          <button class="sd-tab" class:active={activeTab === 'overview'} onclick={() => (activeTab = 'overview')}>Overview</button>
          {#if strain.is_hybrid}
            <button class="sd-tab" class:active={activeTab === 'generations'} onclick={() => (activeTab = 'generations')}>Generations</button>
            <button class="sd-tab" class:active={activeTab === 'pedigree'} onclick={() => (activeTab = 'pedigree')}>Pedigree</button>
          {/if}
        </div>

        <div class="sd-body">

          <!-- Overview tab -->
          {#if activeTab === 'overview'}
            <div class="detail-grid">
              <span class="dg-label">Name</span><span>{strain.name}</span>
              <span class="dg-label">Code</span><span><code>{strain.code}</code></span>
              <span class="dg-label">Type</span><span>{strain.strain_type ?? '—'}</span>
              <span class="dg-label">Status</span>
              <span class="status-badge status-{strain.status}">{statusLabel(strain.status)}</span>
              <span class="dg-label">Specimens</span><span>{strain.specimen_count ?? 0}</span>
              <span class="dg-label">Created</span><span>{strain.created_at?.split('T')[0] ?? '—'}</span>
              {#if strain.claimed_by}
                <span class="dg-label">Claimed By</span><span>{strain.claimed_by}</span>
              {/if}
              {#if strain.confirmation_basis}
                <span class="dg-label">Basis</span><span class="basis-text">{strain.confirmation_basis}</span>
              {/if}
              {#if strain.genomic_fingerprint}
                <span class="dg-label">Fingerprint</span>
                <span>
                  {#if strain.genomic_fingerprint === RESTRICTED_MARKER}
                    <span class="restricted-chip" title="Your role does not have permission to view this field">🔒 Restricted</span>
                  {:else}
                    <code class="fingerprint">{strain.genomic_fingerprint}</code>
                  {/if}
                </span>
              {/if}
            </div>

            {#if strain.is_hybrid}
              <div class="hybrid-section">
                <h3 class="section-title">Hybridisation Event</h3>
                {#if strain.generation_label}
                  <div class="gen-label-display">
                    <span class="gen-badge" class:backcross={strain.generation_label?.startsWith('BC')}>
                      {strain.generation_label}
                    </span>
                    {#if strain.generation_label?.startsWith('BC')}
                      <span class="gen-note">Backcross generation</span>
                    {:else}
                      <span class="gen-note">Filial generation</span>
                    {/if}
                  </div>
                {/if}
                {#if strain.notes}
                  <div class="strain-notes">
                    <h4 class="notes-label">Event Notes</h4>
                    <pre class="notes-pre">{strain.notes}</pre>
                  </div>
                {/if}
              </div>
            {/if}

          <!-- Generations tab -->
          {:else if activeTab === 'generations'}
            {#if genStats.length === 0}
              <div class="empty-tab">No specimens with generation labels found in this strain's descendants.</div>
            {:else}
              <div class="gen-summary">
                <div class="gen-summary-item">
                  <span class="gs-value">{totalSpecimens}</span>
                  <span class="gs-label">Total Specimens</span>
                </div>
                <div class="gen-summary-item">
                  <span class="gs-value healthy">{totalHealthy}</span>
                  <span class="gs-label">Healthy</span>
                </div>
                <div class="gen-summary-item">
                  <span class="gs-value problem">{totalSpecimens - totalHealthy}</span>
                  <span class="gs-label">Other Status</span>
                </div>
              </div>

              <div class="gen-table-wrap">
                <table class="gen-table">
                  <thead>
                    <tr>
                      <th>Generation</th>
                      <th class="num">Specimens</th>
                      <th class="num">Healthy</th>
                      <th class="num">Problem</th>
                      <th>Health %</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each genStats as g}
                      {@const pct = g.specimen_count > 0 ? Math.round((g.healthy_count / g.specimen_count) * 100) : 0}
                      <tr>
                        <td>
                          <span class="gen-badge-sm" class:backcross={g.generation_label.startsWith('BC')}>
                            {g.generation_label}
                          </span>
                        </td>
                        <td class="num">{g.specimen_count}</td>
                        <td class="num healthy">{g.healthy_count}</td>
                        <td class="num problem">{g.problem_count}</td>
                        <td>
                          <div class="health-bar-wrap">
                            <div class="health-bar" style="width:{pct}%"></div>
                            <span class="health-pct">{pct}%</span>
                          </div>
                        </td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              </div>
            {/if}

          <!-- Pedigree tab -->
          {:else if activeTab === 'pedigree'}
            {#if !ancestry || !ancestry.nodes || ancestry.nodes.length === 0}
              <div class="empty-tab">No ancestry data available for this strain.</div>
            {:else}
              <div class="pedigree-list">
                {#each ancestry.nodes as node}
                  <div class="pedigree-entry">
                    <div class="pe-depth">Depth {node.depth ?? 0}</div>
                    <div class="pe-info">
                      <div class="pe-code">{node.code}</div>
                      <div class="pe-name">{node.name}</div>
                      {#if node.is_hybrid}
                        <span class="pe-chip">Hybrid</span>
                      {/if}
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {/if}

        </div>
      {:else if loading}
        <div class="sd-body"><div class="loading-msg">Loading strain details…</div></div>
      {/if}

    </div>
  </div>
</div>

<style>
  .restricted-chip {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 10px;
    background: #f1f5f9;
    color: #64748b;
    font-size: 12px;
    font-style: italic;
  }
  :global(.dark) .restricted-chip {
    background: #1e293b;
    color: #94a3b8;
  }
  .sd-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1050;
    background: rgba(0,0,0,0.45);
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
    padding: 0;
  }
  .sd-panel {
    background: white;
    width: 100%;
    max-width: 520px;
    height: 100%;
    overflow-y: auto;
    box-shadow: -4px 0 32px rgba(0,0,0,0.2);
    display: flex;
    flex-direction: column;
  }
  .sd-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 24px 24px 16px;
    border-bottom: 1px solid #e2e8f0;
    position: sticky;
    top: 0;
    background: white;
    z-index: 2;
  }
  .sd-title { font-size: 18px; font-weight: 700; margin-bottom: 6px; }
  .sd-meta-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .sd-code {
    font-family: monospace;
    font-size: 12px;
    background: #f1f5f9;
    padding: 2px 6px;
    border-radius: 4px;
    color: #475569;
  }
  .sd-type { font-size: 12px; color: #6b7280; }
  .sd-chip {
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 10px;
    font-weight: 600;
  }
  .sd-chip.hybrid { background: #ede9fe; color: #6d28d9; }
  .sd-chip.archived { background: #f1f5f9; color: #64748b; }
  .sd-close {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: #6b7280;
    padding: 4px 8px;
    border-radius: 4px;
    flex-shrink: 0;
    min-height: 0;
  }
  .sd-close:hover { background: #f3f4f6; }

  /* Cross-species permanent banner */
  .cross-species-banner {
    display: flex;
    gap: 12px;
    align-items: flex-start;
    background: #fef2f2;
    border-bottom: 2px solid #f87171;
    padding: 14px 24px;
  }
  .cs-icon { font-size: 22px; color: #dc2626; flex-shrink: 0; }
  .cross-species-banner strong { font-size: 13px; color: #991b1b; display: block; margin-bottom: 4px; }
  .cross-species-banner p { font-size: 12px; color: #b91c1c; margin: 0; line-height: 1.5; }

  .sd-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid #e2e8f0;
    padding: 0 24px;
    background: white;
    position: sticky;
    top: 89px;
    z-index: 1;
  }
  .sd-tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    padding: 10px 14px;
    font-size: 13px;
    font-weight: 500;
    color: #6b7280;
    cursor: pointer;
    margin-bottom: -1px;
    min-height: 0;
    text-transform: none;
    letter-spacing: 0;
  }
  .sd-tab:hover { color: #374151; }
  .sd-tab.active { color: #2563eb; border-bottom-color: #2563eb; }

  .sd-body { padding: 20px 24px 32px; flex: 1; }

  .detail-grid {
    display: grid;
    grid-template-columns: 130px 1fr;
    gap: 8px 12px;
    font-size: 13px;
    margin-bottom: 20px;
  }
  .dg-label { font-weight: 600; color: #6b7280; }

  .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 11px;
    font-weight: 600;
  }
  .status-unverified { background: #f1f5f9; color: #475569; }
  .status-claimed { background: #dbeafe; color: #1e40af; }
  .status-confirmed_manual { background: #fef3c7; color: #92400e; }
  .status-confirmed_genomic { background: #dcfce7; color: #166534; }

  .basis-text { font-size: 12px; font-style: italic; color: #374151; }
  .fingerprint { font-size: 11px; word-break: break-all; }

  /* Hybrid section */
  .hybrid-section { border-top: 1px solid #e2e8f0; padding-top: 16px; margin-top: 4px; }
  .section-title { font-size: 14px; font-weight: 700; color: #1e293b; margin-bottom: 12px; }

  .gen-label-display {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  .gen-badge {
    font-size: 14px;
    font-weight: 700;
    padding: 4px 12px;
    border-radius: 6px;
    background: #dbeafe;
    color: #1e40af;
    letter-spacing: 0.5px;
  }
  .gen-badge.backcross { background: #fef3c7; color: #92400e; }
  .gen-note { font-size: 12px; color: #6b7280; }

  .notes-label { font-size: 12px; font-weight: 600; color: #6b7280; margin-bottom: 6px; }
  .notes-pre {
    font-size: 12px;
    font-family: monospace;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 6px;
    padding: 10px 12px;
    white-space: pre-wrap;
    word-break: break-word;
    color: #374151;
    margin: 0;
  }

  /* Generations tab */
  .gen-summary {
    display: flex;
    gap: 16px;
    margin-bottom: 20px;
    padding: 12px 16px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
  }
  .gen-summary-item { display: flex; flex-direction: column; align-items: center; flex: 1; }
  .gs-value { font-size: 24px; font-weight: 700; color: #1e293b; }
  .gs-value.healthy { color: #16a34a; }
  .gs-value.problem { color: #dc2626; }
  .gs-label { font-size: 11px; color: #6b7280; margin-top: 2px; }

  .gen-table-wrap { overflow-x: auto; }
  .gen-table { width: 100%; font-size: 13px; border-collapse: collapse; }
  .gen-table th { text-align: left; font-size: 11px; font-weight: 600; color: #6b7280; padding: 6px 8px; border-bottom: 1px solid #e2e8f0; }
  .gen-table td { padding: 8px 8px; border-bottom: 1px solid #f1f5f9; vertical-align: middle; }
  .gen-table .num { text-align: right; }
  .gen-table .healthy { color: #16a34a; font-weight: 600; }
  .gen-table .problem { color: #dc2626; font-weight: 600; }

  .gen-badge-sm {
    font-size: 12px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: 4px;
    background: #dbeafe;
    color: #1e40af;
  }
  .gen-badge-sm.backcross { background: #fef3c7; color: #92400e; }

  .health-bar-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 80px;
  }
  .health-bar {
    height: 6px;
    background: #16a34a;
    border-radius: 3px;
    min-width: 2px;
    max-width: 60px;
    transition: width 0.3s;
  }
  .health-pct { font-size: 11px; color: #6b7280; }

  /* Pedigree tab */
  .pedigree-list { display: flex; flex-direction: column; gap: 8px; }
  .pedigree-entry {
    display: flex;
    gap: 12px;
    align-items: center;
    padding: 10px 12px;
    background: #f8fafc;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
  }
  .pe-depth {
    font-size: 11px;
    color: #9ca3af;
    min-width: 50px;
    font-weight: 600;
  }
  .pe-code { font-weight: 700; font-size: 13px; color: #1e293b; font-family: monospace; }
  .pe-name { font-size: 12px; color: #6b7280; margin-top: 2px; }
  .pe-chip {
    font-size: 10px;
    background: #ede9fe;
    color: #6d28d9;
    border-radius: 10px;
    padding: 1px 6px;
    margin-top: 4px;
    display: inline-block;
    font-weight: 600;
  }

  .empty-tab { font-size: 13px; color: #9ca3af; font-style: italic; padding: 24px 0; text-align: center; }
  .loading-msg { font-size: 13px; color: #6b7280; padding: 24px 0; text-align: center; }

  :global(.dark) .sd-panel { background: #1e293b; color: #e2e8f0; }
  :global(.dark) .sd-header { background: #1e293b; border-bottom-color: #334155; }
  :global(.dark) .sd-close { color: #94a3b8; }
  :global(.dark) .sd-close:hover { background: #334155; }
  :global(.dark) .sd-tabs { background: #1e293b; border-bottom-color: #334155; top: 89px; }
  :global(.dark) .sd-tab { color: #94a3b8; }
  :global(.dark) .sd-tab:hover { color: #e2e8f0; }
  :global(.dark) .sd-code { background: #334155; color: #94a3b8; }
  :global(.dark) .cross-species-banner { background: #450a0a; border-bottom-color: #dc2626; }
  :global(.dark) .cross-species-banner strong { color: #fca5a5; }
  :global(.dark) .cross-species-banner p { color: #f87171; }
  :global(.dark) .detail-grid { color: #e2e8f0; }
  :global(.dark) .hybrid-section { border-top-color: #334155; }
  :global(.dark) .section-title { color: #f1f5f9; }
  :global(.dark) .notes-pre { background: #0f172a; border-color: #334155; color: #cbd5e1; }
  :global(.dark) .gen-summary { background: #0f172a; border-color: #334155; }
  :global(.dark) .gs-value { color: #f1f5f9; }
  :global(.dark) .gen-table th { color: #94a3b8; border-bottom-color: #334155; }
  :global(.dark) .gen-table td { border-bottom-color: #1e293b; }
  :global(.dark) .pedigree-entry { background: #0f172a; border-color: #334155; }
  :global(.dark) .pe-code { color: #f1f5f9; }
  :global(.dark) .pe-name { color: #94a3b8; }
  :global(.dark) .status-unverified { background: #334155; color: #94a3b8; }
  :global(.dark) .status-confirmed_manual { background: #78350f; color: #fde68a; }
  :global(.dark) .status-confirmed_genomic { background: #166534; color: #dcfce7; }
  :global(.dark) .status-claimed { background: #1e40af; color: #dbeafe; }
</style>
