// WP-62: distinguishes "running inside the Tauri desktop webview" from
// "running as a plain installed/browser PWA" — used to gate service-worker
// registration (and any other web-only behavior) so it never runs inside
// the desktop app. Tauri v2 injects `window.__TAURI_INTERNALS__` into every
// webview it controls; that global is absent in a normal browser tab.
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
