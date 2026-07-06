<script lang="ts">
  // Per-platform character counter for the composer (R-4 / U-3).
  // Shows a row of small badges — one per selected channel — each
  // displaying the current char count vs that platform's limit.
  // Turns yellow at 90% of the limit, red when over.
  //
  // This is the "global mode" companion to the per-channel
  // ProviderEditor: when the user is composing shared content for
  // multiple channels at once, they still need to see how close they
  // are to each platform's individual limit (X=280, Threads=500, etc.).
  //
  // Provider metadata (label, charLimit) comes from the central
  // $lib/providers.ts module (R-8) so there's a single source of truth.

  import { providerMeta } from "$lib/providers";

  let { content, selectedIntegrations, integrationProviders, integrationNames }: {
    content: string;
    selectedIntegrations: string[];
    integrationProviders: Map<string, string>;
    integrationNames: Map<string, string>;
  } = $props();

  // Plain-text length of the content (HTML tags stripped).
  // Matches what the backend's `sanitize_content` will measure.
  let plainTextLength = $derived(content.replace(/<[^>]*>/g, '').length);

  // Build the list of {label, limit, count, isOver, isWarning} badges
  // for each currently-selected integration, deduplicating by provider
  // so multiple X accounts don't show "X 280" twice.
  let badges = $derived.by(() => {
    const seen = new Set<string>();
    const out: Array<{ key: string; label: string; limit: number; count: number; isOver: boolean; isWarning: boolean }> = [];
    for (const intId of selectedIntegrations) {
      const provider = integrationProviders.get(intId);
      if (!provider) continue;
      if (seen.has(provider)) continue;
      seen.add(provider);
      const meta = providerMeta(provider);
      const count = plainTextLength;
      out.push({
        key: provider,
        label: meta.label,
        limit: meta.charLimit,
        count,
        isOver: count > meta.charLimit,
        isWarning: count > meta.charLimit * 0.9 && count <= meta.charLimit,
      });
    }
    return out;
  });
</script>

{#if badges.length > 0}
  <div class="flex flex-wrap gap-2">
    {#each badges as b (b.key)}
      <span
        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-medium border
          {b.isOver
            ? 'bg-red-500/10 border-red-500/30 text-red-400'
            : b.isWarning
              ? 'bg-yellow-500/10 border-yellow-500/30 text-yellow-400'
              : 'bg-surface-hover border-line text-muted'}"
        title="{b.label} limit: {b.limit} chars"
      >
        <span>{b.label}</span>
        <span class="font-mono">{b.count}/{b.limit}</span>
      </span>
    {/each}
  </div>
{/if}
