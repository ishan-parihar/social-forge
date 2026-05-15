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
};
