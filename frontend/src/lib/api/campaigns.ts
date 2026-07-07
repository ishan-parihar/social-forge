// Campaigns API — CRUD for campaign entities + post stage management.
// Phase 7, v20. v22 Phase 6: expanded with status, progress_metric,
// audience_persona, content_pillars, budget_cents, kpi_targets.

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
  // v22 Phase 6: expanded fields.
  status: 'active' | 'paused' | 'archived' | 'completed';
  progress_metric: 'posts' | 'engagement' | 'reach' | 'followers' | 'custom' | null;
  progress_target: number | null;
  audience_persona: Record<string, unknown> | null;
  content_pillars: Array<{ title: string; description?: string; tags?: string[] }> | null;
  budget_cents: number | null;
  kpi_targets: Record<string, unknown> | null;
  sort_order: number;
  deleted_at: string | null;
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
  // v22 Phase 6: optional expanded fields.
  status?: 'active' | 'paused' | 'archived' | 'completed';
  progress_metric?: 'posts' | 'engagement' | 'reach' | 'followers' | 'custom';
  progress_target?: number;
  audience_persona?: Record<string, unknown>;
  content_pillars?: Array<{ title: string; description?: string; tags?: string[] }>;
  budget_cents?: number;
  kpi_targets?: Record<string, unknown>;
}

export interface UpdateCampaignInput {
  name?: string;
  description?: string;
  color?: string;
  start_date?: string;
  end_date?: string;
  goal?: string;
  // v22 Phase 6: expanded fields.
  status?: 'active' | 'paused' | 'archived' | 'completed';
  progress_metric?: 'posts' | 'engagement' | 'reach' | 'followers' | 'custom';
  progress_target?: number;
  audience_persona?: Record<string, unknown>;
  content_pillars?: Array<{ title: string; description?: string; tags?: string[] }>;
  budget_cents?: number;
  kpi_targets?: Record<string, unknown>;
}

export const campaignsApi = {
  list: () => api.get<Campaign[]>('/api/campaigns'),
  create: (data: CreateCampaignInput) => api.post<Campaign>('/api/campaigns', data),
  update: (id: string, data: UpdateCampaignInput) => api.put<Campaign>(`/api/campaigns/${id}`, data),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/campaigns/${id}`),
  updateStage: (
    postId: string,
    state: string,
    campaignId?: string,
    // v25-3: optional kanban metadata. When omitted, the existing value is
    // preserved (COALESCE on the backend). Pass an empty string to
    // `due_date` to explicitly clear it.
    kanbanMeta?: {
      kanban_substate?: string;
      priority?: string;
      due_date?: string;
    },
  ) =>
    api.patch<{ updated: boolean }>(`/api/posts/${postId}/stage`, {
      state,
      campaign_id: campaignId,
      kanban_substate: kanbanMeta?.kanban_substate,
      priority: kanbanMeta?.priority,
      due_date: kanbanMeta?.due_date,
    }),
};
