// v24-4: Brand profile API client.
import { api } from './client';

export interface BrandProfile {
  user_id: string;
  brand_name: string | null;
  description: string | null;
  tone_of_voice: string | null;
  audience: string | null;
  content_pillars: Array<{ title: string; description?: string }> | null;
  keywords: string[] | null;
  hashtag_sets: Array<{ name: string; tags: string[] }> | null;
  avoid_topics: string[] | null;
  posting_frequency: string | null;
  posts_per_day_goal: number | null;
  created_at: string;
  updated_at: string;
}

export interface UpdateBrandProfileInput {
  brand_name?: string;
  description?: string;
  tone_of_voice?: string;
  audience?: string;
  content_pillars?: Array<{ title: string; description?: string }>;
  keywords?: string[];
  hashtag_sets?: Array<{ name: string; tags: string[] }>;
  avoid_topics?: string[];
  posting_frequency?: string;
  posts_per_day_goal?: number;
}

export const profileApi = {
  get: () => api.get<BrandProfile | null>('/api/profile'),
  update: (data: UpdateBrandProfileInput) => api.put<BrandProfile>('/api/profile', data),
};
