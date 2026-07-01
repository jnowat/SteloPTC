import { mount } from 'svelte';
import App from './App.svelte';
import './lib/styles/tokens.css';
import { isTauri } from './lib/isTauri';

// WP-62: register the PWA service worker only when NOT running inside the
// Tauri desktop webview — the SW must never intercept Tauri's `ipc://`
// requests. `vite-plugin-pwa` is configured with `injectRegister: false`
// specifically so this is the only place registration can happen.
if (!isTauri() && 'serviceWorker' in navigator) {
  import('virtual:pwa-register')
    .then(({ registerSW }) => registerSW({ immediate: true }))
    .catch(() => {
      // Non-fatal — the app works fully without a service worker, it just
      // won't be installable/offline-capable in this browser session.
    });
}

let app: ReturnType<typeof mount> | undefined;

try {
  const target = document.getElementById('app');
  if (!target) {
    throw new Error('Mount target #app not found in DOM');
  }

  // Mount Svelte app into #app
  app = mount(App, { target });

  // Only hide the loader AFTER Svelte has successfully mounted.
  // Uses a CSS class so the loader element stays in the DOM as a fallback.
  document.body.classList.add('app-ready');
} catch (e: unknown) {
  // If mount fails, the loader stays visible (it was never hidden)
  // and the global error handler from index.html will display the error.
  const msg = e instanceof Error ? e.message : String(e);
  console.error('SteloPTC failed to start:', msg);

  // Also call showAppError directly in case the global handler missed it
  const showErr = (window as any).showAppError;
  if (typeof showErr === 'function') {
    showErr(msg);
  }
}

export default app;
