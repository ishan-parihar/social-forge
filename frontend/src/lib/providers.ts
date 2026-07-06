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
