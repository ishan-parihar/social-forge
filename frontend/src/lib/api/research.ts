import { api } from "./client";

/** One of the founder's own posts ranked by engagement, with the channel it
 *  went out on and when. Engagement is likes + comments + shares, from the
 *  cached per-post analytics. */
export interface TopOwnPost {
  post_id: string;
  content: string;
  /** Provider the post published through (`posts` has no provider column). */
  provider: string;
  /** ISO8601, or null when the publish timestamp was never recorded. */
  published_at: string | null;
  engagement: number;
}

/** An imported external/competitor post ranked by engagement. */
export interface TopExternalPost {
  post_id: string;
  provider: string;
  text: string;
  author_name: string | null;
  author_handle: string | null;
  url: string | null;
  /** ISO8601 — the platform post's own timestamp. */
  published_at: string;
  engagement: number;
}

/** One weekday×hour slot with the engagement the founder's own posts earned
 *  there. `engagement_per_post` averages the slot, so a slot with one lucky
 *  post is not ranked above a slot with ten solid ones. */
export interface PostingSlot {
  /** e.g. "Wednesday". */
  weekday: string;
  /** 0-23, in the app's zone when the request named one, UTC otherwise. */
  hour: number;
  /** e.g. "Wednesday 14:00–15:00 America/New_York" - the full slot
   *  description, unique per slot; it is the list key and the tooltip. */
  label: string;
  /** The clock range alone ("14:00–15:00") for narrow table cells. */
  hour_range: string;
  /** How many of the founder's own posts went out in this slot — the sample
   *  the ranking rests on. */
  posts: number;
  engagement: number;
  engagement_per_post: number;
}

export interface TimingInsight {
  /** True when fewer than 3 posts back the estimate — treat as a hint. */
  low_confidence: boolean;
  posts_analyzed: number;
  best_slots: PostingSlot[];
}

/** A word that appears in competitor posts but never in the founder's own.
 *  `external_posts` is how many competitor posts used it. */
export interface ContentGap {
  word: string;
  external_posts: number;
  /** One competitor post text, so the gap shows what the word was used to
   *  talk about. Null only when no text could be resolved. */
  example: string | null;
}

/** A title from one of the founder's connected RSS feeds. No engagement is
 *  recorded for feed items, so this is reported as a list, never ranked. */
export interface FeedItem {
  id: string;
  title: string;
  url: string;
  published_at: string | null;
}

/** How many rows each section was computed from. A section that is empty
 *  says which source had nothing. */
export interface ResearchSources {
  own_posts: number;
  external_posts: number;
  feed_items: number;
}

export interface ResearchResponse {
  top_own_posts: TopOwnPost[];
  top_external_posts: TopExternalPost[];
  /** Null when no own-post engagement data exists in the window. */
  timing: TimingInsight | null;
  content_gaps: ContentGap[];
  connected_feed_items: FeedItem[];
  sources: ResearchSources;
  window_days: number;
}

export const researchApi = {
  /**
   * Aggregated read-only research snapshot. No writes; every section is
   * derived from stored data only and comes back empty when it has none.
   */
  get: (days?: number, tz?: string, signal?: AbortSignal) => {
    const parts: string[] = [];
    if (days) parts.push(`days=${days}`);
    if (tz) parts.push(`tz=${encodeURIComponent(tz)}`);
    const q = parts.length ? `?${parts.join('&')}` : "";
    return api.get<ResearchResponse>(`/api/research${q}`, signal);
  },
};
