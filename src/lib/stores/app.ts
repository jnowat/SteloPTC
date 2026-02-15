import { writable } from 'svelte/store';

export type View = 'dashboard' | 'specimens' | 'specimen-detail' | 'media' | 'reminders' | 'compliance' | 'species' | 'users' | 'audit' | 'settings';

export const currentView = writable<View>('dashboard');
export const selectedSpecimenId = writable<string | null>(null);
export const darkMode = writable<boolean>(
  localStorage.getItem('stelo_dark') === 'true' ||
  window.matchMedia('(prefers-color-scheme: dark)').matches
);
export const notifications = writable<Array<{ id: string; message: string; type: 'info' | 'warning' | 'error' | 'success'; timestamp: number }>>([]);

darkMode.subscribe((value) => {
  localStorage.setItem('stelo_dark', String(value));
  if (value) {
    document.documentElement.classList.add('dark');
  } else {
    document.documentElement.classList.remove('dark');
  }
});

export function navigateTo(view: View, specimenId?: string) {
  currentView.set(view);
  if (specimenId) {
    selectedSpecimenId.set(specimenId);
  }
}

export function addNotification(message: string, type: 'info' | 'warning' | 'error' | 'success' = 'info') {
  const id = crypto.randomUUID();
  notifications.update((n) => [...n, { id, message, type, timestamp: Date.now() }]);
  setTimeout(() => {
    notifications.update((n) => n.filter((x) => x.id !== id));
  }, 5000);
}
