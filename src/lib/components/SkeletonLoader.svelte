<script lang="ts">
  let { rows = 5, cols = 4 }: { rows?: number; cols?: number } = $props();
</script>

<div class="skeleton-table" aria-busy="true" aria-label="Loading…">
  {#each Array(rows) as _, i}
    <div class="skeleton-row" style="animation-delay: {i * 60}ms">
      {#each Array(cols) as _, j}
        <div class="skeleton-cell" style="width: {70 + ((i * 3 + j * 7) % 25)}%"></div>
      {/each}
    </div>
  {/each}
</div>

<style>
  .skeleton-table {
    padding: 0 2px;
  }
  .skeleton-row {
    display: flex;
    gap: 16px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border, #e5e7eb);
    animation: skeleton-fade 1.4s ease-in-out infinite;
  }
  .skeleton-cell {
    height: 14px;
    border-radius: 4px;
    background: linear-gradient(90deg, var(--skeleton-base, #e5e7eb) 25%, var(--skeleton-shine, #f3f4f6) 50%, var(--skeleton-base, #e5e7eb) 75%);
    background-size: 200% 100%;
    animation: skeleton-shimmer 1.4s ease-in-out infinite;
    flex-shrink: 0;
  }

  @keyframes skeleton-shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }
  @keyframes skeleton-fade {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.7; }
  }

  /* Dark mode support — matches the app's manual toggle via data-theme */
  :global([data-theme="dark"]) .skeleton-cell {
    background: linear-gradient(90deg, #374151 25%, #4b5563 50%, #374151 75%);
    background-size: 200% 100%;
    animation: skeleton-shimmer 1.4s ease-in-out infinite;
  }
  :global([data-theme="dark"]) .skeleton-row {
    border-bottom-color: #374151;
  }
</style>
