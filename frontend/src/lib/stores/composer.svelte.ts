// Composer state — isolated, multi-field, mimics Zustand pattern
// Each field is $state in a module-level object for Svelte 5 reactivity

import type { MediaItem } from "$lib/api/media";

// Per-channel content overrides (Social Forge pattern: different content per platform)
export interface ChannelContent {
  integrationId: string;
  content: string;        // platform-specific override
  mediaIds: string[];
}

// Using a simple reactive object pattern (no external deps)
let _state = $state({
  open: false,
  content: "",
  title: "",
  scheduledAt: null as string | null,
  selectedIntegrationIds: [] as string[],
  channelContents: [] as ChannelContent[],
  mediaItems: [] as MediaItem[],
  isScheduling: false,
  isSubmitting: false,
});

export const composer = {
  get state() { return _state; },

  open() { _state.open = true; },
  close() { _state.open = false; this.reset(); },

  setContent(content: string) { _state.content = content; },
  setTitle(title: string) { _state.title = title; },
  setScheduledAt(dt: string | null) { _state.scheduledAt = dt; },

  toggleIntegration(id: string) {
    const idx = _state.selectedIntegrationIds.indexOf(id);
    if (idx >= 0) _state.selectedIntegrationIds.splice(idx, 1);
    else _state.selectedIntegrationIds.push(id);
  },

  setChannelContent(integrationId: string, content: string) {
    const existing = _state.channelContents.find(c => c.integrationId === integrationId);
    if (existing) existing.content = content;
    else _state.channelContents.push({ integrationId, content, mediaIds: [] });
  },

  addMedia(item: MediaItem) { _state.mediaItems.push(item); },
  removeMedia(id: string) {
    _state.mediaItems = _state.mediaItems.filter(m => m.id !== id);
  },

  reset() {
    _state.content = "";
    _state.title = "";
    _state.scheduledAt = null;
    _state.selectedIntegrationIds = [];
    _state.channelContents = [];
    _state.mediaItems = [];
    _state.isScheduling = false;
    _state.isSubmitting = false;
  },
};
