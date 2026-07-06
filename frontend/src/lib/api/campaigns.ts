// Campaigns API — CRUD for campaign entities + post stage management.
// Phase 7, v20.

import { api } from './client';

export interface Campaign {
  id: string;
  user_id: string;
  name: string;
  description: string | null;
  color: string;
  start_date: string | null;
  end_date: string | null;
  goal: string | null;
  created_at: string;
  updated_at: string;
  post_count: number | null;
}

export interface CreateCampaignInput {
  name: string;
  description?: string;
  color?: string;
  start_date?: string;
  end_date?: string;
  goal?: string;
}

export interface UpdateCampaignInput {
  name?: string;
  description?: string;
  color?: string;
  start_date?: string;
  end_date?: string;
  goal?: string;
}

export const campaignsApi = {
  list: () => api.get<Campaign[]>('/api/campaigns'),
  create: (data: CreateCampaignInput) => api.post<Campaign>('/api/campaigns', data),
  update: (id: string, data: UpdateCampaignInput) => api.put<Campaign>(`/api/campaigns/${id}`, data),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/campaigns/${id}`),
  updateStage: (postId: string, state: string, campaignId?: string) =>
    api.patch<{ updated: boolean }>(`/api/posts/${postId}/stage`, { state, campaign_id: campaignId }),
};
