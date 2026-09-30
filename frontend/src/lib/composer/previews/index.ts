// Per-platform preview registry (Phase 4, completed in v25 F2).
//
// Maps provider identifiers to their custom preview components.
// The PlatformPreviewPane reads this registry to decide which preview
// to render for each selected channel.
//
// v25 F2 completed the Tier-1 set: all 12 depth platforms now have a
// frame-accurate preview. Previously only 8 of them did — YouTube, TikTok
// and Pinterest fell through to GeneralPreview, an X-shaped card, which is
// exactly wrong for a 9:16 vertical-video platform. See docs/planning/
// PLAN_PARITY_DEPTH_SINGLEUSER_v25.md §4/F2.
//
// Adding a new platform preview:
//   1. Create frontend/src/lib/composer/previews/{Platform}Preview.svelte
//   2. Import it here and add an entry to the PREVIEW_REGISTRY below.
//
// Platforms not in the registry fall back to GeneralPreview (the
// Twitter/X-like default card). That fallback is the right level of fidelity
// for Tier-2 — we do not claim depth there, so we do not simulate its chrome.
//
// CHROME CONTRACT: every preview here renders the DESTINATION platform's own
// light card, not this app's theme. That is why previews/** is exempt from the
// check-hex-tokens gate: retinting these to app tokens would make the preview
// lie about what lands on the platform. Keep hexes inside this folder.

import type { Component } from 'svelte';
import type { MediaItem } from '$lib/api/media';
import GeneralPreview from './GeneralPreview.svelte';
import InstagramPreview from './InstagramPreview.svelte';
import LinkedInPreview from './LinkedInPreview.svelte';
import FacebookPreview from './FacebookPreview.svelte';
// v24-8: new platform previews.
import XPreview from './XPreview.svelte';
import RedditPreview from './RedditPreview.svelte';
import ThreadsPreview from './ThreadsPreview.svelte';
import BlueskyPreview from './BlueskyPreview.svelte';
// v25 F2: the three that were missing — the vertical-video tier.
import YouTubePreview from './YouTubePreview.svelte';
import TikTokPreview from './TikTokPreview.svelte';
import PinterestPreview from './PinterestPreview.svelte';

/**
 * The prop shape every preview speaks (F2).
 *
 * F2 found that the four v24-8 previews took `integrationName` while
 * PlatformPreviewPane passed `authorName`/`authorHandle` — so those four
 * silently rendered a hardcoded default account and the user could not tell
 * which of their channels a card belonged to. Declaring the contract once
 * here is what makes that class of bug impossible to reintroduce.
 *
 * Every field is optional: a platform that ignores `title` (X) or
 * `firstComment` (Bluesky) simply does not destructure it.
 */
export interface PreviewProps {
  content?: string;
  provider?: string;
  authorName?: string;
  authorHandle?: string;
  authorAvatar?: string;
  /** Post title. Required by YouTube / Pinterest / Reddit. */
  title?: string;
  media?: MediaItem[];
  /** First-comment body. Surfaces on X, Facebook, Threads, Instagram. */
  firstComment?: string;
  /** ISO-8601 UTC. Previews that show a timestamp render it in this zone. */
  scheduledAt?: string | null;
  /** IANA zone name, for timestamp previews. */
  timezone?: string;
  /** TikTok sound track title, from the composer's MusicPicker. */
  audioTitle?: string;
  /** Pinterest board name, from the TargetPicker. */
  targetLabel?: string;
}

export const PREVIEW_REGISTRY: Record<string, Component<any, any, any>> = {
  instagram: InstagramPreview,
  'instagram-standalone': InstagramPreview,
  linkedin: LinkedInPreview,
  'linkedin-page': LinkedInPreview,
  facebook: FacebookPreview,
  // v24-8: new platform previews.
  x: XPreview,
  twitter: XPreview,
  reddit: RedditPreview,
  threads: ThreadsPreview,
  bluesky: BlueskyPreview,
  // v25 F2: completed the Tier-1 set. Before this, YouTube / TikTok /
  // Pinterest fell through to GeneralPreview — an X-shaped card, which is
  // badly wrong for a 9:16 vertical-video platform.
  youtube: YouTubePreview,
  tiktok: TikTokPreview,
  pinterest: PinterestPreview,
};

/** True when this provider has its own chrome rather than the fallback. */
export function hasDedicatedPreview(provider: string): boolean {
  return provider in PREVIEW_REGISTRY;
}

/** Get the preview component for a provider. Falls back to GeneralPreview. */
export function getPreviewComponent(provider: string): Component<any, any, any> {
  return PREVIEW_REGISTRY[provider] || GeneralPreview;
}

export { GeneralPreview };
