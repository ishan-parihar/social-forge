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

export interface Webhook {
  id: string;
  name: string;
  url: string;
  secret?: string | null;
  event_types: string[];
  is_active: boolean;
  last_triggered_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface WebhookDelivery {
  id: string;
  webhook_id: string;
  event_type: string;
  status: string;
  status_code: number | null;
  response_body: string | null;
  attempted_at: string;
  delivered_at: string | null;
}

export const developerApi = {
  listKeys: () => api.get<ApiKeySummary[]>("/api/developer/api-keys"),
  createKey: (name: string, expires_at?: string) =>
    api.post<ApiKeyCreated>("/api/developer/api-keys", { name, expires_at }),
  revokeKey: (id: string) => api.del(`/api/developer/api-keys/${id}`),
  regenerateKey: (id: string) =>
    api.post<ApiKeyCreated>(`/api/developer/api-keys/${id}/regenerate`),

  // Webhooks (backend already exists)
  listWebhooks: () => api.get<Webhook[]>("/api/webhooks"),
  createWebhook: (data: { name: string; url: string; secret?: string; event_types: string[] }) =>
    api.post<Webhook>("/api/webhooks", data),
  updateWebhook: (id: string, data: { name?: string; url?: string; secret?: string; event_types?: string[]; is_active?: boolean }) =>
    api.put<Webhook>(`/api/webhooks/${id}`, data),
  deleteWebhook: (id: string) => api.del(`/api/webhooks/${id}`),
  testWebhook: (id: string) => api.post<{ status_code: number; response_body: string; delivery: WebhookDelivery }>(`/api/webhooks/${id}/test`),
  getDeliveries: (id: string) => api.get<WebhookDelivery[]>(`/api/webhooks/${id}/deliveries`),
};
