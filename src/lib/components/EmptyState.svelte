<script lang="ts">
  let {
    icon = '📭',
    title,
    message = '',
    actionLabel = '',
    onaction,
  }: {
    icon?: string;
    title: string;
    message?: string;
    actionLabel?: string;
    onaction?: () => void;
  } = $props();
</script>

<div class="empty-state-block">
  <div class="empty-icon" aria-hidden="true">{icon}</div>
  <p class="empty-title">{title}</p>
  {#if message}
    <p class="empty-message">{message}</p>
  {/if}
  {#if actionLabel && onaction}
    <button class="btn btn-primary empty-action" onclick={onaction}>{actionLabel}</button>
  {/if}
</div>

<style>
  .empty-state-block {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 56px 24px;
    text-align: center;
    gap: 8px;
  }
  .empty-icon {
    font-size: 40px;
    line-height: 1;
    margin-bottom: 4px;
    opacity: 0.7;
  }
  .empty-title {
    font-size: 16px;
    font-weight: 600;
    color: #6b7280;
    margin: 0;
  }
  .empty-message {
    font-size: 13px;
    color: #9ca3af;
    margin: 0;
    max-width: 320px;
  }
  .empty-action {
    margin-top: 8px;
  }

  @media (prefers-color-scheme: dark) {
    .empty-title { color: #9ca3af; }
    .empty-message { color: #6b7280; }
  }
</style>
