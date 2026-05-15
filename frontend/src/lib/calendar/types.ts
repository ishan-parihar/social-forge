import type { PostSummary } from "$lib/api/posts";

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
}

export interface WeekDay {
  date: Date;
  dateStr: string;      // YYYY-MM-DD
  isToday: boolean;
  isCurrentMonth: boolean;
  events: CalendarEvent[];
}

export function toCalendarEvent(post: PostSummary): CalendarEvent {
  return {
    id: post.id,
    date: (post.scheduled_at || post.created_at).split("T")[0],
    time: post.scheduled_at ? post.scheduled_at.split("T")[1]?.slice(0, 5) : undefined,
    title: post.title || post.content.slice(0, 60),
    content: post.content,
    state: post.state,
    platform: post.integration_name,
    integrationName: post.integration_name,
    postUrl: post.platform_post_url,
    error: post.error_message,
  };
}
