import { api } from './client';
import type { Tag } from './tags';

export interface CalendarPost {
  id: string;
  integration_id: string;
  integration_name: string;
  state: string;
  content: string;
  title?: string | null;
  media?: { url: string; mime_type: string; alt?: string }[] | null;
  scheduled_at?: string | null;
  published_at?: string | null;
  platform_post_url?: string | null;
  error_message?: string | null;
  created_at: string;
  tags?: Tag[];
  group_id?: string | null;
  first_comment?: string | null;
  sequence?: number;
  repeat_interval_days?: number | null;
  repeat_end_date?: string | null;
  campaign_id?: string | null;
  /**
   * The calendar endpoint returns engagement as FOUR FLAT fields, not
   * the nested `engagement` object the posts list uses. This type
   * mirrors the endpoint rather than borrowing `PostSummary`, which
   * describes a different response.
   */
  likes?: number | null;
  comments?: number | null;
  shares?: number | null;
  impressions?: number | null;
}

export interface CalendarDay {
  date: string;
  posts: CalendarPost[];
  post_count: number;
}

// v23-4: optional filter params for the calendar endpoint.
export interface CalendarFilters {
  integration_ids?: string[]; // integration UUIDs
  campaign_id?: string;       // campaign UUID
}

export const calendarApi = {
  /**
   * @param tz The founder's zone as an IANA NAME ("America/New_York").
   *
   * A post's DAY is a local day, and the server resolves the name
   * per-instant (DST-correct), so the bucketing matches the days the
   * founder sees even across a transition. The legacy `tz_offset`
   * (a constant) is still accepted by the server for callers that
   * only know an offset.
   */
  get: (start: string, end: string, filters?: CalendarFilters, tz?: string) => {
    let url = `/api/calendar?start=${start}&end=${end}`;
    if (tz) {
      url += `&tz=${encodeURIComponent(tz)}`;
    }
    // v23-4: append optional filters.
    if (filters?.integration_ids?.length) {
      url += `&integration_ids=${filters.integration_ids.join(',')}`;
    }
    if (filters?.campaign_id) {
      url += `&campaign_id=${filters.campaign_id}`;
    }
    return api.get<{ days: CalendarDay[]; total: number }>(url);
  },
};
