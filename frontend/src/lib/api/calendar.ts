import { api } from './client';
import type { PostSummary } from './posts';

export interface CalendarDay {
  date: string;
  posts: PostSummary[];
  post_count: number;
}

// v23-4: optional filter params for the calendar endpoint.
export interface CalendarFilters {
  integration_ids?: string[]; // integration UUIDs
  campaign_id?: string;       // campaign UUID
}

export const calendarApi = {
  get: (start: string, end: string, filters?: CalendarFilters) => {
    let url = `/api/calendar?start=${start}&end=${end}`;
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
