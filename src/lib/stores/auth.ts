import { writable, derived } from 'svelte/store';

export interface User {
  id: string;
  username: string;
  display_name: string;
  email: string | null;
  role: string;
  is_active: boolean;
}

function getStoredToken(): string | null {
  try {
    return localStorage.getItem('stelo_token');
  } catch {
    return null;
  }
}

export const token = writable<string | null>(getStoredToken());
export const currentUser = writable<User | null>(null);
export const isLoggedIn = derived(token, ($token) => $token !== null);
// Tracks whether we are restoring a saved session on startup
export const initializing = writable<boolean>(getStoredToken() !== null);

token.subscribe((value) => {
  try {
    if (value) {
      localStorage.setItem('stelo_token', value);
    } else {
      localStorage.removeItem('stelo_token');
    }
  } catch {
    // localStorage unavailable
  }
});

export function setAuth(newToken: string, user: User) {
  token.set(newToken);
  currentUser.set(user);
  initializing.set(false);
}

export function clearAuth() {
  token.set(null);
  currentUser.set(null);
  initializing.set(false);
}
