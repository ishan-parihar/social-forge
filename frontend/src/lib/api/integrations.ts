import { api } from './client';

export interface Integration {
  id: string; provider_identifier: string; provider_name: string;
  profile_name?: string; profile_picture?: string; profile_url?: string;
  disabled: boolean; refresh_needed: boolean;
}

export const integrationsApi = {
  list: () => api.get<{ integrations: Integration[] }>("/api/integrations"),
  connect: (provider: string) => api.get<{ url: string; state: string }>(`/api/integrations/connect/${provider}`),
  disconnect: (id: string) => api.del<{ deleted: boolean }>(`/api/integrations/${id}`),
};
