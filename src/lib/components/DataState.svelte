<script lang="ts">
  import type { Snippet } from 'svelte';
  import SkeletonLoader from './SkeletonLoader.svelte';
  import EmptyState from './EmptyState.svelte';

  let {
    loading = false,
    error = null,
    empty = false,
    rows = 5,
    cols = 4,
    emptyIcon = '📭',
    emptyTitle = 'No data',
    emptyMessage = '',
    emptyActionLabel = '',
    onemptyaction,
    onretry,
    children,
  }: {
    loading?: boolean;
    error?: string | null;
    empty?: boolean;
    rows?: number;
    cols?: number;
    emptyIcon?: string;
    emptyTitle?: string;
    emptyMessage?: string;
    emptyActionLabel?: string;
    onemptyaction?: () => void;
    onretry?: () => void;
    children?: Snippet;
  } = $props();
</script>

{#if loading}
  <div class="card" style="overflow-x:auto;">
    <SkeletonLoader {rows} {cols} />
  </div>
{:else if error}
  <div class="ds-error card" role="alert" aria-live="polite">
    <div class="ds-error-icon" aria-hidden="true">⚠</div>
    <p class="ds-error-title">Failed to load data</p>
    <p class="ds-error-msg">{error}</p>
    {#if onretry}
      <button class="btn btn-sm ds-retry" onclick={onretry}>Try again</button>
    {/if}
  </div>
{:else if empty}
  <EmptyState
    icon={emptyIcon}
    title={emptyTitle}
    message={emptyMessage}
    actionLabel={emptyActionLabel}
    onaction={onemptyaction}
  />
{:else if children}
  {@render children()}
{/if}

<style>
  .ds-error {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 56px 24px;
    text-align: center;
    gap: 8px;
  }
  .ds-error-icon {
    font-size: 36px;
    line-height: 1;
    margin-bottom: 4px;
    color: var(--color-danger, #dc2626);
    opacity: 0.8;
  }
  .ds-error-title {
    font-size: 16px;
    font-weight: 600;
    color: var(--color-text-muted, #6b7280);
    margin: 0;
  }
  .ds-error-msg {
    font-size: 13px;
    color: var(--color-text-faint, #9ca3af);
    margin: 0;
    max-width: 360px;
    line-height: 1.5;
  }
  .ds-retry {
    margin-top: 4px;
  }

  :global([data-theme="dark"]) .ds-error-title { color: #94a3b8; }
  :global([data-theme="dark"]) .ds-error-msg { color: #64748b; }
</style>
