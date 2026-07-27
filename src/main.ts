import { mount } from 'svelte';
import App from './App.svelte';
import './lib/styles/tokens.css';
import { isTauri } from './lib/isTauri';

// Prototype-pollution hardening, applied before any other module can run.
//
// `xlsx` (SheetJS) 0.18.5 carries GHSA-4r6h-8v6p-xvw6, a prototype-pollution
// flaw reachable from `XLSX.read` on a user-supplied workbook — exactly what
// the Import screen does. The advisory is fixed in 0.19.3+, but SheetJS stopped
// publishing to the npm registry at 0.18.5, so `npm update` can never resolve
// it; the fixed builds live at cdn.sheetjs.com. Until that dependency move
// happens, freezing the prototype turns the pollution write into a no-op
// (sloppy mode) or a throw that rejects the malicious file (strict mode, which
// is what ES modules run in). Either way the payload cannot reach the rest of
// the app — and this WebView has `withGlobalTauri`, so "the rest of the app"
// includes the full IPC surface.
//
// Verified not to disturb normal operation: a full xlsx write/read/sheet_to_json
// round-trip passes with the prototype frozen (see exportUtils tests), and no
// dependency here extends Object.prototype.
//
// This is defence in depth, NOT a substitute for updating the dependency.
Object.freeze(Object.prototype);

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
