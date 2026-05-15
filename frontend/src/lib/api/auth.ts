import { api } from './client';

let _token: string | null = null;
const isBrowser = typeof window !== "undefined" && typeof localStorage !== "undefined";

export function getToken() {
  return _token;
}
export function setToken(t: string | null) {
  _token = t;
  if (isBrowser) {
    if (t) localStorage.setItem("token", t);
    else localStorage.removeItem("token");
  }
}
export function loadToken() {
  if (isBrowser) {
    const t = localStorage.getItem("token");
    if (t) _token = t;
  }
  return _token;
}

export const auth = {
  register: (e: string, p: string, n: string) =>
    api.post<{ token: string; user: { id: string; email: string; name: string } }>("/api/auth/register", { email: e, password: p, name: n }),
  login: (e: string, p: string) =>
    api.post<{ token: string; user: { id: string; email: string; name: string } }>("/api/auth/login", { email: e, password: p }),
  me: () => api.get<{ id: string; email: string; name: string }>("/api/auth/me"),
};
