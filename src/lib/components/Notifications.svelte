<script lang="ts">
  import { notifications, navigateTo } from '../stores/app';

  function handleErrorClick(notif: { type: string }) {
    if (notif.type === 'error' || notif.type === 'warning') {
      navigateTo('error-log');
    }
  }
</script>

{#if $notifications.length > 0}
  <div class="notifications">
    {#each $notifications as notif (notif.id)}
      <div
        class="notif notif-{notif.type}"
        class:notif-clickable={notif.type === 'error' || notif.type === 'warning'}
        onclick={() => handleErrorClick(notif)}
        role={notif.type === 'error' || notif.type === 'warning' ? 'button' : undefined}
        tabindex={notif.type === 'error' || notif.type === 'warning' ? 0 : undefined}
        onkeydown={(e) => e.key === 'Enter' && handleErrorClick(notif)}
        title={notif.type === 'error' || notif.type === 'warning' ? 'Click to open Error Log' : undefined}
      >
        <span class="notif-msg">{notif.message}</span>
        {#if notif.type === 'error' || notif.type === 'warning'}
          <span class="notif-hint">View in Error Log →</span>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .notifications {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 1000;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .notif {
    padding: 12px 16px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    animation: slideIn 0.2s ease-out;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .notif-clickable {
    cursor: pointer;
    transition: filter 0.15s, box-shadow 0.15s;
  }
  .notif-clickable:hover {
    filter: brightness(1.08);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
  }
  .notif-msg { line-height: 1.4; }
  .notif-hint {
    font-size: 11px;
    font-weight: 600;
    opacity: 0.7;
    letter-spacing: 0.2px;
  }
  .notif-info { background: #dbeafe; color: #1e40af; }
  .notif-success { background: #dcfce7; color: #166534; }
  .notif-warning { background: #fef9c3; color: #854d0e; }
  .notif-error { background: #fef2f2; color: #991b1b; }

  @keyframes slideIn {
    from { transform: translateX(100%); opacity: 0; }
    to { transform: translateX(0); opacity: 1; }
  }
</style>
