import { api } from "./client";

export interface AnalyticsDataPoint {
  date: string;
  impressions: number;
  likes: number;
  shares: number;
  comments: number;
}

export interface PostAnalytics {
  data: AnalyticsDataPoint[];
}

export interface AnalyticsSummary {
  total_posts: number;
  published: number;
  failed: number;
  draft: number;
  queued: number;
  best_provider: { provider: string; count: number } | null;
  posts_by_provider: Array<{ provider: string; count: number }>;
  posts_by_day: Array<{ date: string; count: number }>;
}

const daysQuery = (d?: number) => d ? `?days=${d}` : '';

export const analyticsApi = {
  getPostAnalytics: (postId: string, days?: number, signal?: AbortSignal) => {
    return api.get<PostAnalytics>(`/api/analytics/post/${postId}${daysQuery(days)}`, signal);
  },
  getSummary: (days?: number, signal?: AbortSignal) => {
    return api.get<AnalyticsSummary>(`/api/analytics/summary${daysQuery(days)}`, signal);
  },
};
