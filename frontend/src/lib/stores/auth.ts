const DEFAULT_USER = { id: "22222222-2222-2222-2222-222222222222", email: "user@socialforge.local", name: "User" };

export const currentUser = { subscribe: (run: (v: typeof DEFAULT_USER) => void) => { run(DEFAULT_USER); return () => {}; } };
export const isAuthenticated = { subscribe: (run: (v: boolean) => void) => { run(true); return () => {}; } };

export function setAuth(_user: unknown, _token: string) {}
export function clearAuth() {}
export function initializeAuth() {}
