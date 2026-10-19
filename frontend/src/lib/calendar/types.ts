import type { PostSummary } from "$lib/api/posts";
import type { Tag } from "$lib/api/tags";

export type CalendarView = "month" | "week" | "day" | "list";

export interface CalendarEvent {
  id: string;
  date: string;         // YYYY-MM-DD
  time?: string;        // HH:mm (optional)
  title: string;
  content: string;      // preview text
  state: string;
  platform: string;
  integrationName: string;
  postUrl?: string;
  error?: string;
  tags?: Array<{ id: string; name: string; color: string }>;
  repeatIntervalDays?: number | null;
  repeatEndDate?: string | null;
  groupId?: string | null;
  // Phase 8: campaign awareness for calendar.
  campaignId?: string | null;
  campaignColor?: string | null;
  // Engagement metrics (optional - populated from analytics_cache)
  likes?: number | null;
  comments?: number | null;
  shares?: number | null;
  impressions?: number | null;
}

export interface WeekDay {
  date: Date;
  dateStr: string;      // YYYY-MM-DD
  isToday: boolean;
  isCurrentMonth: boolean;
  events: CalendarEvent[];
}

/**
 * The wall-clock date and time of a UTC instant, in `zone`.
 *
 * The backend sends RFC3339 UTC. The calendar grid is built from LOCAL
 * dates (`formatDateKey` uses `getFullYear`/`getMonth`/`getDate`), so
 * slicing the UTC string put a 23:30-local post on the NEXT day for
 * every user west of Greenwich: `2026-10-31T04:30:00Z` sliced to
 * "2026-10-31" while the grid cell the user meant was the 30th.
 *
 * This converts the instant into the user's zone first, then reads the
 * wall clock off it - the same conversion the composer's SchedulePicker
 * already does, so what the user set is what the calendar shows.
 */
export function wallClock(iso: string | null | undefined, zone: string): { date: string; time: string } {
  if (!iso) return { date: "", time: "" };
  try {
    const parts = new Intl.DateTimeFormat("en-CA", {
      timeZone: zone,
      hourCycle: "h23",
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).formatToParts(new Date(iso));
    const get = (t: string) => parts.find((p) => p.type === t)?.value ?? "";
    return {
      date: `${get("year")}-${get("month")}-${get("day")}`,
      time: `${get("hour")}:${get("minute")}`,
    };
  } catch {
    // A malformed zone or timestamp falls back to the raw UTC date.
    return { date: iso.slice(0, 10), time: iso.slice(11, 16) };
  }
}

export function toCalendarEvent(
  post: PostSummary,
  zone = "UTC",
  campaignColor?: string | null,
): CalendarEvent {
  // For published posts, use the actual published_at timestamp
  // For queued/draft posts, use the scheduled_at timestamp
  const ts = post.state === "published" && post.published_at ? post.published_at : post.scheduled_at;
  const wall = wallClock(ts, zone);
  return {
    id: post.id,
    date: wall.date,
    time: wall.time || undefined,
    title: post.title || post.content.slice(0, 60),
    content: post.content,
    state: post.state,
    platform: post.integration_name,
    integrationName: post.integration_name,
    postUrl: post.platform_post_url,
    error: post.error_message,
    tags: post.tags?.map(t => ({ id: t.id, name: t.name, color: t.color })),
    repeatIntervalDays: post.repeat_interval_days ?? null,
    repeatEndDate: post.repeat_end_date ?? null,
    groupId: post.group_id ?? null,
    // v23-4 shipped the campaign chip colour, but the mapping never copied
    // `campaign_id` across, so `campaignColor` was always undefined and the
    // chip fell back to the first tag's colour. The backend does return
    // `campaign_id`; the colour comes from the campaign list the page
    // already holds.
    campaignId: post.campaign_id ?? null,
    campaignColor: post.campaign_id ? (campaignColor ?? null) : null,
    likes: post.likes ?? null,
    comments: post.comments ?? null,
    shares: post.shares ?? null,
    impressions: post.impressions ?? null,
  };
}
