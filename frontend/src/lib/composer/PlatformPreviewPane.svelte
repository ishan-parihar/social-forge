<script lang="ts">
  // PlatformPreviewPane — the right-column live preview (v25 F2).
  //
  // F2 changed the shape of this component. Before, it rendered ONE preview:
  // whichever channel the SelectCurrent tab strip happened to be on. That is
  // a fine design for a two-channel post and a bad one for the thing this app
  // is for — a solo founder firing the same thought at X, LinkedIn, a Page,
  // Facebook, IG, Threads, YouTube, TikTok, Reddit, Bluesky and Pinterest at
  // once. You cannot see what any of them will look like without clicking
  // through every tab. So it now renders one frame per SELECTED channel, and
  // marks which one the editor is currently pointed at.
  //
  // Per-integration content is resolved here (override → global) rather than
  // by the parent, because the pane is now the thing that knows about every
  // integration at once. That also removes the old prop mismatch where the
  // parent could only hand over a single `content` string.
  //
  // Every frame gets the full post context — title, media, first comment,
  // schedule, TikTok sound, Pinterest board — because a preview that omits the
  // required field is not frame-accurate; it is decoration.

  import { getPreviewComponent, hasDedicatedPreview } from './previews/index.js';
  import { providerMeta } from '$lib/providers';
  import { specFor, blockersFor } from './platforms';
  import ProviderIcon from '$lib/channels/ProviderIcon.svelte';
  import { timezone } from '$lib/stores/timezone.svelte';
  import type { MediaItem } from '$lib/api/media';
  import type { TargetInfo } from '$lib/api/integrations';

  let {
    content = '',
    overrides = new Map<string, string>(),
    current = 'global',
    selectedIntegrations = [] as string[],
    integrationProviders = new Map<string, string>(),
    integrationNames = new Map<string, string>(),
    media = [] as MediaItem[],
    title = '',
    firstComment = '',
    scheduledAt = null as string | null,
    audioTitle = '',
    targetNames = new Map<string, string>(),
  }: {
    content?: string;
    overrides?: Map<string, string>;
    current?: string;
    selectedIntegrations?: string[];
    integrationProviders?: Map<string, string>;
    integrationNames?: Map<string, string>;
    media?: MediaItem[];
    title?: string;
    firstComment?: string;
    scheduledAt?: string | null;
    audioTitle?: string;
    targetNames?: Map<string, string>;
  } = $props();

  // Dedup by provider: two X accounts would otherwise render the same card
  // twice and push the other 10 platforms off the pane. The account name in
  // the frame header still distinguishes them.
  let frames = $derived.by(() => {
    const seen = new Set<string>();
    const out: Array<{ id: string; provider: string; label: string; name: string; body: string; focused: boolean }> = [];
    for (const id of selectedIntegrations) {
      const provider = integrationProviders.get(id) || 'x';
      if (seen.has(provider)) continue;
      seen.add(provider);
      out.push({
        id,
        provider,
        label: specFor(provider)?.label ?? providerMeta(provider).label,
        name: integrationNames.get(id) || 'Your Brand',
        // An override is what this channel will actually publish; falling back
        // to the global body is correct because that is also what it publishes.
        body: overrides.get(id) ?? content,
        focused: current === id,
      });
    }
    return out;
  });

  let imageCount = $derived(media.filter(m => m.mime_type?.startsWith('image/')).length);
  let videoCount = $derived(media.length - imageCount);

  // The first reason this platform will reject the post, if any. Shown as a
  // warning strip under the frame so the preview explains its own warning
  // instead of the user wondering why the card is highlighted.
  function frameBlocker(provider: string, body: string): string | null {
    const b = blockersFor(provider, body, {
      title,
      mediaCount: media.length,
      imageCount,
      videoCount,
    });
    return b.length > 0 ? b[0].message : null;
  }
</script>

<div class="space-y-2">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold">Preview</h3>
    <span class="text-xs text-muted">
      {frames.length === 0 ? 'no channels' : `${frames.length} channel${frames.length > 1 ? 's' : ''}`}
    </span>
  </div>

  {#if frames.length === 0}
    <div class="bg-surface border border-line rounded-xl p-8 text-center">
      <p class="text-sm text-muted">Select a channel to see a preview.</p>
      <p class="text-xs text-faint mt-1">
        Every selected channel gets its own frame, in its own chrome.
      </p>
    </div>
  {:else}
    <div class="space-y-3">
      {#each frames as f (f.id)}
        {@const PreviewComponent = getPreviewComponent(f.provider)}
        {@const blocker = frameBlocker(f.provider, f.body)}
        <section
          class="rounded-xl border transition-colors
            {f.focused ? 'border-accent ring-1 ring-accent/40' : 'border-line'}"
          aria-label="Preview for {f.label}"
        >
          <!-- Frame header: which channel this card belongs to. Without it a
               user with 5 connected accounts cannot tell which card is whose. -->
          <div class="flex items-center gap-2 px-3 py-2 border-b border-line">
            <ProviderIcon provider={f.provider} size="sm" />
            <div class="min-w-0 flex-1">
              <div class="text-xs font-medium truncate">{f.name}</div>
              <div class="text-[10px] text-faint truncate">
                {f.label}{#if !hasDedicatedPreview(f.provider)} · generic preview{/if}
              </div>
            </div>
            {#if f.focused}
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-accent-soft text-accent shrink-0">
                editing
              </span>
            {/if}
          </div>

          <div class="p-2 bg-background">
            <PreviewComponent
              content={f.body}
              provider={f.provider}
              authorName={f.name}
              authorHandle={f.name.toLowerCase().replace(/\s/g, '')}
              {title}
              {media}
              {firstComment}
              {scheduledAt}
              timezone={timezone.value}
              {audioTitle}
              targetLabel={targetNames.get(f.id) || ''}
            />
          </div>

          {#if blocker}
            <p class="text-[11px] text-warning px-3 py-1.5 border-t border-line">
              {blocker}
            </p>
          {/if}
          {#if scheduledAt}
            <p class="text-[11px] text-faint px-3 py-1.5 border-t border-line">
              Publishes {timezone.formatDateTime(scheduledAt)}
              <span class="text-faint">({timezone.value})</span>
            </p>
          {/if}
        </section>
      {/each}
    </div>
  {/if}
</div>
