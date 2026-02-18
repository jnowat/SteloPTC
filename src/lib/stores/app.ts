import { writable, derived } from 'svelte/store';

export type View = 'dashboard' | 'specimens' | 'specimen-detail' | 'media' | 'reminders' | 'compliance' | 'species' | 'inventory' | 'users' | 'audit' | 'error-log' | 'settings';

export const currentView = writable<View>('dashboard');
export const selectedSpecimenId = writable<string | null>(null);

function getInitialDarkMode(): boolean {
  try {
    const stored = localStorage.getItem('stelo_dark');
    if (stored !== null) return stored === 'true';
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  } catch {
    return false;
  }
}

export const darkMode = writable<boolean>(getInitialDarkMode());

function getInitialDevMode(): boolean {
  try { return localStorage.getItem('stelo_devmode') === 'true'; } catch { return false; }
}
export const devMode = writable<boolean>(getInitialDevMode());
devMode.subscribe((value) => {
  try { localStorage.setItem('stelo_devmode', String(value)); } catch { /* noop */ }
});

export const notifications = writable<Array<{ id: string; message: string; type: 'info' | 'warning' | 'error' | 'success'; timestamp: number }>>([]);

// Error log state
export const unreadErrorCount = writable<number>(0);

darkMode.subscribe((value) => {
  try {
    localStorage.setItem('stelo_dark', String(value));
    if (value) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  } catch {
    // DOM not available
  }
});

export function navigateTo(view: View, specimenId?: string) {
  currentView.set(view);
  if (specimenId) {
    selectedSpecimenId.set(specimenId);
  }
}

// Import is deferred to avoid circular deps — logError is wired up in App.svelte after auth
let _logErrorFn: ((title: string, message: string, module?: string, payload?: string) => void) | null = null;
export function setErrorLogger(fn: typeof _logErrorFn) {
  _logErrorFn = fn;
}

export function addNotification(message: string, type: 'info' | 'warning' | 'error' | 'success' = 'info') {
  const id = crypto.randomUUID();
  notifications.update((n) => [...n, { id, message, type, timestamp: Date.now() }]);
  setTimeout(() => {
    notifications.update((n) => n.filter((x) => x.id !== id));
  }, 5000);

  // Fire-and-forget error persistence
  if ((type === 'error' || type === 'warning') && _logErrorFn) {
    try {
      _logErrorFn(type === 'error' ? 'Application Error' : 'Warning', message, undefined, undefined);
    } catch {
      // never throw from notification path
    }
  }
}

// Called from components with full context (module + payload)
export function addErrorWithContext(
  title: string,
  message: string,
  module: string,
  formPayload?: Record<string, unknown>
) {
  addNotification(message, 'error');
  if (_logErrorFn) {
    try {
      _logErrorFn(title, message, module, formPayload ? JSON.stringify(formPayload, null, 2) : undefined);
    } catch {
      // never throw
    }
  }
}
