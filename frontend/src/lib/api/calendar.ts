import { api } from './client';
import type { PostSummary } from './posts';

export interface CalendarDay {
  date: string;
  posts: PostSummary[];
  post_count: number;
}

export const calendarApi = {
  get: (start: string, end: string) =>
    api.get<{ days: CalendarDay[]; total: number }>(`/api/calendar?start=${start}&end=${end}`),
  reschedule: (id: string, scheduledAt: string) =>
    api.post<{ success: boolean }>(`/api/calendar/reschedule`, { post_id: id, scheduled_at: scheduledAt }),
};
