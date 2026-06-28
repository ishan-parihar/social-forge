import { api } from './client';

export const auth = {
  me: () => api.get<{ id: string; email: string; name: string }>("/api/auth/me"),
};
