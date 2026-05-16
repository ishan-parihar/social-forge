import { api } from './client';

export interface Integration {
  id: string; provider_identifier: string; provider_name: string;
  profile_name?: string; profile_picture?: string; profile_url?: string;
  disabled: boolean; refresh_needed: boolean;
  posting_times?: { time: number }[];
}

export interface TimeslotEntry {
  time: number; // minutes from midnight
}

export const integrationsApi = {
  list: () => api.get<{ integrations: Integration[] }>("/api/integrations"),
  connect: (provider: string) => api.get<{ url: string; state: string }>(`/api/integrations/connect/${provider}`),
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
};
