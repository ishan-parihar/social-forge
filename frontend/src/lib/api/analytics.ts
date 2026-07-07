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

export interface ProviderAnalytics {
  data: Array<{
    label: string;
    data: Array<{ total: string; date: string }>;
    percentage_change: number;
  }>;
}

// v23: new dashboard analytics types.
export interface EngagementResponse {
  total_likes: number;
  total_comments: number;
  total_shares: number;
  total_impressions: number;
  likes_delta: number;
  comments_delta: number;
  shares_delta: number;
  impressions_delta: number;
  by_day: Array<{ date: string; likes: number; comments: number; shares: number; impressions: number }>;
}

export interface AdherenceResponse {
  scheduled: number;
  published: number;
  failed: number;
  adherence_rate: number;
}

export interface CadenceResponse {
  goal_per_day: number | null;
  actual_per_day: number;
  streak_days: number;
  total_posts: number;
  by_day: Array<{ date: string; count: number }>;
}

export interface EventLogEntry {
  id: string;
  event_type: string;
  payload: Record<string, unknown>;
  created_at: string;
}

const daysQuery = (d?: number) => d ? `?days=${d}` : '';

export const analyticsApi = {
  getSummary: (days?: number, signal?: AbortSignal) => {
    return api.get<AnalyticsSummary>(`/api/analytics/summary${daysQuery(days)}`, signal);
  },
  getProvider: (provider: string, days?: number, signal?: AbortSignal) => {
    return api.get<ProviderAnalytics>(`/api/analytics?provider=${provider}${days ? `&days=${days}` : ''}`, signal);
  },
  getPostAnalytics: (postId: string, days?: number, signal?: AbortSignal) => {
    const q = days ? `?days=${days}` : '';
    return api.get<PostAnalytics>(`/api/analytics/post/${postId}${q}`, signal);
  },
  // v23: new dashboard endpoints.
  getEngagement: (days?: number, signal?: AbortSignal) => {
    return api.get<EngagementResponse>(`/api/analytics/engagement${daysQuery(days)}`, signal);
  },
  getAdherence: (days?: number, signal?: AbortSignal) => {
    return api.get<AdherenceResponse>(`/api/analytics/adherence${daysQuery(days)}`, signal);
  },
  getCadence: (days?: number, signal?: AbortSignal) => {
    return api.get<CadenceResponse>(`/api/analytics/cadence${daysQuery(days)}`, signal);
  },
  getRecentEvents: (limit?: number, signal?: AbortSignal) => {
    const q = limit ? `?limit=${limit}` : '';
    return api.get<EventLogEntry[]>(`/api/events/recent${q}`, signal);
  },
};
