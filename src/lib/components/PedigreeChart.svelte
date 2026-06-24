<script lang="ts">
  import { onMount } from 'svelte';
  import {
    getStrainAncestry,
    getStrainDescendants,
    exportStrainPedigree,
  } from '$lib/api';
  import type { PedigreeNode, PedigreeEdge, StrainSummary } from '$lib/api';

  let {
    strainId,
    onstrainclick,
  }: {
    strainId: string;
    onstrainclick?: (id: string) => void;
  } = $props();

  type ViewMode = 'ancestors' | 'descendants';

  interface FlatNode {
    strain: StrainSummary;
    depth: number;
    edge: PedigreeEdge | null;
  }

  let view = $state<ViewMode>('ancestors');
  let ancestryRoot = $state<PedigreeNode | null>(null);
  let descendantsRoot = $state<PedigreeNode | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let exporting = $state(false);

  function flattenNode(node: PedigreeNode, mode: ViewMode): FlatNode[] {
    const result: FlatNode[] = [];
    function recurse(n: PedigreeNode) {
      result.push({ strain: n.strain, depth: n.depth, edge: n.edge });
      const sub = mode === 'ancestors' ? n.parents : n.children;
      for (const s of sub) recurse(s);
    }
    recurse(node);
    return result;
  }

  let flatNodes = $derived(
    view === 'ancestors' && ancestryRoot
      ? flattenNode(ancestryRoot, 'ancestors')
      : view === 'descendants' && descendantsRoot
        ? flattenNode(descendantsRoot, 'descendants')
        : []
  );

  onMount(async () => {
    try {
      const [anc, desc] = await Promise.all([
        getStrainAncestry(strainId, 5),
        getStrainDescendants(strainId, 5),
      ]);
      ancestryRoot = anc;
      descendantsRoot = desc;
    } catch (e) {
      error = e instanceof Error ? e.message : 'Failed to load pedigree';
    } finally {
      loading = false;
    }
  });

  async function handleExport() {
    exporting = true;
    try {
      const data = await exportStrainPedigree(strainId, 5);
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `pedigree-${strainId}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = e instanceof Error ? e.message : 'Export failed';
    } finally {
      exporting = false;
    }
  }

  function statusLabel(status: string): string {
    const map: Record<string, string> = {
      unverified: 'Unverified',
      claimed: 'Claimed',
      confirmed_manual: 'Confirmed (Manual)',
      confirmed_genomic: 'Confirmed (Genomic)',
    };
    return map[status] ?? status;
  }

  function ancestorCount(): number {
    if (!ancestryRoot) return 0;
    return flattenNode(ancestryRoot, 'ancestors').length - 1;
  }

  function descendantCount(): number {
    if (!descendantsRoot) return 0;
    return flattenNode(descendantsRoot, 'descendants').length - 1;
  }
</script>

<div class="pedigree-chart">
  <div class="pedigree-toolbar">
    <div class="view-toggle" role="tablist" aria-label="Pedigree view">
      <button
        role="tab"
        aria-selected={view === 'ancestors'}
        class:active={view === 'ancestors'}
        onclick={() => (view = 'ancestors')}
      >
        Ancestors
        {#if !loading && ancestryRoot}
          <span class="count-badge">{ancestorCount()}</span>
        {/if}
      </button>
      <button
        role="tab"
        aria-selected={view === 'descendants'}
        class:active={view === 'descendants'}
        onclick={() => (view = 'descendants')}
      >
        Descendants
        {#if !loading && descendantsRoot}
          <span class="count-badge">{descendantCount()}</span>
        {/if}
      </button>
    </div>
    <button
      class="btn-export"
      onclick={handleExport}
      disabled={exporting || loading}
      aria-label="Export pedigree as JSON"
    >
      {exporting ? 'Exporting…' : 'Export JSON'}
    </button>
  </div>

  {#if loading}
    <div class="pedigree-state">Loading pedigree…</div>
  {:else if error}
    <div class="pedigree-state pedigree-error">{error}</div>
  {:else if flatNodes.length === 0}
    <div class="pedigree-state">No {view === 'ancestors' ? 'ancestry' : 'descendants'} recorded.</div>
  {:else}
    <div class="pedigree-tree" role="tree" aria-label="Strain pedigree">
      {#each flatNodes as node (node.strain.id + '-' + node.depth)}
        <div
          class="pedigree-row"
          role="treeitem"
          aria-selected={false}
          aria-level={node.depth + 1}
          style:padding-left="{node.depth * 28}px"
        >
          {#if node.depth > 0}
            <span class="connector" aria-hidden="true">└─</span>
          {/if}
          <button
            class="node-card"
            class:is-root={node.depth === 0}
            class:is-archived={node.strain.is_archived}
            onclick={() => onstrainclick?.(node.strain.id)}
            title="Navigate to {node.strain.name}"
          >
            <span class="node-name">{node.strain.name}</span>
            <span class="node-code">[{node.strain.code}]</span>
            <span class="status-badge status-{node.strain.status}">
              {statusLabel(node.strain.status)}
            </span>
            {#if node.strain.is_hybrid}
              <span class="hybrid-badge">Hybrid</span>
            {/if}
            <span class="spec-count" title="{node.strain.specimen_count} live specimens">
              {node.strain.specimen_count} sp.
            </span>
            {#if node.edge?.parent_role}
              <span class="role-badge">{node.edge.parent_role}</span>
            {/if}
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .pedigree-chart {
    background: var(--surface, #fff);
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 8px;
    overflow: hidden;
    font-size: 13px;
  }

  .pedigree-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border, #e5e7eb);
    background: var(--surface-alt, #f9fafb);
    gap: 8px;
  }

  .view-toggle {
    display: flex;
    gap: 4px;
  }

  .view-toggle button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-muted, #6b7280);
    font-size: 13px;
    cursor: pointer;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
  }

  .view-toggle button.active {
    background: var(--color-primary, #6366f1);
    color: #fff;
    border-color: var(--color-primary, #6366f1);
  }

  .count-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 18px;
    height: 18px;
    padding: 0 5px;
    border-radius: 9px;
    font-size: 11px;
    font-weight: 600;
    background: rgba(255,255,255,0.3);
    color: inherit;
  }

  .view-toggle button:not(.active) .count-badge {
    background: var(--border, #e5e7eb);
    color: var(--color-text-muted, #6b7280);
  }

  .btn-export {
    padding: 5px 12px;
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 6px;
    background: transparent;
    color: var(--color-text-muted, #6b7280);
    font-size: 13px;
    cursor: pointer;
    white-space: nowrap;
    transition: background 0.12s;
  }

  .btn-export:hover:not(:disabled) {
    background: var(--surface, #fff);
  }

  .btn-export:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .pedigree-tree {
    padding: 12px 14px;
    overflow-x: auto;
    min-height: 80px;
  }

  .pedigree-state {
    padding: 28px 16px;
    text-align: center;
    color: var(--color-text-muted, #6b7280);
    font-size: 13px;
  }

  .pedigree-error {
    color: var(--color-danger, #ef4444);
  }

  .pedigree-row {
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 3px;
  }

  .connector {
    color: var(--color-text-faint, #9ca3af);
    font-family: monospace;
    font-size: 12px;
    flex-shrink: 0;
    user-select: none;
  }

  .node-card {
    display: inline-flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 5px 10px;
    border: 1px solid var(--border, #e5e7eb);
    border-radius: 6px;
    background: var(--surface, #fff);
    cursor: pointer;
    text-align: left;
    font-size: 13px;
    transition: border-color 0.12s, background 0.12s;
  }

  .node-card:hover {
    border-color: var(--color-primary, #6366f1);
    background: var(--color-primary-subtle, #eef2ff);
  }

  .node-card.is-root {
    border-color: var(--color-primary, #6366f1);
    font-weight: 600;
  }

  .node-card.is-archived {
    opacity: 0.5;
  }

  .node-name {
    font-weight: 600;
    color: var(--color-text, #111827);
  }

  .node-code {
    color: var(--color-text-muted, #6b7280);
    font-family: monospace;
    font-size: 12px;
  }

  .spec-count {
    color: var(--color-text-faint, #9ca3af);
    font-size: 11px;
  }

  .hybrid-badge {
    background: #e0e7ff;
    color: #4338ca;
    border-radius: 4px;
    padding: 1px 5px;
    font-size: 11px;
    font-weight: 500;
  }

  .role-badge {
    background: #f3f4f6;
    color: #6b7280;
    border-radius: 4px;
    padding: 1px 5px;
    font-size: 10px;
  }

  .status-badge {
    border-radius: 4px;
    padding: 1px 5px;
    font-size: 11px;
    font-weight: 500;
  }

  .status-unverified     { background: #f3f4f6; color: #6b7280; }
  .status-claimed        { background: #dbeafe; color: #1d4ed8; }
  .status-confirmed_manual   { background: #fef3c7; color: #b45309; }
  .status-confirmed_genomic  { background: #dcfce7; color: #166534; }

  :global([data-theme="dark"]) .pedigree-chart {
    background: #1e293b;
    border-color: #374151;
  }
  :global([data-theme="dark"]) .pedigree-toolbar {
    background: #0f172a;
    border-color: #374151;
  }
  :global([data-theme="dark"]) .node-card {
    background: #1e293b;
    border-color: #374151;
  }
  :global([data-theme="dark"]) .node-card:hover {
    background: #1e2d4d;
    border-color: var(--color-primary, #6366f1);
  }
  :global([data-theme="dark"]) .node-card.is-root {
    border-color: var(--color-primary, #818cf8);
  }
  :global([data-theme="dark"]) .node-name { color: #f1f5f9; }
  :global([data-theme="dark"]) .node-code { color: #94a3b8; }
  :global([data-theme="dark"]) .view-toggle button { border-color: #374151; color: #94a3b8; }
  :global([data-theme="dark"]) .btn-export { border-color: #374151; color: #94a3b8; }
  :global([data-theme="dark"]) .status-unverified { background: #374151; color: #9ca3af; }
  :global([data-theme="dark"]) .status-claimed { background: #1e3a5f; color: #60a5fa; }
  :global([data-theme="dark"]) .status-confirmed_manual { background: #4a2c00; color: #fbbf24; }
  :global([data-theme="dark"]) .status-confirmed_genomic { background: #14532d; color: #4ade80; }
  :global([data-theme="dark"]) .hybrid-badge { background: #312e81; color: #a5b4fc; }
  :global([data-theme="dark"]) .role-badge { background: #374151; color: #9ca3af; }
</style>
