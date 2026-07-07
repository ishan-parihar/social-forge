// Central provider metadata (R-8): single source of truth for provider
// labels, colors, icons, and char limits.
//
// Before this file existed, the same provider→label/color/icon mapping
// was duplicated across 6+ files (dashboard, search, feed, channels,
// PerPlatformCharCount, comments). Adding a new provider meant editing
// all 6 places and inevitably forgetting one.
//
// Usage:
//   import { providerMeta, providerLabel, providerIcon, providerColor } from '$lib/providers';
//   providerMeta('x')           // → { label: 'X', color: '#9ca3af', icon: 'X', charLimit: 280, ... }
//   providerLabel('instagram')  // → 'Instagram'
//   providerColor('reddit')     // → '#fb923c'
//   providerIcon('x')           // → 'X'
//
// All accessors are null-safe and fall back to a sensible default
// (capitalized provider name, indigo color, '•' icon) so unknown
// providers don't break the UI.

export interface ProviderMeta {
  /** Human-readable label, e.g. 'X', 'LinkedIn', 'Instagram' */
  label: string;
  /** Hex color used for icon text and accent borders */
  color: string;
  /** Short 1-3 character icon glyph (used when no SVG is available) */
  icon: string;
  /** Max content length in characters — kept in sync with backend
   *  `Provider::max_content_length()` impls in src/social/ */
  charLimit: number;
}

// The master map. New providers must be added here AND in the backend
// ProviderRegistry — adding to only one will cause drift.
export const PROVIDERS: Record<string, ProviderMeta> = {
  x:                    { label: 'X',          color: '#9ca3af', icon: 'X',   charLimit: 280 },
  reddit:               { label: 'Reddit',     color: '#fb923c', icon: 'R',   charLimit: 10000 },
  linkedin:             { label: 'LinkedIn',   color: '#3b82f6', icon: 'in',  charLimit: 3000 },
  'linkedin-page':      { label: 'LinkedIn',   color: '#3b82f6', icon: 'in',  charLimit: 3000 },
  facebook:             { label: 'Facebook',   color: '#2563eb', icon: 'f',   charLimit: 63206 },
  instagram:            { label: 'Instagram',  color: '#f472b6', icon: 'IG',  charLimit: 2200 },
  'instagram-standalone': { label: 'Instagram',color: '#f472b6', icon: 'IG',  charLimit: 2200 },
  threads:              { label: 'Threads',    color: '#a78bfa', icon: 'TH',  charLimit: 500 },
  bluesky:              { label: 'Bluesky',    color: '#38bdf8', icon: 'BS',  charLimit: 300 },
  mastodon:             { label: 'Mastodon',   color: '#6364ff', icon: 'MA',  charLimit: 500 },
  pinterest:            { label: 'Pinterest',  color: '#f87171', icon: 'PIN', charLimit: 500 },
  tiktok:               { label: 'TikTok',     color: '#67e8f9', icon: 'TT',  charLimit: 2200 },
  youtube:              { label: 'YouTube',    color: '#ef4444', icon: 'YT',  charLimit: 5000 },
  discord:              { label: 'Discord',    color: '#5865f2', icon: 'DC',  charLimit: 2000 },
  slack:                { label: 'Slack',      color: '#e01e5a', icon: 'SL',  charLimit: 40000 },
  'telegram-bot':       { label: 'Telegram',   color: '#229ed9', icon: 'TG',  charLimit: 4096 },
  'telegram-user':      { label: 'Telegram',   color: '#229ed9', icon: 'TG',  charLimit: 4096 },
  whatsapp:             { label: 'WhatsApp',   color: '#25d366', icon: 'WA',  charLimit: 65536 },
  github:               { label: 'GitHub',     color: '#d1d5db', icon: 'GH',  charLimit: 65536 },
  devto:                { label: 'Dev.to',     color: '#9ca3af', icon: 'DT',  charLimit: 65536 },
  medium:               { label: 'Medium',     color: '#22c55e', icon: 'MD',  charLimit: 65536 },
  wordpress:            { label: 'WordPress',  color: '#60a5fa', icon: 'WP',  charLimit: 65536 },
  hashnode:             { label: 'Hashnode',   color: '#60a5fa', icon: 'HN',  charLimit: 65536 },
  lemmy:                { label: 'Lemmy',      color: '#f97316', icon: 'LE',  charLimit: 10000 },
  vk:                   { label: 'VK',         color: '#60a5fa', icon: 'VK',  charLimit: 65536 },
  kick:                 { label: 'Kick',       color: '#53fc18', icon: 'KI',  charLimit: 65536 },
  skool:                { label: 'Skool',      color: '#facc15', icon: 'SK',  charLimit: 65536 },
  gmail:                { label: 'Gmail',      color: '#ea4335', icon: 'GM',  charLimit: 65536 },
  drive:                { label: 'Drive',      color: '#1fa463', icon: 'DR',  charLimit: 65536 },
  'google-my-business': { label: 'Google Biz', color: '#4285f4', icon: 'GB',  charLimit: 1500 },
};

