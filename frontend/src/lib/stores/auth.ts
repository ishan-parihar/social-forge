// Svelte 5 module-level reactivity replaces Context
import { writable } from "svelte/store";

export interface User {
  id: string; email: string; name: string;
}

// Use Svelte writable store for auth (needs cross-component subscription via $ syntax)
export const currentUser = writable<User | null>(null);
export const isAuthenticated = writable<boolean>(false);

// Helper: set auth state from login/register response
export function setAuth(user: User, token: string) {
  localStorage.setItem("token", token);
  currentUser.set(user);
  isAuthenticated.set(true);
}

export function clearAuth() {
  localStorage.removeItem("token");
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
