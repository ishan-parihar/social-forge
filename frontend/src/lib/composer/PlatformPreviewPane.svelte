<script lang="ts">
  // PlatformPreviewPane — right-column preview container (Phase 4).
  //
  // Renders the platform-specific preview for the currently-selected
  // channel. When the user is in 'global' editing mode, shows the
  // GeneralPreview (default X-like card). When they switch to a specific
  // channel, shows that platform's custom preview (IG/LinkedIn/FB/etc.).
  //
  // Inspired by postiz-app's ShowAllProviders + withProvider pattern,
  // but simplified: we render ONE preview at a time (the current one),
  // not all of them with CSS-hidden switching. Svelte's reactivity is
  // fast enough that switching is instant without the portal trick.
  //
  // Security: all preview components escape user content before
  // rendering with {@html}. No raw user input is ever injected.

  import { getPreviewComponent } from './previews/index.js';
  import { providerMeta } from '$lib/providers';
  import type { MediaItem } from '$lib/api/media';

  let {
    content = '',
    current = 'global',  // 'global' or an integration ID
    selectedIntegrations = [] as string[],
    integrationProviders = new Map<string, string>(),
    integrationNames = new Map<string, string>(),
    media = [] as MediaItem[],
  }: {
    content?: string;
    current?: string;
    selectedIntegrations?: string[];
    integrationProviders?: Map<string, string>;
    integrationNames?: Map<string, string>;
    media?: MediaItem[];
  } = $props();

  // Determine which provider to preview.
  // - If current === 'global', use the first selected integration's
  //   provider (or 'x' as a neutral default if none selected).
  // - If current is an integration ID, use that integration's provider.
  let activeProvider = $derived.by(() => {
    if (current === 'global') {
      if (selectedIntegrations.length === 0) return 'x';
      return integrationProviders.get(selectedIntegrations[0]) || 'x';
    }
    return integrationProviders.get(current) || 'x';
  });

  let PreviewComponent = $derived(getPreviewComponent(activeProvider));
  let providerLabel = $derived(providerMeta(activeProvider).label);

  // Determine which content to show:
  // - If current === 'global', show the global content.
  // - If current is a specific integration, show that integration's
  //   override (or the global content if no override exists — but the
  //   parent ComposerModal ensures an override exists when current
  //   is set to a per-channel tab via clone-on-first-divergence).
  // For simplicity, we just show the `content` prop passed in — the
  // parent ComposerModal is responsible for passing the right content
  // based on the current tab.
</script>

<div class="space-y-2">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold">Preview</h3>
    <span class="text-xs text-muted">{providerLabel}</span>
  </div>

  {#if selectedIntegrations.length === 0}
    <div class="bg-surface border border-line rounded-xl p-8 text-center">
      <p class="text-sm text-muted">Select a channel to see a preview.</p>
    </div>
  {:else}
    <PreviewComponent
      {content}
      provider={activeProvider}
      authorName={current === 'global' ? 'Your Brand' : (integrationNames.get(current) || 'Your Brand')}
      authorHandle={(current === 'global' ? 'yourbrand' : (integrationNames.get(current) || 'yourbrand')).toLowerCase().replace(/\s/g, '')}
      {media}
    />
  {/if}
</div>
