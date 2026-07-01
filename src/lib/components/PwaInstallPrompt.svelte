<script lang="ts">
  // WP-62: prompts installation when the browser fires `beforeinstallprompt`.
  // Never renders inside the Tauri desktop webview — that event only fires
  // in a real browser context anyway, but the explicit guard keeps this
  // component inert-by-default if it's ever mounted somewhere unexpected.
  import { isTauri } from '../isTauri';

  let deferredPrompt = $state<any>(null);
  let dismissed = $state(false);

  function handleBeforeInstallPrompt(e: Event) {
    if (isTauri()) return;
    e.preventDefault();
    deferredPrompt = e;
  }

  async function install() {
    if (!deferredPrompt) return;
    deferredPrompt.prompt();
    await deferredPrompt.userChoice;
    deferredPrompt = null;
  }

  function dismiss() {
    dismissed = true;
    deferredPrompt = null;
  }
</script>

<svelte:window onbeforeinstallprompt={handleBeforeInstallPrompt} />

{#if deferredPrompt && !dismissed && !isTauri()}
  <div class="pwa-install-banner" role="status">
    <span>Install Stelo Lab Suite for offline access and a native-like experience.</span>
    <div class="pwa-install-actions">
      <button class="btn btn-primary btn-sm" onclick={install}>Install</button>
      <button class="btn btn-sm" aria-label="Dismiss install prompt" onclick={dismiss}>Not now</button>
    </div>
  </div>
{/if}

<style>
  .pwa-install-banner {
    position: fixed;
    left: 50%;
    bottom: 16px;
    transform: translateX(-50%);
    z-index: 3000;
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--color-surface, #0f172a);
    color: var(--color-text, #f1f5f9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    border-radius: 10px;
    padding: 10px 14px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.35);
    font-size: 13px;
    max-width: 92vw;
  }
  .pwa-install-actions {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
  }
</style>
