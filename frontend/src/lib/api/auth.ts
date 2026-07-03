import { api } from './client';

export const auth = {
  /** Verify password against APP_PASSWORD, set sf_session cookie. */
  login: (password: string) =>
    api.post<{ authenticated: boolean }>("/api/auth/login", { password }),
  /** Clear the session cookie. */
  logout: () =>
    api.post<{ logged_out: boolean }>("/api/auth/logout"),
  /** Confirm the session cookie is still valid. */
  me: () =>
    api.get<{ authenticated: boolean; user_id: string }>("/api/auth/me"),
};
