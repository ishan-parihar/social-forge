import { theme } from '$lib/stores/theme.svelte';

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
  /** Max content length in characters - kept in sync with backend
   *  `Provider::max_content_length()` impls in src/social/ */
  charLimit: number;
}

// The master map. New providers must be added here AND in the backend
// ProviderRegistry - adding to only one will cause drift.
//
// Tier-1 (12) = depth platforms; Tier-2 (14) = publish-maintained.
// kick/vk/whop/lemmy were removed in v25 §1. farcaster is Tier-3 (archive)
// and therefore absent here - it renders with the fallback glyph, which is
// correct, because it is not registered unless ENABLE_ARCHIVE_PROVIDERS is set.
// Source of truth for membership: src/social/tier.rs (backend), which
// `/api/providers` reports as a `tier` field.
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
  skool:                { label: 'Skool',      color: '#facc15', icon: 'SK',  charLimit: 65536 },
  gmail:                { label: 'Gmail',      color: '#ea4335', icon: 'GM',  charLimit: 65536 },
  drive:                { label: 'Drive',      color: '#1fa463', icon: 'DR',  charLimit: 65536 },
  'google-my-business': { label: 'Google Biz', color: '#4285f4', icon: 'GB',  charLimit: 1500 },
};

const FALLBACK_COLOR = '#818cf8'; // indigo-400
const FALLBACK_ICON = '•';

/**
 * v25 F1: providers that must NOT appear in a human-facing list.
 *
 * The backend registry still resolves these (so an existing integration or a
 * historic post keeps rendering - silently breaking those would lose data), but
 * they are out of the supported set: a solo founder should not be offered a
 * connect button for a platform the product no longer ships support for. This
 * is the single place that decision is written down, so every UI list agrees.
 *
 * Deliberately a separate export rather than a deletion from `PROVIDERS`:
 * `providerLabel`/`providerColor` must keep working for historical records, and
 * B1 owns the backend-side tier work.
 */
export const HIDDEN_PROVIDERS: ReadonlySet<string> = new Set([
  'kick',
  'vk',
  'whop',
  'lemmy',
]);

/** Providers offered in UI pickers, connect dialogs and filter lists.
 *  Insertion-ordered, so the list matches the map's authoring order. */
export function visibleProviders(): string[] {
  return Object.keys(PROVIDERS).filter((p) => !HIDDEN_PROVIDERS.has(p));
}

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

/**
 * WCAG-AA-safe TEXT colour for a provider's brand hue.
 *
 * The brand hexes in `PROVIDERS` are tuned for a solid FILL — several of them
 * (x `#9ca3af`, reddit `#fb923c`, slack `#e01e5a`) sit near the luminance of
 * the page surface, so as TEXT on `--bg`/`--bg-card` they measure as low as
 * 1.32:1.
 *
 * THE POINT the first attempt missed: no single static hex clears 4.5:1 on
 * both `#ffffff` and `#131720`. The two surfaces are 1.0 and 0.007 in
 * relative luminance, and every mid-tone hue fails one of them no matter how
 * its lightness is walked. So the ink is chosen against the theme that is
 * actually on screen: darken on the light card, lighten on the dark card.
 * Hue and saturation are preserved so the glyph still reads as its channel;
 * lightness is the only channel that moves.
 *
 * Only for text. Fills keep `providerColor`: the contrast of text over a
 * fill is the `inkFor` caller's business.
 */
export function providerInk(provider: string): string {
  const brand = providerColor(provider);
  return theme.value === 'light'
    ? inkForSurface(brand, '#ffffff')
    : inkForSurface(brand, '#131720');
}

/// Pick a hex clearing 4.5:1 on `surface`, moving lightness in whichever
/// direction the comparison favours, and give up to black only if the hue
/// cannot be carried there at all.
function inkForSurface(hex: string, surface: string): string {
  const need = 4.5;
  const surfaceL = hexLuminance(surface);
  // WCAG's ratio is symmetric: (lighter + 0.05) / (darker + 0.05). Writing it
  // the asymmetric way (surface over ink) makes the score fall as the ink
  // lightens against a dark surface, which silently inverts the search.
  const against = (c: string) => {
    const inkL = hexLuminance(c);
    const hi = Math.max(surfaceL, inkL);
    const lo = Math.min(surfaceL, inkL);
    return (hi + 0.05) / (lo + 0.05);
  };
  if (against(hex) >= need) return hex;
  // Darker surface -> lighten; lighter surface -> darken.
  const step = surfaceL < 0.1 ? 0.03 : -0.03;
  for (let i = 1; i <= 30; i++) {
    const c = shiftLightness(hex, step * i);
    if (against(c) >= need) return c;
  }
  return surfaceL < 0.1 ? '#ffffff' : '#000000';
}

/// WCAG relative luminance of an `#rrggbb` hex.
function hexLuminance(hex: string): number {
  const lin = (i: number) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(1) + 0.7152 * lin(3) + 0.0722 * lin(5);
}

/// Scale an `#rrggbb` hex's HSL lightness by `delta`, keeping hue+saturation.
function shiftLightness(hex: string, delta: number): string {
  const hsl = hexToHsl(hex);
  const l = Math.max(0, Math.min(1, hsl.l + delta));
  return hslToHex(hsl.h, hsl.s, l);
}

function hexToHsl(hex: string): { h: number; s: number; l: number } {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  let h: number;
  if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
  else if (max === g) h = ((b - r) / d + 2) / 6;
  else h = ((r - g) / d + 4) / 6;
  return { h, s, l };
}

function hslToHex(h: number, s: number, l: number): string {
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const f = (t: number) => {
    let tt = t;
    if (tt < 0) tt += 1;
    if (tt > 1) tt -= 1;
    if (tt < 1 / 6) return p + (q - p) * 6 * tt;
    if (tt < 1 / 2) return q;
    if (tt < 2 / 3) return p + (q - p) * (2 / 3 - tt) * 6;
    return p;
  };
  const to = (v: number) =>
    Math.round(v * 255)
      .toString(16)
      .padStart(2, '0');
  return `#${to(f(h + 1 / 3))}${to(f(h))}${to(f(h - 1 / 3))}`;
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
