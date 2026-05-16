import { api } from './client';

export interface RssFeed {
  id: string;
  user_id: string;
  feed_url: string;
  integration_id: string;
  title: string;
  last_polled_at: string | null;
  poll_interval_min: number;
  enabled: boolean;
  use_ai_summary: boolean;
  created_at: string;
  updated_at: string;
}

export interface RssFeedItem {
  id: string;
  feed_id: string;
  guid: string;
  title: string;
  url: string;
  published_at: string | null;
  is_imported: boolean;
  post_id: string | null;
  created_at: string;
}

export const rssApi = {
  list: () => api.get<RssFeed[]>("/api/rss/feeds"),
  create: (data: { feed_url: string; integration_id: string; title?: string; use_ai_summary?: boolean }) =>
    api.post<RssFeed>("/api/rss/feeds", data),
  delete: (id: string) => api.del(`/api/rss/feeds/${id}`),
  toggle: (id: string) => api.put<RssFeed>(`/api/rss/feeds/${id}/toggle`, {}),
  poll: (id: string) => api.post<{ success: boolean; new_items?: number }>(`/api/rss/feeds/${id}/poll`, {}),
  listItems: (id: string) => api.get<RssFeedItem[]>(`/api/rss/feeds/${id}/items`),
  importItem: (feedId: string, guid: string) =>
    api.post<{ success: boolean; post_id?: string }>(`/api/rss/feeds/${feedId}/items/${encodeURIComponent(guid)}/import`, {}),
};
