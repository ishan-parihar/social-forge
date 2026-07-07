// Composer store (Phase 2 — keystone for the modal-based composer).
//
// This store drives the ComposerModal: open/close, mode (create vs edit),
// preset date/integrations, and the editing post id. The actual composer
// form state (content, media, tags, etc.) lives inside ComposerModal.svelte
// as local component state — only the open/close + entry-point state is
// global, because that's what other components need to trigger.
//
// Inspired by postiz-app's useLaunchStore + useExistingData pattern, but
// simplified for Svelte 5 runes and Social Forge's single-user model.
//
// Usage:
//   import { composer } from '$lib/stores/composer.svelte';
//
//   // Open for create, optionally with a preset date (from calendar slot click):
//   composer.openCreate('2026-07-10');
//
//   // Open for create, optionally with preset integrations (from channel menu):
//   composer.openCreate(undefined, ['int-id-1', 'int-id-2']);
//
//   // Open for edit:
//   composer.openEdit('post-uuid');
//
//   // Close (with askClose confirm handled inside ComposerModal):
//   composer.close();

class ComposerStore {
  /** Is the composer modal currently open? */
  open = $state(false);

  /** Mode: 'create' (new post) or 'edit' (existing post). */
  mode = $state<'create' | 'edit'>('create');

  /** For create mode: optional preset date (YYYY-MM-DD) from calendar slot. */
  presetDate = $state<string | null>(null);

  /** For create mode: optional preset integration IDs (from channel menu). */
  presetIntegrationIds = $state<string[]>([]);

  /** For edit mode: the post id to edit. ComposerModal will fetch the full detail. */
  editingPostId = $state<string | null>(null);

  /** For create mode: optional prefilled content (from "duplicate" flow). */
  prefilledContent = $state<string | null>(null);

  /**
   * Phase v21/5.7: optional prefilled media items (from repurpose flow).
   * Each item has {url, mime_type, original_name} — the composer wraps
   * them with a generated id + file_size=0 when loading. The media is
   * already uploaded (these are URLs to existing media), so no upload
   * progress is needed.
   */
  prefilledMedia = $state<Array<{ url: string; mime_type: string; original_name?: string }> | null>(null);

  /** Open the composer in create mode. */
  openCreate(
    presetDate?: string | null,
    presetIntegrationIds?: string[],
    prefilledContent?: string,
    prefilledMedia?: Array<{ url: string; mime_type: string; original_name?: string }>,
  ) {
    this.mode = 'create';
    this.presetDate = presetDate ?? null;
    this.presetIntegrationIds = presetIntegrationIds ?? [];
    this.prefilledContent = prefilledContent ?? null;
    this.prefilledMedia = prefilledMedia ?? null;
    this.editingPostId = null;
    this.open = true;
  }

  /** Open the composer in edit mode for an existing post. */
  openEdit(postId: string) {
    this.mode = 'edit';
    this.editingPostId = postId;
    this.presetDate = null;
    this.presetIntegrationIds = [];
    this.prefilledContent = null;
    this.prefilledMedia = null;
    this.open = true;
  }

  /** Close the composer. ComposerModal's onClose handles the askClose confirm. */
  close() {
    this.open = false;
    // Reset entry-point state after the modal unmounts.
    setTimeout(() => {
      this.presetDate = null;
      this.presetIntegrationIds = [];
      this.editingPostId = null;
      this.prefilledContent = null;
      this.prefilledMedia = null;
    }, 100);
  }
}

export const composer = new ComposerStore();
