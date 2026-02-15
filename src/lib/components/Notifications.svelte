<script lang="ts">
  import { notifications } from '../stores/app';
</script>

{#if $notifications.length > 0}
  <div class="notifications">
    {#each $notifications as notif (notif.id)}
      <div class="notif notif-{notif.type}">
        {notif.message}
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
    padding: 12px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    animation: slideIn 0.2s ease-out;
    max-width: 360px;
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
