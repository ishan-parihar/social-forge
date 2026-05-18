import { api } from './client';

export interface Integration {
  id: string; provider_identifier: string; provider_name: string;
  internal_id: string;
  profile_name?: string; profile_picture?: string; profile_url?: string;
  disabled: boolean; refresh_needed: boolean;
  posting_times?: { time: number }[];
  auth_method?: string;
  root_internal_id?: string;
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
  connectXCookie: (auth_token: string, ct0: string) =>
    api.post<{ integration: Integration }>("/api/integrations/connect/x-cookie", { auth_token, ct0 }),
  connectGithubPat: (pat: string, label?: string) =>
    api.post<{ integration: Integration }>("/api/integrations/connect/github-pat", { pat, label }),
  verifyOneTimeToken: (provider: string, code: string) =>
    api.post<{ url: string; state: string }>(`/api/integrations/connect/${provider}/verify`, { code }),
  whatsappPair: (phone_number: string) =>
    api.post<{ pair_code: string; expires_in: number }>("/api/integrations/connect/whatsapp/pair", { phone_number }),
  whatsappStatus: () =>
    api.get<{ authenticated: boolean; jid?: string }>("/api/integrations/connect/whatsapp/status"),
  telegramUserRequestCode: (phone: string) =>
    api.post<{ status: string }>("/api/integrations/connect/telegram-user/request-code", { phone }),
  telegramUserSignIn: (code: string) =>
    api.post<{ integration: Integration }>("/api/integrations/connect/telegram-user/sign-in", { code }),
  connectTelegramBotToken: (token: string) =>
    api.post<{ integration: Integration }>("/api/integrations/connect/telegram-bot/token", { token }),
};
