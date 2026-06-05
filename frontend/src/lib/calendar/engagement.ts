// ─── Engagement helpers — platform-aware metric display ──────
// Unifies likes/upvotes/reactions, shares/reposts/quotes, comments/replies
// into single slots with platform-appropriate icons and labels.

export function engagementIcon(metric: string, platform?: string | null): string {
  const p = platform?.toLowerCase() ?? '';
  switch (metric) {
    case 'likes':
      if (p === 'reddit' || p === 'lemmy') return '👍';
      if (p === 'x' || p === 'bluesky' || p === 'threads') return '❤️';
      if (p === 'youtube') return '👍';
      if (p === 'facebook') return '❤️';
      return '❤️';
    case 'comments':
      if (p === 'x' || p === 'bluesky' || p === 'threads') return '💬';
      if (p === 'reddit' || p === 'lemmy') return '💬';
      if (p === 'youtube') return '💬';
      return '💬';
    case 'shares':
      if (p === 'x' || p === 'bluesky' || p === 'threads') return '🔄';
      return '🔗';
    case 'impressions':
      return '👁️';
    default:
      return '📊';
  }
}

export function engagementLabel(metric: string, platform?: string | null): string {
  const p = platform?.toLowerCase() ?? '';
  switch (metric) {
    case 'likes':
      if (p === 'reddit' || p === 'lemmy') return 'Upvotes';
      if (p === 'x' || p === 'bluesky' || p === 'threads') return 'Likes';
      if (p === 'youtube') return 'Likes';
      if (p === 'facebook') return 'Reactions';
      return 'Likes';
    case 'comments':
      if (p === 'x' || p === 'bluesky' || p === 'threads') return 'Replies';
      if (p === 'reddit' || p === 'lemmy') return 'Comments';
      return 'Comments';
    case 'shares':
      if (p === 'x' || p === 'bluesky' || p === 'threads') return 'Reposts';
      if (p === 'reddit') return 'Shares';
      return 'Shares';
    case 'impressions':
      if (p === 'x' || p === 'bluesky' || p === 'threads') return 'Views';
      if (p === 'youtube') return 'Views';
      return 'Impressions';
    default:
      return metric;
  }
}

export function formatMetricCount(n: number): string {
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(1).replace(/\.0$/, '') + 'M';
  if (n >= 1_000) return (n / 1_000).toFixed(1).replace(/\.0$/, '') + 'k';
  return String(n);
}
