// Timezone store — stores the user's preferred display timezone in localStorage.
// All dates from the backend are UTC; the frontend renders them in this timezone.
// Falls back to the browser's system timezone if not set.

import { browser } from '$app/environment';

const STORAGE_KEY = 'social-forge-timezone';

function getInitial(): string {
  if (!browser) return 'UTC';
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored) return stored;
  // Fall back to browser timezone
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  } catch {
    return 'UTC';
  }
}

class TimezoneStore {
  value = $state<string>(getInitial());

  set(tz: string) {
    this.value = tz;
    if (browser) {
      localStorage.setItem(STORAGE_KEY, tz);
    }
  }

  // Format a UTC ISO string in the user's timezone
  format(iso: string, options?: Intl.DateTimeFormatOptions): string {
    try {
      return new Intl.DateTimeFormat('en-US', {
        timeZone: this.value,
        ...options,
      }).format(new Date(iso));
    } catch {
      return new Date(iso).toLocaleString();
    }
  }

  // Format date only (e.g. "Jul 5, 2026")
  formatDate(iso: string): string {
    return this.format(iso, { year: 'numeric', month: 'short', day: 'numeric' });
  }

  // Format time only (e.g. "9:00 AM")
  formatTime(iso: string): string {
    return this.format(iso, { hour: 'numeric', minute: '2-digit' });
  }

  // Format date+time (e.g. "Jul 5, 2026, 9:00 AM")
  formatDateTime(iso: string): string {
    return this.format(iso, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
    });
  }

  // Get the list of common timezones for the picker
  get commonTimezones(): string[] {
    return [
      'UTC',
      'America/New_York',
      'America/Chicago',
      'America/Denver',
      'America/Los_Angeles',
      'America/Anchorage',
      'America/Toronto',
      'America/Vancouver',
      'America/Sao_Paulo',
      'America/Mexico_City',
      'Europe/London',
      'Europe/Paris',
      'Europe/Berlin',
      'Europe/Madrid',
      'Europe/Rome',
      'Europe/Amsterdam',
      'Europe/Stockholm',
      'Europe/Moscow',
      'Europe/Istanbul',
      'Africa/Cairo',
      'Africa/Johannesburg',
      'Asia/Dubai',
      'Asia/Kolkata',
      'Asia/Bangkok',
      'Asia/Singapore',
      'Asia/Shanghai',
      'Asia/Hong_Kong',
      'Asia/Tokyo',
      'Asia/Seoul',
      'Australia/Sydney',
      'Australia/Melbourne',
      'Pacific/Auckland',
    ];
  }
}

export const timezone = new TimezoneStore();
