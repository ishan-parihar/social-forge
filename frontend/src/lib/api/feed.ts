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

/**
 * Proxy an external media URL through the backend to bypass CDN CORS/referrer restrictions.
 * Only proxies known CDN domains (video.twimg.com, pbs.twimg.com, etc.).
 * Returns the proxy URL or the original URL if it's not a known CDN.
 */
export function proxyMediaUrl(url: string): string {
  // Only proxy URLs from known CDN domains that block direct browser access
  const proxyDomains = [
    'video.twimg.com',
    'pbs.twimg.com',
    'media.tenor.com',
    'i.imgur.com',
    'i.ytimg.com',
  ];
  try {
    const u = new URL(url);
    if (proxyDomains.some(d => u.hostname === d || u.hostname.endsWith('.' + d))) {
      return `/api/proxy-media?url=${encodeURIComponent(url)}`;
    }
  } catch {
    // Not a valid URL — return as-is
  }
  return url;
}

export const feedApi = {
  list: (cursor?: string, provider?: string, authorHandle?: string, limit = 20, q?: string) => {
    const params = new URLSearchParams();
    params.set('limit', String(limit));
    if (cursor) params.set('cursor', cursor);
    if (provider) params.set('provider', provider);
    if (authorHandle) params.set('author_handle', authorHandle);
    if (q && q.trim()) params.set('q', q.trim());
    return api.get<FeedResponse>(`/api/feed?${params.toString()}`);
  },
  import: () => {
    return api.post<{ imported: number; status: string }>('/api/feed/import');
  },
  accounts: () => {
    return api.get<FeedAccount[]>('/api/feed/accounts');
  },
  analytics: (days?: number) => {
    const q = days ? `?days=${days}` : '';
    return api.get<{
      total_likes: number;
      total_comments: number;
      total_shares: number;
      total_views: number;
      total_reposts: number;
      total_replies: number;
      total_upvotes: number;
      total_awards: number;
      posts_with_engagement: number;
    }>(`/api/feed/analytics${q}`);
  },
  delete: (postId: string) => {
    return api.del<{ hidden: boolean }>(`/api/feed/${postId}`);
  },
  save: (postId: string) => {
    return api.post<{ saved: boolean }>(`/api/feed/${postId}/save`, {});
  },
  unsave: (postId: string) => {
    return api.del<{ saved: boolean }>(`/api/feed/${postId}/save`);
  },
  /**
   * Update an imported feed post's cached text/media/metadata.
   * Does NOT touch the original post on the platform — only the cached copy
   * in `external_posts`. Use cases: fix import errors, annotate metadata.
   */
  update: (postId: string, d: { text?: string; media?: unknown; metadata?: unknown }) =>
    api.put<{ id: string; text: string; media: unknown; metadata: unknown }>(`/api/feed/${postId}`, d),
  /**
   * Convert an imported feed post into a Social Forge `posts` row.
   * Creates a new draft (or queued if scheduled_at is provided) post with
   * source_external_post_id set to the feed post's id for provenance.
   * Returns the newly-created post so the caller can open the composer
   * to edit/schedule/publish it.
   */
  repurpose: (postId: string, d: { integration_id: string; content?: string; title?: string; scheduled_at?: string }) =>
    api.post<{
      post: import('./posts').PostSummary & { integration_id: string };
      source_external_post_id: string;
    }>(`/api/feed/${postId}/repurpose`, d),
};