const FALLBACK_COLOR = '#818cf8'; // indigo-400
const FALLBACK_ICON = '•';

/** Full metadata for a provider. Falls back to a capitalized provider
 *  name and the default color/icon for unknown providers. */
export function providerMeta(provider: string): ProviderMeta {
  const meta = PROVIDERS[provider];
  if (meta) return meta;
  return {
    label: provider.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase()),
    color: FALLBACK_COLOR,
    icon: FALLBACK_ICON,
    charLimit: 65536,
  };
}

/** Just the human-readable label. */
export function providerLabel(provider: string): string {
  return providerMeta(provider).label;
}

/** Just the hex color. */
export function providerColor(provider: string): string {
  return providerMeta(provider).color;
}

/** Just the short icon glyph. */
export function providerIcon(provider: string): string {
  return providerMeta(provider).icon;
}

/** Just the per-platform character limit. */
export function providerCharLimit(provider: string): number {
  return providerMeta(provider).charLimit;
}

/**
 * v23-6: Construct a "manage on platform" URL for a published post.
 *
 * Given a provider identifier (e.g. "x", "linkedin", "reddit") and the
 * platform's post ID, returns the URL where the user can view/manage
 * the post on the platform's own UI. Returns null if the provider is
 * unknown or the platform_post_id is missing.
 *
 * For providers where the URL pattern isn't known, returns null (the
 * caller can fall back to the post.url field if available).
 */
export function platformPostUrl(provider: string, platformPostId: string | null | undefined): string | null {
  if (!platformPostId) return null;
  const p = provider.toLowerCase();
  switch (p) {
    case 'x':
    case 'twitter':
      // X post URLs use the numeric ID: https://x.com/i/status/{id}
      return `https://x.com/i/status/${platformPostId}`;
    case 'linkedin':
    case 'linkedin-page':
      // LinkedIn UGC posts: https://www.linkedin.com/feed/update/{ugcPostId}/
      return `https://www.linkedin.com/feed/update/${platformPostId}/`;
    case 'reddit':
      // Reddit post URLs: https://www.reddit.com/comments/{id}
      return `https://www.reddit.com/comments/${platformPostId}`;
    case 'facebook':
      // Facebook post URLs: https://www.facebook.com/{pageId}/posts/{postId}
      // We only have the post ID, so use the permalink pattern.
      return `https://www.facebook.com/${platformPostId}`;
    case 'instagram':
      // Instagram doesn't expose a public post URL by ID alone; the
      // permalink requires the shortcode. Return null and fall back to
      // post.url if available.
      return null;
    case 'threads':
      return `https://www.threads.net/post/${platformPostId}`;
    case 'bluesky':
      return `https://bsky.app/profile/post/${platformPostId}`;
    case 'mastodon':
      // Mastodon URLs are instance-specific; can't construct without
      // the instance URL. Fall back to post.url.
      return null;
    case 'youtube':
      return `https://www.youtube.com/watch?v=${platformPostId}`;
    case 'pinterest':
      return `https://www.pinterest.com/pin/${platformPostId}/`;
    case 'tiktok':
      return `https://www.tiktok.com/@user/video/${platformPostId}`;
    case 'discord':
      // Discord message URLs require channel + guild IDs; can't construct.
      return null;
    case 'telegram-bot':
    case 'telegram-user':
      // Telegram message URLs require the chat ID; can't construct.
      return null;
    default:
      return null;
  }
}
