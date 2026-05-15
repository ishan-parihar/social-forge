import { writable } from "svelte/store";

export interface User {
  id: string; email: string; name: string;
}

// Use Svelte writable store for auth (needs cross-component subscription via $ syntax)
export const currentUser = writable<User | null>(null);
export const isAuthenticated = writable<boolean>(false);

const isBrowser = typeof window !== "undefined";

// Helper: set auth state from login/register response
export function setAuth(user: User, token: string) {
  if (isBrowser) localStorage.setItem("token", token);
  currentUser.set(user);
  isAuthenticated.set(true);
}

export function clearAuth() {
  if (isBrowser) localStorage.removeItem("token");
  currentUser.set(null);
  isAuthenticated.set(false);
}

// Helper: check existing token on app load
export function initializeAuth() {
  const token = typeof window !== "undefined" ? localStorage.getItem("token") : null;
  if (token) {
    isAuthenticated.set(true);
    // Optionally fetch /api/auth/me to populate user
  }
}
