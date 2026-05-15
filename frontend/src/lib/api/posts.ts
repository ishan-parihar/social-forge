import { api } from './client';

export interface PostSummary {
  id: string; integration_name: string; state: string;
  content: string; title?: string; scheduled_at?: string;
  platform_post_url?: string; error_message?: string; created_at: string;
}
export interface PostDetail {
  id: string; integration_id: string; state: string;
  content: string; title?: string; scheduled_at?: string;
  published_at?: string; platform_post_url?: string; error_message?: string; created_at: string;
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
  create: (d: { integration_ids: string[]; content: string; title?: string; scheduled_at?: string }) =>
    api.post<PostDetail>("/api/posts", d),
  schedule: (id: string, at: string) => api.post<PostDetail>(`/api/posts/${id}/schedule`, { scheduled_at: at }),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/posts/${id}`),
};
