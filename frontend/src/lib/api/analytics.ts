import { api } from "./client";

export interface AnalyticsDataPoint {
  date: string;
  impressions: number;
  likes: number;
  shares: number;
  comments: number;
}

// The provider's own analytics shape: one series per metric, each with a
// list of points whose `total` is a STRING (the provider formats it) and
// an ISO date. `/api/analytics` and `/api/analytics/post/{id}` both
// return this, and it is NOT the flat `AnalyticsDataPoint[]` shape - a
// component that assumes the flat shape reads `undefined` for every
// metric and renders NaN totals.
export interface AnalyticsSeries {
  label: string;
  data: Array<{ total: string; date: string }>;
  percentage_change: number;
}

export interface PostAnalytics {
  data: AnalyticsSeries[];
  cached?: boolean;
  cached_at?: string;
}

export interface AnalyticsSummary {
  total_posts: number;
  published: number;
  /**
   * The posts PUBLISHED inside the window, by publish time.
   *
   * `published` is counted by creation date, so a post drafted long ago
   * and published today is in `published` but not in the window. This
   * is the denominator for "interactions per post published in the
   * last N days" - the frontend cannot compute it, because it only
   * ever holds the newest 100 posts.
   */
  published_in_window: number;
  failed: number;
  draft: number;
  queued: number;
  best_provider: { provider: string; count: number } | null;
  posts_by_provider: Array<{ provider: string; count: number }>;
  posts_by_day: Array<{ date: string; count: number }>;
}

export interface ProviderAnalytics {
  data: AnalyticsSeries[];
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
  /**
   * published / scheduled * 100 - the SUCCESS RATE.
   *
   * A different number from `adherence_rate`, which is measured
   * against the larger of the plan and the goal. A card that called the
   * goal-based number a "success rate" while showing "11 of 13
   * scheduled posts went out" underneath could not be reconciled by
   * the person reading it.
   */
  success_rate: number;
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
  /**
   * The human-readable sentence for this row, computed by the backend
   * (`api::activity::event_summary`). The backend owns this mapping so the
   * activity log and this widget cannot disagree.
   */
  summary: string;
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
