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
  // Engagement metrics (optional — populated from analytics_cache)
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

export function toCalendarEvent(post: PostSummary): CalendarEvent {
  // For published posts, use the actual published_at timestamp
  // For queued/draft posts, use the scheduled_at timestamp
  const ts = post.state === "published" && post.published_at ? post.published_at : post.scheduled_at;
  return {
    id: post.id,
    date: ts ? ts.split("T")[0] : "",
    time: ts ? ts.split("T")[1]?.slice(0, 5) : undefined,
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
    likes: post.likes ?? null,
    comments: post.comments ?? null,
    shares: post.shares ?? null,
    impressions: post.impressions ?? null,
  };
}
