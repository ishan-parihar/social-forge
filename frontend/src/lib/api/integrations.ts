import { api } from './client';

export interface Integration {
  id: string; provider_identifier: string; provider_name: string;
  internal_id: string;
  profile_name?: string; profile_picture?: string; profile_url?: string;
  disabled: boolean; refresh_needed: boolean;
  posting_times?: { time: number }[];
}

export interface TimeslotEntry {
  time: number; // minutes from midnight
}

export interface ConnectApiKeyRequest {
  provider: string;
  api_key: string;
  instance_url?: string;
  label?: string;
  verification_code?: string;
}

export interface ConnectWeb3Request {
  provider: string;
  address: string;
  label?: string;
}

export interface PageInfo {
  id: string; name: string; access_token?: string;
  picture?: string; username?: string;
}

export const integrationsApi = {
  list: () => api.get<{ integrations: Integration[] }>("/api/integrations"),
  connect: (provider: string) => api.get<{ url: string; state: string }>(`/api/integrations/connect/${provider}`),
  connectApiKey: (body: ConnectApiKeyRequest) =>
    api.post<{ integration: Integration }>("/api/integrations/connect/api-key", body),
  connectWeb3: (body: ConnectWeb3Request) =>
    api.post<{ integration: Integration }>("/api/integrations/connect/web3", body),
  disconnect: (id: string) => api.del<{ deleted: boolean }>(`/api/integrations/${id}`),
  updateTimeslots: (id: string, timeslots: TimeslotEntry[]) =>
    api.put<{ success: boolean; posting_times: TimeslotEntry[] }>(
      `/api/integrations/${id}/timeslots`,
      { timeslots }
    ),
  toggleDisable: (id: string, disabled: boolean) =>
    api.put<{ success: boolean; disabled: boolean }>(
      `/api/integrations/${id}/disable`,
      { disabled }
    ),
  refresh: (id: string) =>
    api.post<{ success: boolean }>(`/api/integrations/${id}/refresh`),
  availablePages: (integrationId: string) =>
    api.get<{ pages: PageInfo[]; parent_integration_id: string; provider: string }>(
      `/api/integrations/${integrationId}/available-pages`
    ),
  connectPage: (parentIntegrationId: string, pageId: string) =>
    api.post<{ integration: Integration; parent_id: string }>(
      `/api/integrations/${parentIntegrationId}/connect-page/${pageId}`
    ),
};
