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

  // Per-provider character limits — kept in sync with the Rust
  // `Provider::max_content_length()` impls in src/social/. If a new
  // provider is added on the backend, add its limit here too.
  const PROVIDER_CHAR_LIMITS: Record<string, { label: string; limit: number }> = {
    x: { label: 'X', limit: 280 },
    reddit: { label: 'Reddit', limit: 10000 },
    linkedin: { label: 'LinkedIn', limit: 3000 },
    'linkedin-page': { label: 'LinkedIn', limit: 3000 },
    facebook: { label: 'Facebook', limit: 63206 },
    instagram: { label: 'Instagram', limit: 2200 },
    'instagram-standalone': { label: 'Instagram', limit: 2200 },
    threads: { label: 'Threads', limit: 500 },
    bluesky: { label: 'Bluesky', limit: 300 },
    mastodon: { label: 'Mastodon', limit: 500 },
    pinterest: { label: 'Pinterest', limit: 500 },
    tiktok: { label: 'TikTok', limit: 2200 },
    youtube: { label: 'YouTube', limit: 5000 },
    discord: { label: 'Discord', limit: 2000 },
    slack: { label: 'Slack', limit: 40000 },
    'telegram-bot': { label: 'Telegram', limit: 4096 },
    'telegram-user': { label: 'Telegram', limit: 4096 },
    whatsapp: { label: 'WhatsApp', limit: 65536 },
  };

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
      const meta = PROVIDER_CHAR_LIMITS[provider];
      if (!meta) continue;
      if (seen.has(provider)) continue;
      seen.add(provider);
      const count = plainTextLength;
      out.push({
        key: provider,
        label: meta.label,
        limit: meta.limit,
        count,
        isOver: count > meta.limit,
        isWarning: count > meta.limit * 0.9 && count <= meta.limit,
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
