<script lang="ts">
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount } from "svelte";
  import ChannelCard from "$lib/channels/ChannelCard.svelte";
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";

  let integrations = $state<Integration[]>([]);
  let loading = $state(true);
  let availableProviders = $state([
    "x", "facebook", "instagram", "threads", "linkedin", "linkedin-page",
    "youtube", "pinterest", "reddit", "bluesky", "discord", "telegram", "whatsapp", "skool",
  ]);

  let groups = $derived.by(() => {
    const g = new Map<string, Integration[]>();
    for (const int of integrations) {
      const key = int.provider_name || int.provider_identifier;
      const existing = g.get(key) || [];
      existing.push(int);
      g.set(key, existing);
    }
    return g;
  });

  async function load() {
    loading = true;
    const r = await integrationsApi.list();
    if (r.data) integrations = r.data.integrations;
    loading = false;
  }

  async function disconnect(id: string) {
    await integrationsApi.disconnect(id);
    await load();
  }

  async function connect(provider: string) {
    const r = await integrationsApi.connect(provider);
    if (r.data?.url) window.open(r.data.url, "_blank");
  }

  onMount(load);
</script>

<div class="space-y-6">
  <h2 class="text-xl font-semibold">Channel Management</h2>

  <!-- Connected channels -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <h3 class="text-sm font-semibold text-[#6b7280] uppercase tracking-wider mb-3">Connected Channels</h3>
    {#if loading}
      <div class="text-center text-sm text-[#6b7280] py-8">Loading...</div>
    {:else if integrations.length === 0}
      <div class="text-center text-sm text-[#6b7280] py-8">No channels connected yet. Select a provider below to connect.</div>
    {:else}
      {#each [...groups.entries()] as [name, ints]}
        <div class="mb-4 last:mb-0">
          <div class="text-xs text-[#6b7280] px-1 mb-1">{name} ({ints.length})</div>
          <div class="space-y-0.5">
            {#each ints as int}
              <ChannelCard integration={int} onDisconnect={disconnect} />
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Available providers grid -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <h3 class="text-sm font-semibold text-[#6b7280] uppercase tracking-wider mb-3">Available Providers</h3>
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
      {#each availableProviders as provider}
        <button
          onclick={() => connect(provider)}
          class="flex flex-col items-center gap-2 p-4 bg-[#0d1117] border border-[#1e2435] rounded-xl hover:border-indigo-500/50 transition-colors"
        >
          <ProviderIcon {provider} size="lg" />
          <span class="text-xs capitalize">{provider.replace("-", " ")}</span>
        </button>
      {/each}
    </div>
  </div>
</div>
