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

export const analyticsApi = {
  getPostAnalytics: (postId: string, days?: number) => {
    const q = days ? `?days=${days}` : "";
    return api.get<PostAnalytics>(`/api/analytics/post/${postId}${q}`);
  },
};
