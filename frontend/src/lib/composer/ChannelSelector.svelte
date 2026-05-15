<script lang="ts">
  import { onMount } from "svelte";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";

  let { selected = [], onToggle }: {
    selected?: string[];
    onToggle?: (id: string) => void;
  } = $props();

  let integrations = $state<Integration[]>([]);

  onMount(async () => {
    const r = await integrationsApi.list();
    if (r.data) integrations = r.data.integrations.filter(i => !i.disabled);
  });
</script>

<div>
  {#if integrations.length === 0}
    <div class="text-sm text-[#6b7280] py-4 text-center">
      No active channels. <a href="/channels" class="text-indigo-400 hover:underline">Connect a channel</a> first.
    </div>
  {:else}
    <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
      {#each integrations as int}
        {@const isSelected = selected.includes(int.id)}
        <button
          onclick={() => onToggle?.(int.id)}
          class="flex items-center gap-2 p-2.5 rounded-lg border transition-colors text-left {isSelected ? 'border-indigo-500 bg-indigo-500/10' : 'border-[#1e2435] hover:bg-[#1a1f2e]'}"
        >
          <ProviderIcon provider={int.provider_identifier} size="sm" />
          <div class="flex-1 min-w-0">
            <div class="text-xs truncate">{int.profile_name || int.provider_name}</div>
          </div>
          {#if isSelected}
            <span class="text-indigo-400 text-xs">✓</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
