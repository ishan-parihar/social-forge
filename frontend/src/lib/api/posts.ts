import { api } from './client';
import type { Tag } from './tags';

export interface PostSummary {
  id: string; integration_name: string; state: string;
  content: string; title?: string; scheduled_at?: string;
  platform_post_url?: string; error_message?: string; created_at: string;
  tags?: Tag[];
  repeat_interval_days?: number | null;
  repeat_end_date?: string | null;
  group_id?: string | null;
  first_comment?: string | null;
  sequence?: number;
}
export interface PostDetail {
  id: string; integration_id: string; integration_name: string; state: string;
  content: string; title?: string; scheduled_at?: string;
  published_at?: string; platform_post_url?: string; error_message?: string; created_at: string;
  tags?: Tag[];
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
}

export const postsApi = {
  list: (params?: { state?: string; limit?: number; offset?: number }) => {
    const q = new URLSearchParams();
    if (params?.state) q.set("state", params.state);
    if (params?.limit) q.set("limit", String(params.limit));
    if (params?.offset) q.set("offset", String(params.offset));
    return api.get<{ posts: PostSummary[]; total: number }>(`/api/posts?${q}`);
  },
  get: (id: string) => api.get<PostDetail>(`/api/posts/${id}`),
  create: (d: { integration_ids: string[]; content: string; title?: string; scheduled_at?: string; tag_ids?: string[]; first_comment?: string }) =>
    api.post<{ posts: PostSummary[]; group_id?: string }>("/api/posts", d),
  createThread: (d: ThreadRequest) =>
    api.post<{ posts: PostSummary[]; group_id: string }>("/api/posts/thread", d),
  update: (id: string, d: { content: string; title?: string }) =>
    api.put<PostDetail>(`/api/posts/${id}`, d),
  schedule: (id: string, at: string) => api.post<PostDetail>(`/api/posts/${id}/schedule`, { scheduled_at: at }),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/posts/${id}`),
  setTags: (id: string, tag_ids: string[]) => api.put<PostDetail>(`/api/posts/${id}/tags`, { tag_ids }),
  findSlot: (integrationId?: string) => {
    const q = integrationId ? `?integration_id=${integrationId}` : "";
    return api.get<{ date: string }>(`/api/posts/find-slot${q}`);
  },
  repeat: (id: string, intervalDays: number, endDate: string) =>
    api.post<{ group_id: string; count: number; post_ids: string[]; scheduled_dates: string[] }>(
      `/api/posts/${id}/repeat`, { interval_days: intervalDays, end_date: endDate }
    ),
};
