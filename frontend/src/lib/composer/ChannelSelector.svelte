<script lang="ts">
  import { onMount } from "svelte";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";
  import { toast } from "$lib/stores/toast";

  let { selected = [], onToggle }: {
    selected?: string[];
    onToggle?: (id: string) => void;
  } = $props();

  let integrations = $state<Integration[]>([]);
  let skippedCount = $state(0);

  onMount(async () => {
    try {
      const [r, provRes] = await Promise.all([
        integrationsApi.list(),
        integrationsApi.listProviders(),
      ]);
      if (!r.data) return;
      const active = r.data.integrations.filter(i => !i.disabled);

      // A channel that cannot take a post must not be offered here. The
      // signal is the provider's own content limit: Gmail, Drive and
      // GitHub report 0, because they have no "publish a post" surface
      // at all. The composer used to list them, and a post to one came
      // back as a nonsensical 400 ("Content is 42 chars, max 0 for
      // Gmail") at submit time.
      const limits = new Map<string, number>();
      for (const p of provRes.data ?? []) limits.set(p.identifier, p.char_limit);

      const publishable = active.filter((i) => {
        const limit = limits.get(i.provider_identifier);
        // A provider the server did not report is kept: the limit list
        // is best-effort, and hiding a channel the user connected is
        // worse than showing one that turns out to reject the post.
        return limit === undefined || limit > 0;
      });
      skippedCount = active.length - publishable.length;
      integrations = publishable;
    } catch (e) {
      toast(`Failed to load integrations: ${e instanceof Error ? e.message : "unknown"}`, "error");
      integrations = [];
    }
  });
</script>

<div>
  {#if integrations.length === 0}
    <div class="text-sm text-muted py-4 text-center">
      No active channels. <a href="/channels" class="text-accent hover:underline">Connect a channel</a> first.
    </div>
  {:else}
    {#if skippedCount > 0}
      <p class="text-[11px] text-muted mb-2">
        {skippedCount} connected {skippedCount === 1 ? "channel is" : "channels are"} hidden:
        they cannot publish posts.
      </p>
    {/if}
    <div class="grid grid-cols-2 sm:grid-cols-3 gap-2" role="group" aria-label="Channels">
      {#each integrations as int (int.id)}
        {@const isSelected = selected.includes(int.id)}
        {@const name = int.profile_name || int.provider_name}
        <button
          onclick={() => onToggle?.(int.id)}
          class="flex items-center gap-2 p-2.5 rounded-lg border transition-colors text-left {isSelected ? 'border-accent bg-accent-fill/10' : 'border-line hover:bg-surface-hover'}"
          aria-pressed={isSelected}
          aria-label="{isSelected ? 'Remove' : 'Add'} {name}"
        >
          <ProviderIcon provider={int.provider_identifier} size="sm" />
          <div class="flex-1 min-w-0">
            <div class="text-xs truncate">{name}</div>
          </div>
          {#if isSelected}
            <span class="text-accent text-xs" aria-hidden="true">✓</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
