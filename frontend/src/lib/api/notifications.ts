import { api } from './client';

export interface Notification {
    id: string;
    title: string;
    body: string;
    notification_type: string;
    reference_type: string | null;
    reference_id: string | null;
    is_read: boolean;
    created_at: string;
}

export interface NotificationPrefs {
    post_published: string;
    post_failed: string;
    team_invite: string;
    analytics_weekly: string;
    quiet_hours_start: string | null;
    quiet_hours_end: string | null;
    timezone: number;
}

export const notificationsApi = {
    list: (limit?: number, offset?: number) =>
        api.get<{ data: Notification[] }>(`/api/notifications?limit=${limit ?? 50}&offset=${offset ?? 0}`),
    unreadCount: () =>
        api.get<{ count: number }>('/api/notifications/unread-count'),
    markRead: (id: string) =>
        api.put<{ data: Notification }>(`/api/notifications/${id}/read`),
    markAllRead: () =>
        api.put<{ updated: number }>('/api/notifications/read-all'),
    getPrefs: () =>
        api.get<NotificationPrefs>('/api/notifications/prefs'),
    updatePrefs: (prefs: Partial<NotificationPrefs>) =>
        api.put<NotificationPrefs>('/api/notifications/prefs', prefs),
};
