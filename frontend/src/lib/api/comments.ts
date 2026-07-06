import { api } from './client';

export interface Comment {
  id: string;
  post_id: string;
  platform: string;
  author: string;
  content: string;
  status: string;
  post_content: string;
  created_at: string;
}

export const commentsApi = {
  list: (params?: { platform?: string; status?: string }) => {
    const q = new URLSearchParams();
    if (params?.platform) q.set('platform', params.platform);
    if (params?.status) q.set('status', params.status);
    return api.get<{ comments: Comment[] }>(`/api/comments?${q}`);
  },
  resolve: (id: string) =>
    api.post<{ success: boolean }>(`/api/comments/${id}/resolve`),
  reply: (id: string, content: string) =>
    api.post<{ success: boolean }>(`/api/comments/${id}/reply`, { content }),
};
