import { api } from './client';

export interface MediaAttachment {
  url: string;
  mime_type: string;
  alt: string | null;
  poster_url?: string | null;
}

export interface EngagementMetrics {
  likes: number | null;
  comments: number | null;
  shares: number | null;
  views: number | null;
  saves: number | null;
  quotes: number | null;
  reposts: number | null;
  replies: number | null;
  reactions: Record<string, number> | null;
  upvotes: number | null;
  downvotes: number | null;
  upvote_ratio: number | null;
  awards: number | null;
}

export interface FeedPost {
  id: string;
  provider: string;
  platform_post_id: string;
  text: string;
  author_name: string | null;
  author_handle: string | null;
  author_avatar: string | null;
  created_at: string;
  url: string | null;
  media: MediaAttachment[];
  metadata: Record<string, unknown> | null;
  imported_at: string;
  engagement: EngagementMetrics | null;
}

export interface FeedAccount {
  provider: string;
  author_name: string | null;
  author_handle: string | null;
  author_avatar: string | null;
}

export interface FeedResponse {
  posts: FeedPost[];
  next_cursor: string | null;
  has_more: boolean;
}

export interface AnalyticsResponse {
  total_likes: number;
  total_comments: number;
  total_shares: number;
  total_views: number;
  total_reposts: number;
  total_replies: number;
  total_upvotes: number;
  total_awards: number;
  posts_with_engagement: number;
}

export const feedApi = {
  list: (cursor?: string, provider?: string, authorHandle?: string, limit = 20) => {
    const params = new URLSearchParams();
    params.set('limit', String(limit));
    if (cursor) params.set('cursor', cursor);
    if (provider) params.set('provider', provider);
    if (authorHandle) params.set('author_handle', authorHandle);
    return api.get<FeedResponse>(`/api/feed?${params.toString()}`);
  },
  import: () => {
    return api.post<{ imported: number; status: string }>('/api/feed/import');
  },
  analytics: (provider?: string) => {
    const params = new URLSearchParams();
    if (provider) params.set('provider', provider);
    return api.get<AnalyticsResponse>(`/api/feed/analytics?${params.toString()}`);
  },
  accounts: () => {
    return api.get<FeedAccount[]>('/api/feed/accounts');
  },
};
