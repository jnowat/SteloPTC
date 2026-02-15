import { writable, derived } from 'svelte/store';

export interface User {
  id: string;
  username: string;
  display_name: string;
  email: string | null;
  role: string;
  is_active: boolean;
}

export const token = writable<string | null>(localStorage.getItem('stelo_token'));
export const currentUser = writable<User | null>(null);
export const isLoggedIn = derived(token, ($token) => $token !== null);

token.subscribe((value) => {
  if (value) {
    localStorage.setItem('stelo_token', value);
  } else {
    localStorage.removeItem('stelo_token');
  }
});

export function setAuth(newToken: string, user: User) {
  token.set(newToken);
  currentUser.set(user);
}

export function clearAuth() {
  token.set(null);
  currentUser.set(null);
}
