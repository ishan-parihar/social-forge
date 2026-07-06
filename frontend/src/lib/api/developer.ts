import { api } from "./client";

export interface ApiKeySummary {
  id: string;
  name: string;
  key_prefix: string;
  last_used_at: string | null;
  expires_at: string | null;
  is_active: boolean;
  created_at: string;
}

export interface ApiKeyCreated extends ApiKeySummary {
  full_key: string;
}

export const developerApi = {
  listKeys: () => api.get<ApiKeySummary[]>("/api/developer/api-keys"),
  createKey: (name: string, expires_at?: string) =>
    api.post<ApiKeyCreated>("/api/developer/api-keys", { name, expires_at }),
  revokeKey: (id: string) => api.del(`/api/developer/api-keys/${id}`),
  regenerateKey: (id: string) =>
    api.post<ApiKeyCreated>(`/api/developer/api-keys/${id}/regenerate`),
};
