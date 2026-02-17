import { mount } from 'svelte';
import App from './App.svelte';

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
