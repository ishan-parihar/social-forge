// Per-platform preview registry (Phase 4).
//
// Maps provider identifiers to their custom preview components.
// The PlatformPreviewPane reads this registry to decide which preview
// to render for the currently-selected channel.
//
// Adding a new platform preview:
//   1. Create frontend/src/lib/composer/previews/{Platform}Preview.svelte
//   2. Import it here and add an entry to the PREVIEW_REGISTRY below.
//
// Platforms not in the registry fall back to GeneralPreview (the
// Twitter/X-like default card).

import type { Component } from 'svelte';
import GeneralPreview from './GeneralPreview.svelte';
import InstagramPreview from './InstagramPreview.svelte';
import LinkedInPreview from './LinkedInPreview.svelte';
import FacebookPreview from './FacebookPreview.svelte';
// v24-8: new platform previews.
import XPreview from './XPreview.svelte';
import RedditPreview from './RedditPreview.svelte';
import ThreadsPreview from './ThreadsPreview.svelte';
import BlueskyPreview from './BlueskyPreview.svelte';

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
};

/** Get the preview component for a provider. Falls back to GeneralPreview. */
export function getPreviewComponent(provider: string): Component<any, any, any> {
  return PREVIEW_REGISTRY[provider] || GeneralPreview;
}

export { GeneralPreview };
