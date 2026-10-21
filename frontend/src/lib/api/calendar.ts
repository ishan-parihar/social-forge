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
   * @param tzOffsetSeconds The founder's UTC offset, in SECONDS.
   *
   * A post's DAY is a local day, and the server has no timezone
   * database and no per-user zone. The client knows the zone (an
   * IANA name in a localStorage store) and resolves the offset for
   * the window, so the server can bucket the posts into the days the
   * founder actually sees.
   *
   * Without it the server buckets in UTC, so a post at 02:00 UTC sat
   * on "the 25th" while the founder in New York saw it on the 24th.
   */
  get: (start: string, end: string, filters?: CalendarFilters, tzOffsetSeconds?: number) => {
    let url = `/api/calendar?start=${start}&end=${end}`;
    if (tzOffsetSeconds !== undefined) {
      url += `&tz_offset=${tzOffsetSeconds}`;
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
