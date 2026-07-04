import { api } from './client';

export interface Webhook {
  id: string;
  name: string;
  url: string;
  secret: string | null;
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
  payload: Record<string, unknown>;
  status_code: number | null;
  response_body: string | null;
  attempted_at: string;
  delivered_at: string | null;
}

export const webhooksApi = {
  list: () =>
    api.get<{ webhooks: Webhook[] }>('/api/webhooks'),
  get: (id: string) =>
    api.get<{ webhook: Webhook }>(`/api/webhooks/${id}`),
  create: (data: { name: string; url: string; secret?: string; event_types: string[] }) =>
    api.post<{ webhook: Webhook }>('/api/webhooks', data),
  update: (id: string, updates: Partial<Webhook>) =>
    api.put<{ webhook: Webhook }>(`/api/webhooks/${id}`, updates),
  delete: (id: string) =>
    api.del<{ success: boolean }>(`/api/webhooks/${id}`),
  test: (id: string) =>
    api.post<{ success: boolean }>(`/api/webhooks/${id}/test`),
  deliveries: (id: string) =>
    api.get<{ deliveries: WebhookDelivery[] }>(`/api/webhooks/${id}/deliveries`),
};
