import { api } from './client';

export interface Comment {
  id: string;
  post_id: string;
  platform: string;
  author_name: string | null;
  author_avatar: string | null;
  text: string;
  created_at: string;
  like_count: number;
  reply_count: number;
  is_resolved: boolean;
  integration_id: string;
}

export const commentsApi = {
  list: (params?: { integration_id?: string; resolved?: boolean; limit?: number; offset?: number }) => {
    const q = new URLSearchParams();
    if (params?.integration_id) q.set('integration_id', params.integration_id);
    if (params?.resolved !== undefined) q.set('resolved', String(params.resolved));
    if (params?.limit) q.set('limit', String(params.limit));
    if (params?.offset) q.set('offset', String(params.offset));
    return api.get<{ comments: Comment[]; total: number }>(`/api/comments?${q}`);
  },
  resolve: (id: string) =>
    api.post<{ success: boolean }>(`/api/comments/${id}/resolve`),
  reply: (id: string, content: string) =>
    api.post<{ success: boolean }>(`/api/comments/${id}/reply`, { content }),
};
