import { api } from './client';
import type { Tag } from './tags';

export interface PostSummary {
  id: string; integration_id?: string; integration_name: string; state: string;
  content: string; title?: string; scheduled_at?: string;
  published_at?: string | null;
  platform_post_url?: string; error_message?: string; created_at: string;
  tags?: Tag[];
  repeat_interval_days?: number | null;
  repeat_end_date?: string | null;
  group_id?: string | null;
  first_comment?: string | null;
  sequence?: number;
  // v22 Phase 6: campaign_id for kanban filtering. Previously the
  // kanban accessed this via `as any` because the field didn't exist
  // on the type. Now it's a proper optional field.
  campaign_id?: string | null;
  // v22 Phase 6: kanban fields.
  kanban_substate?: string | null;
  priority?: string;
  due_date?: string | null;
  // v25-3: kanban_sort_order is now surfaced by the backend (was DB-only).
  // Used by the drag-to-reorder feature (v25-4) — included here for type
  // completeness so future iterations don't need to touch this interface.
  kanban_sort_order?: number;
  // Engagement metrics (optional — populated when analytics_cache available)
  likes?: number | null;
  comments?: number | null;
  shares?: number | null;
  impressions?: number | null;
}
export interface PostDetail {
  id: string; integration_id: string; integration_name: string; state: string;
  content: string; title?: string; scheduled_at?: string;
  published_at?: string; platform_post_url?: string; error_message?: string; created_at: string;
  tags?: Tag[];
  media?: { url: string; mime_type: string; alt?: string }[] | null;
  repeat_interval_days?: number | null;
  repeat_end_date?: string | null;
  group_id?: string | null;
  first_comment?: string | null;
  sequence?: number;
}

export interface ThreadRequest {
  content_parts: string[];
  integration_ids: string[];
  scheduled_at?: string;
  /** Delay (minutes) between each thread part. Part N publishes at scheduled_at + (N-1)*delay. */
  delay_minutes?: number;
}

export const postsApi = {
  list: (params?: {
    state?: string;
    limit?: number;
    offset?: number;
    q?: string;
    integration_ids?: string[];
    tag_ids?: string[];
    sort?: string;
  }) => {
    const q = new URLSearchParams();
    if (params?.state) q.set("state", params.state);
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.offset) q.set("offset", String(params.offset));
    if (params?.q && params.q.trim()) q.set("q", params.q.trim());
    if (params?.integration_ids?.length) q.set("integration_ids", params.integration_ids.join(","));
    if (params?.tag_ids?.length) q.set("tag_ids", params.tag_ids.join(","));
    if (params?.sort) q.set("sort", params.sort);
    return api.get<{ posts: PostSummary[]; total: number }>(`/api/posts?${q}`);
  },
  get: (id: string) => api.get<PostDetail>(`/api/posts/${id}`),
  create: (d: { integration_ids: string[]; content: string; title?: string; scheduled_at?: string; tag_ids?: string[]; first_comment?: string; media?: { id: string; url: string; mime_type: string; alt?: string }[]; overrides?: Record<string, { content?: string; settings?: Record<string, unknown> }>; settings?: Record<string, unknown> }) =>
    api.post<{ posts: PostSummary[]; group_id?: string }>("/api/posts", d),
  validate: (d: { integration_ids: string[]; content: string; title?: string; scheduled_at?: string; tag_ids?: string[]; first_comment?: string; media?: { id: string; url: string; mime_type: string; alt?: string }[]; overrides?: Record<string, { content?: string; settings?: Record<string, unknown> }>; settings?: Record<string, unknown> }) =>
    api.post<{ valid: boolean; errors: Array<{ integration_id: string; provider: string; provider_name: string; kind: string; message: string; max_length?: number; actual_length?: number }> }>("/api/posts/validate", d),
  createThread: (d: ThreadRequest) =>
    api.post<{ posts: PostSummary[]; group_id: string }>("/api/posts/thread", d),
  // v23-5: update now accepts tag_ids + first_comment (was silently
  // dropped on edit-mode save).
  update: (id: string, d: { content: string; title?: string; media?: { id: string; url: string; mime_type: string; alt?: string }[]; settings?: Record<string, unknown>; tag_ids?: string[]; first_comment?: string }) =>
    api.put<PostDetail>(`/api/posts/${id}`, d),
  schedule: (id: string, at: string) => api.post<PostDetail>(`/api/posts/${id}/schedule`, { scheduled_at: at }),
  // v24-1: unschedule — transitions a post back to draft state.
  unschedule: (id: string) => api.post<PostDetail>(`/api/posts/${id}/unschedule`, {}),
  reschedule: (id: string, scheduledAt: string, moveGroup?: boolean, action?: 'schedule' | 'update') =>
    api.put<{ rescheduled: boolean; post?: PostDetail; group_id?: string; count?: number; action?: string }>(
      `/api/posts/${id}/date`,
      {
        scheduled_at: scheduledAt,
        move_group: moveGroup || false,
        ...(action ? { action } : {}),
      }
    ),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/posts/${id}`),
  setTags: (id: string, tagIds: string[]) =>
    api.put<{ success: boolean }>(`/api/posts/${id}/tags`, { tag_ids: tagIds }),
  findSlot: (integrationId?: string) => {
    const q = integrationId ? `?integration_id=${integrationId}` : "";
    return api.get<{ date: string }>(`/api/posts/find-slot${q}`);
  },
  publish: (id: string) => api.post<PostDetail>(`/api/posts/${id}/publish`, {}),
  repeat: (id: string, intervalDays: number, endDate: string) =>
    api.post<{ group_id: string; count: number; post_ids: string[]; scheduled_dates: string[] }>(
      `/api/posts/${id}/repeat`, { interval_days: intervalDays, end_date: endDate }
    ),
  /** Fetch all sibling posts sharing a group_id (for thread/group editing). */
  getGroup: (groupId: string) =>
    api.get<PostSummary[]>(`/api/posts/group/${groupId}`),
  /** v25-4: Reorder posts within a kanban column. Sends the full new order
   *  so the backend can renumber kanban_sort_order to match. */
  reorderKanban: (state: string, orderedPostIds: string[]) =>
    api.patch<{ updated: number }>('/api/posts/kanban-reorder', {
      state,
      ordered_post_ids: orderedPostIds,
    }),
};
