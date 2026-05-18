<script lang="ts">
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount } from "svelte";
  import { groupIntegrations } from "$lib/channels/group-integrations";
  import ChannelCard from "$lib/channels/ChannelCard.svelte";
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";
  import ConnectFlow from "$lib/channels/ConnectFlow.svelte";
  import { getAuthType } from "$lib/channels/auth-types";

  let integrations = $state<Integration[]>([]);
  let loading = $state(true);
  let error = $state("");
  const providerLabels: Record<string, string> = {
    x: "X (Twitter)", facebook: "Facebook", instagram: "Instagram",
    "instagram-standalone": "Instagram (Standalone)", threads: "Threads",
    linkedin: "LinkedIn", "linkedin-page": "LinkedIn Page",
    google: "Google Suite", youtube: "YouTube", google_my_business: "Google Business",
    reddit: "Reddit", bluesky: "Bluesky", discord: "Discord", pinterest: "Pinterest",
    tiktok: "TikTok", twitch: "Twitch", vk: "VK", mewe: "MeWe",
    moltbook: "Moltbook", kick: "Kick", mastodon: "Mastodon",
    "telegram-bot": "Telegram Bot", "telegram-user": "Telegram User",
    whatsapp: "WhatsApp", slack: "Slack",
    wordpress: "WordPress", medium: "Medium", devto: "Dev.to", hashnode: "Hashnode",
    github: "GitHub", lemmy: "Lemmy", whop: "Whop",
    farcaster: "Farcaster", nostr: "Nostr",
    skool: "Skool",
  };

  function providerLabel(provider: string): string {
    return providerLabels[provider] ?? provider.replace(/_/g, " ");
  }

  let availableProviders = $state([
    "x", "facebook", "instagram", "instagram-standalone", "threads",
    "linkedin", "linkedin-page",
    "google",
    "reddit", "bluesky", "discord", "pinterest",
    "tiktok", "twitch", "vk", "mewe", "moltbook", "kick", "mastodon",
    "google_my_business", "whop", "slack",
    "telegram-bot", "telegram-user",
    "whatsapp",
    "wordpress", "medium", "devto", "hashnode",
    "github", "lemmy",
    "farcaster", "nostr",
    "skool",
  ]);
  let connecting = $state<string | null>(null);
  let connectProvider = $state<string | null>(null);

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

  function initiateConnect(provider: string) {
    const authType = getAuthType(provider);
    if (authType === "oauth") {
      // OAuth flow: open popup, listen for postMessage
      connecting = provider;
      error = "";
      integrationsApi.connect(provider).then((r) => {
        if (r.error) {
          error = r.error;
          connecting = null;
          return;
        }
        if (!r.data?.url) {
          error = "Failed to initiate connection";
          connecting = null;
          return;
        }
        // Non-OAuth auto-connect: provider connected server-side, just reload
        if (r.data.state === "auto") {
          connecting = null;
          load();
          return;
        }
        // OAuth: open popup and wait for postMessage callback
        const popup = window.open(r.data.url, "_blank", "width=600,height=700");
        if (popup) {
          const onMessage = (e: MessageEvent) => {
            if (e.data?.type === "oauth-connected") {
              window.removeEventListener("message", onMessage);
              connecting = null;
              load();
            }
          };
          window.addEventListener("message", onMessage);
          // Fallback: poll popup closed in case postMessage fails
          const interval = setInterval(() => {
            if (popup.closed) {
              clearInterval(interval);
              window.removeEventListener("message", onMessage);
              connecting = null;
              load();
            }
          }, 1000);
        } else {
          // Popup blocked — fallback to same window
          connecting = null;
        }
      }).catch((e) => {
        error = "Failed to connect " + provider;
        console.error("Connect failed:", e);
        connecting = null;
      });
    } else {
      // Non-OAuth: show connect dialog
      connectProvider = provider;
    }
  }

  async function handleChannelRefresh(id: string) {
    try {
      await integrationsApi.refresh(id);
      await load();
    } catch (e) {
      error = "Failed to refresh token";
      console.error("Refresh failed:", e);
    }
  }

  async function handleToggleDisableIntegration(id: string, disabled: boolean) {
    try {
      await integrationsApi.toggleDisable(id, disabled);
      await load();
    } catch (e) {
      error = "Failed to toggle channel";
      console.error("Toggle disable failed:", e);
    }
  }

  function handleConnectSuccess() {
    connectProvider = null;
    load();
  }

  function handleConnectClose() {
    connectProvider = null;
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
              <ChannelCard integration={int} onDisconnect={disconnect} onRefresh={() => handleChannelRefresh(int.id)} onToggleDisable={handleToggleDisableIntegration} />
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
          onclick={() => initiateConnect(provider)}
          disabled={connecting === provider}
          class="flex flex-col items-center gap-2 p-4 bg-[#0d1117] border border-[#1e2435] rounded-xl hover:border-indigo-500/50 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        >
          <ProviderIcon {provider} size="lg" />
          <span class="text-xs">{providerLabel(provider)}</span>
        </button>
      {/each}
    </div>
  </div>
</div>

<ConnectFlow
  provider={connectProvider ?? ""}
  show={connectProvider !== null}
  onSuccess={handleConnectSuccess}
  onClose={handleConnectClose}
/>
