<script lang="ts">
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount } from "svelte";
  import { groupIntegrations } from "$lib/channels/group-integrations";
  import ChannelCard from "$lib/channels/ChannelCard.svelte";
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";

  let integrations = $state<Integration[]>([]);
  let loading = $state(true);
  let error = $state("");
  let availableProviders = $state([
    "x", "facebook", "instagram", "threads", "linkedin", "linkedin-page",
    "youtube", "pinterest", "reddit", "bluesky", "discord", "telegram", "whatsapp", "skool", "lemmy",
    "twitch", "vk", "google_my_business", "whop",
    "mewe", "moltbook", "kick",
    "farcaster", "nostr",
  ]);
  let connecting = $state<string | null>(null);

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
    try {
      await integrationsApi.disconnect(id);
      await load();
    } catch (e) {
      error = "Failed to disconnect channel";
      console.error("Disconnect failed:", e);
    }
  }

  async function connect(provider: string) {
    connecting = provider;
    error = "";
    try {
      const r = await integrationsApi.connect(provider);
      if (r.data?.url) window.open(r.data.url, "_blank");
    } catch (e) {
      error = "Failed to connect " + provider;
      console.error("Connect failed:", e);
    }
    connecting = null;
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
    {:else if error}
      <div class="text-center text-sm text-red-400 py-4">{error}</div>
    {:else if integrations.length === 0}
      <div class="text-center text-sm text-[#6b7280] py-8">No channels connected yet. Select a provider below to connect.</div>
    {:else}
      {#each [...groups.entries()] as [name, ints] (name)}
        <div class="mb-4 last:mb-0">
          <div class="text-xs text-[#6b7280] px-1 mb-1">{name} ({ints.length})</div>
          <div class="space-y-0.5">
            {#each ints as int (int.id)}
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
      {#each availableProviders as provider (provider)}
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
