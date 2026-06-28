<script lang="ts">
  import { onMount } from "svelte";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { groupIntegrations } from "./group-integrations";
  import ChannelCard from "./ChannelCard.svelte";

  let { onConnect, onDisconnect: externalDisconnect, onRefresh: externalRefresh }: {
    onConnect?: (provider: string) => void;
    onDisconnect?: (id: string) => void;
    onRefresh?: (id: string) => void;
  } = $props();
  let integrations = $state<Integration[]>([]);
  let loading = $state(true);
  let error = $state("");

  let groups = $derived.by(() => groupIntegrations(integrations));

  async function load() {
    loading = true;
    error = "";
    try {
      const r = await integrationsApi.list();
      if (r.data) integrations = r.data.integrations;
    } catch {
      error = "Failed to load channels";
    }
    loading = false;
  }

  async function disconnect(id: string) {
    if (externalDisconnect) {
      externalDisconnect(id);
    } else {
      await integrationsApi.disconnect(id);
      await load();
    }
  }

  export function reloadIntegrations() {
    return load();
  }

  onMount(load);
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold uppercase tracking-wider text-[#6b7280]">Channels</h3>
    <span class="text-xs text-[#6b7280]">{integrations.length}</span>
  </div>

  {#if loading}
    <div class="text-center text-sm text-[#6b7280] py-4">Loading...</div>
  {:else if error}
    <div class="text-center text-sm text-red-400 py-4">{error}</div>
  {:else if integrations.length === 0}
    <div class="text-center text-sm text-[#6b7280] py-4">
      <p>No channels connected</p>
      <button onclick={() => onConnect?.("")} class="text-indigo-400 hover:underline mt-2">Connect one</button>
    </div>
  {:else}
    {#each [...groups.entries()] as [name, ints]}
      <div>
        <div class="text-xs text-[#6b7280] px-3 py-1">{name} ({ints.length})</div>
        {#each ints as int}
          <ChannelCard integration={int} onDisconnect={disconnect} onRefresh={externalRefresh ? () => externalRefresh(int.id) : undefined} />
        {/each}
      </div>
    {/each}
  {/if}
</div>
