<script lang="ts">
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount, onDestroy } from "svelte";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";
  import { groupIntegrations } from "$lib/channels/group-integrations";
  import ChannelCard from "$lib/channels/ChannelCard.svelte";
  import TimeSlotEditor from "$lib/channels/TimeSlotEditor.svelte";
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";
  import ConnectFlow from "$lib/channels/ConnectFlow.svelte";
  import { getAuthType, MULTI_AUTH_PROVIDERS } from "$lib/channels/auth-types";

  let integrations = $state<Integration[]>([]);
  let loading = $state(true);
  let error = $state("");
  let credDialog = $state<{ provider: string; type: "cookie" | "pat" } | null>(null);
  let credFields = $state<Record<string, string>>({});
  let connectChoice = $state<string | null>(null);
  let onboardDialog = $state<{
    provider: string;
    step: "phone" | "pair_code" | "polling" | "sms_code" | "bot_code" | "done";
    phone?: string;
    pairCode?: string;
    code?: string;
    instructions?: string;
  } | null>(null);
  const providerLabels: Record<string, string> = {
    x: "X (Twitter)", facebook: "Facebook", instagram: "Instagram",
    "instagram-standalone": "Instagram (Standalone)", threads: "Threads",
    linkedin: "LinkedIn", "linkedin-page": "LinkedIn Page",
    google: "Google Suite", youtube: "YouTube", google_my_business: "Google Business",
    reddit: "Reddit", bluesky: "Bluesky", discord: "Discord", pinterest: "Pinterest",
    tiktok: "TikTok", vk: "VK", kick: "Kick", mastodon: "Mastodon",
    "telegram-bot": "Telegram Bot", "telegram-user": "Telegram User",
    whatsapp: "WhatsApp", slack: "Slack",
    wordpress: "WordPress", medium: "Medium", devto: "Dev.to", hashnode: "Hashnode",
    github: "GitHub", lemmy: "Lemmy", whop: "Whop",
    farcaster: "Farcaster",
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
    "tiktok", "vk", "kick", "mastodon",
    "google_my_business", "whop", "slack",
    "telegram-bot", "telegram-user",
    "whatsapp",
    "wordpress", "medium", "devto", "hashnode",
    "github", "lemmy",
    "farcaster",
    "skool",
  ]);
  let connecting = $state<string | null>(null);
  let connectProvider = $state<string | null>(null);
  let scheduleIntegration = $state<{ id: string; timeslots: import("$lib/api/integrations").TimeslotEntry[] } | null>(null);

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
      toast(`Disconnect failed: ${e instanceof Error ? (e instanceof Error ? e.message : String(e)) : "unknown"}`, "error");
    }
  }

  function initiateConnect(provider: string) {
    const multiAuth = MULTI_AUTH_PROVIDERS[provider];
    if (multiAuth && !multiAuth.includes("oauth")) {
      credDialog = { provider, type: multiAuth[0] as "cookie" | "pat" };
      credFields = {};
      return;
    }
    if (multiAuth && multiAuth.includes("oauth") && multiAuth.includes("cookie")) {
      connectChoice = provider;
      return;
    }
    // WhatsApp / Telegram User: step-by-step onboarding
    if (provider === "whatsapp" || provider === "telegram-user") {
      onboardDialog = { provider, step: "phone", phone: "" };
      return;
    }
    const authType = getAuthType(provider);
    if (authType === "oauth") {
      connecting = provider;
      error = "";
      integrationsApi.connect(provider).then((r) => {
        if (r.error) { error = r.error; connecting = null; return; }
        if (!r.data?.url) { error = "Failed to initiate connection"; connecting = null; return; }
        if (r.data.state === "auto") { connecting = null; load(); return; }
        if (r.data.state === "one-time-token") {
          connecting = null;
          const match = r.data.url.match(/\/connect\s+(\S+)/);
          onboardDialog = { provider, step: "bot_code", instructions: r.data.url, code: match?.[1] ?? "" };
          return;
        }
        const popup = window.open(r.data.url, "_blank", "width=600,height=700");
        if (popup) {
          const onMessage = (e: MessageEvent) => {
            if (e.data?.type === "oauth-connected") { window.removeEventListener("message", onMessage); connecting = null; load(); }
          };
          window.addEventListener("message", onMessage);
          const interval = setInterval(() => { if (popup.closed) { clearInterval(interval); window.removeEventListener("message", onMessage); connecting = null; load(); } }, 1000);
        } else { connecting = null; }
      }).catch((e) => { error = "Failed to connect " + provider; toast(`Connect failed: ${e instanceof Error ? (e instanceof Error ? e.message : String(e)) : "unknown"}`, "error"); connecting = null; });
    } else {
      connectProvider = provider;
    }
  }

  function initiateOAuth(provider: string) {
    connecting = provider;
    error = "";
    integrationsApi.connect(provider).then((r) => {
      if (r.error) { error = r.error; connecting = null; return; }
      if (!r.data?.url) { error = "Failed to initiate connection"; connecting = null; return; }
      if (r.data.state === "auto") { connecting = null; load(); return; }
      const popup = window.open(r.data.url, "_blank", "width=600,height=700");
      if (popup) {
        const onMessage = (e: MessageEvent) => {
          if (e.data?.type === "oauth-connected") { window.removeEventListener("message", onMessage); connecting = null; load(); }
        };
        window.addEventListener("message", onMessage);
        const interval = setInterval(() => { if (popup.closed) { clearInterval(interval); window.removeEventListener("message", onMessage); connecting = null; load(); } }, 1000);
      } else { connecting = null; }
    }).catch(() => { error = "Failed to connect " + provider; connecting = null; });
  }

  async function submitCredDialog() {
    if (!credDialog) return;
    error = "";
    try {
      if (credDialog.provider === "x" && credDialog.type === "cookie") {
        const r = await integrationsApi.connectXCookie(credFields.auth_token || "", credFields.ct0 || "");
        if (r.error) { error = r.error; return; }
      } else if (credDialog.provider === "reddit" && credDialog.type === "cookie") {
        const r = await integrationsApi.connectRedditCookie(credFields.cookie_string || "");
        if (r.error) { error = r.error; return; }
      } else if (credDialog.provider === "github" && credDialog.type === "pat") {
        const r = await integrationsApi.connectGithubPat(credFields.pat || "", credFields.label || undefined);
        if (r.error) { error = r.error; return; }
      } else if (credDialog.provider === "telegram-bot") {
        const r = await integrationsApi.connectTelegramBotToken(credFields.token || "");
        if (r.error) { error = r.error; return; }
      }
      credDialog = null;
      credFields = {};
      await load();
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : undefined) || "Connection failed";
    }
  }

  function showXCookieDialog() {
    credDialog = { provider: "x", type: "cookie" };
    credFields = {};
  }

  let refreshing = $state<string | null>(null);

  async function handleChannelRefresh(id: string) {
    refreshing = id;
    try {
      const r = await integrationsApi.refresh(id);
      if (r.error) {
        error = `Refresh failed: ${r.error}`;
      } else {
        await load();
      }
    } catch (e: unknown) {
      error = `Refresh failed: ${(e instanceof Error ? e.message : undefined) || "Unknown error"}`;
    }
    refreshing = null;
  }

  async function handleReconnect(id: string) {
    refreshing = id;
    try {
      // Delete existing integration, then start fresh OAuth
      const int = integrations.find(i => i.id === id);
      if (!int) { error = "Integration not found"; refreshing = null; return; }
      await integrationsApi.disconnect(id);
      // Re-initiate the connect flow for this provider
      initiateConnect(int.provider_identifier);
    } catch (e: unknown) {
      error = `Reconnect failed: ${(e instanceof Error ? e.message : undefined) || "Unknown error"}`;
    }
    refreshing = null;
  }

  async function handleToggleDisableIntegration(id: string, disabled: boolean) {
    try {
      await integrationsApi.toggleDisable(id, disabled);
      await load();
    } catch (e) {
      error = "Failed to toggle channel";
      toast(`Toggle disable failed: ${e instanceof Error ? (e instanceof Error ? e.message : String(e)) : "unknown"}`, "error");
    }
  }

  async function onboardSubmitPhone() {
    if (!onboardDialog || !onboardDialog.phone) return;
    error = "";
    connecting = onboardDialog.provider;
    try {
      if (onboardDialog.provider === "whatsapp") {
        const r = await integrationsApi.whatsappPair(onboardDialog.phone);
        if (r.error) { error = r.error; connecting = null; return; }
        onboardDialog = { ...onboardDialog, step: "pair_code", pairCode: r.data?.pair_code ?? "" };
        connecting = null;
        // Start polling for auth
        pollWhatsAppAuth();
      } else if (onboardDialog.provider === "telegram-user") {
        const r = await integrationsApi.telegramUserRequestCode(onboardDialog.phone);
        if (r.error) { error = r.error; connecting = null; return; }
        onboardDialog = { ...onboardDialog, step: "sms_code", code: "" };
        connecting = null;
      }
    } catch (e: unknown) { error = (e instanceof Error ? e.message : undefined) || "Failed"; connecting = null; }
  }

  async function pollWhatsAppAuth() {
    for (let i = 0; i < 60; i++) {
      await new Promise(r => setTimeout(r, 3000));
      if (!onboardDialog || onboardDialog.provider !== "whatsapp") return;
      try {
        const r = await integrationsApi.whatsappStatus();
        if (r.data?.authenticated) {
          // Now verify to create integration
          const v = await integrationsApi.verifyOneTimeToken("whatsapp", "");
          if (v.error) { error = v.error; return; }
          onboardDialog = null;
          await load();
          return;
        }
      } catch { /* keep polling */ }
    }
    error = "Timed out waiting for WhatsApp authentication";
  }

  async function onboardSubmitCode() {
    if (!onboardDialog || !onboardDialog.code) return;
    error = "";
    connecting = onboardDialog.provider;
    try {
      if (onboardDialog.provider === "telegram-user") {
        const r = await integrationsApi.telegramUserSignIn(onboardDialog.code);
        if (r.error) { error = r.error; connecting = null; return; }
        onboardDialog = null;
        connecting = null;
        await load();
      } else {
        // telegram-bot verify
        const r = await integrationsApi.verifyOneTimeToken(onboardDialog.provider, onboardDialog.code ?? "");
        if (r.error) { error = r.error; connecting = null; return; }
        onboardDialog = null;
        connecting = null;
        await load();
      }
    } catch (e: unknown) { error = (e instanceof Error ? e.message : undefined) || "Verification failed"; connecting = null; }
  }

  function handleConnectSuccess() {
    connectProvider = null;
    load();
  }

  function handleConnectClose() {
    connectProvider = null;
  }

  let chanUnsubscribers: (() => void)[] = [];

  onMount(() => {
    load();
    const events = ['integration_connected', 'integration_disconnected'];
    for (const evt of events) {
      chanUnsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    chanUnsubscribers.forEach(fn => fn());
  });
</script>

<div class="page-enter space-y-6">
  <h2 class="text-xl font-semibold">Channel Management</h2>

  <!-- Connected channels -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Connected Channels</h3>
    {#if loading}
      <div class="text-center text-sm text-muted py-8">Loading...</div>
    {:else if error}
      <div class="text-center text-sm text-red-400 py-4">{error}</div>
    {:else if integrations.length === 0}
      <div class="text-center text-sm text-muted py-8">No channels connected yet. Select a provider below to connect.</div>
    {:else}
      {#each [...groups.entries()] as [name, ints] (name)}
        <div class="mb-4 last:mb-0">
          <div class="text-xs text-muted px-1 mb-1">{name} ({ints.length})</div>
          <div class="page-enter space-y-0.5">
            {#each ints as int (int.id)}
              <ChannelCard
                integration={int}
                timeslots={int.posting_times?.map((t: { time: number }) => ({ time: t.time })) || []}
                onDisconnect={disconnect}
                onRefresh={() => handleChannelRefresh(int.id)}
                onReconnect={() => handleReconnect(int.id)}
                onToggleDisable={handleToggleDisableIntegration}
                isRefreshing={refreshing === int.id}
              />
            {/each}
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <!-- Available providers grid -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-3">Available Providers</h3>
    <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
      {#each availableProviders as provider (provider)}
        <button
          onclick={() => initiateConnect(provider)}
          disabled={connecting === provider}
          class="flex flex-col items-center gap-2 p-4 bg-background-input border border-line rounded-xl hover:border-indigo-500/50 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
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

{#if connectChoice}
<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
  <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-sm">
    <h3 class="text-lg font-semibold mb-4">Connect {providerLabel(connectChoice)}</h3>
    <p class="text-sm text-muted mb-4">Choose how to connect:</p>
    <div class="flex flex-col gap-3">
      {#if connectChoice === "x"}
        <button onclick={() => { const p = connectChoice; connectChoice = null; initiateOAuth(p!); }} class="px-4 py-3 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-indigo-500/50 text-left">
          <div class="text-sm font-medium">OAuth 2.0</div>
          <div class="text-xs text-muted">Standard login — limited to API scopes</div>
        </button>
        <button onclick={() => { connectChoice = null; credDialog = { provider: "x", type: "cookie" }; credFields = {}; }} class="px-4 py-3 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-orange-500/50 text-left">
          <div class="text-sm font-medium">Browser Cookies</div>
          <div class="text-xs text-muted">Full access — DMs, analytics, advanced features</div>
        </button>
      {:else if connectChoice === "reddit"}
        <button onclick={() => { const p = connectChoice; connectChoice = null; initiateOAuth(p!); }} class="px-4 py-3 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-indigo-500/50 text-left">
          <div class="text-sm font-medium">OAuth 2.0</div>
          <div class="text-xs text-muted">Standard Reddit API access</div>
        </button>
        <button onclick={() => { connectChoice = null; credDialog = { provider: "reddit", type: "cookie" }; credFields = {}; }} class="px-4 py-3 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-orange-500/50 text-left">
          <div class="text-sm font-medium">Browser Cookies</div>
          <div class="text-xs text-muted">Full access — voting, moderation, all subreddits</div>
        </button>
      {:else if connectChoice === "telegram-bot"}
        <button onclick={() => { const p = connectChoice; connectChoice = null; initiateOAuth(p!); }} class="px-4 py-3 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-indigo-500/50 text-left">
          <div class="text-sm font-medium">Use Configured Bot</div>
          <div class="text-xs text-muted">Connect a chat/channel to the bot already set up in .env</div>
        </button>
        <button onclick={() => { connectChoice = null; credDialog = { provider: "telegram-bot", type: "pat" }; credFields = {}; }} class="px-4 py-3 bg-[#161b22] border border-[#30363d] rounded-lg hover:border-emerald-500/50 text-left">
          <div class="text-sm font-medium">Add Custom Bot Token</div>
          <div class="text-xs text-muted">Paste a bot token from @BotFather</div>
        </button>
      {/if}
    </div>
    <button onclick={() => connectChoice = null} class="mt-4 text-sm text-muted hover:text-white w-full text-center">Cancel</button>
  </div>
</div>
{/if}

{#if credDialog}
<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
  <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-md">
    <h3 class="text-lg font-semibold mb-4">
      {#if credDialog.provider === "x"}Connect X via Cookies{:else if credDialog.provider === "reddit"}Connect Reddit via Cookies{:else if credDialog.provider === "telegram-bot"}Add Telegram Bot{:else}Connect GitHub via PAT{/if}
    </h3>
    {#if credDialog.provider === "x" && credDialog.type === "cookie"}
      <label class="block text-sm text-muted mb-1">auth_token</label>
      <input type="text" bind:value={credFields.auth_token} placeholder="Paste auth_token cookie" class="w-full mb-3 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm" />
      <label class="block text-sm text-muted mb-1">ct0</label>
      <input type="text" bind:value={credFields.ct0} placeholder="Paste ct0 cookie" class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm" />
    {:else if credDialog.provider === "reddit" && credDialog.type === "cookie"}
      <p class="text-sm text-muted mb-3">Paste your Reddit cookie string from browser DevTools (Application → Cookies → www.reddit.com).</p>
      <label class="block text-sm text-muted mb-1">Cookie String</label>
      <textarea bind:value={credFields.cookie_string} placeholder="reddit_session=...; token_v2=...; csv=..." rows="4" class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm font-mono"></textarea>
    {:else if credDialog.provider === "telegram-bot"}
      <p class="text-sm text-muted mb-3">Get a token from <a href="https://t.me/BotFather" target="_blank" class="text-indigo-400 hover:text-indigo-300">@BotFather</a> on Telegram.</p>
      <label class="block text-sm text-muted mb-1">Bot Token</label>
      <input type="password" bind:value={credFields.token} placeholder="123456:ABC-DEF..." class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm font-mono" />
    {:else if credDialog.provider === "github"}
      <label class="block text-sm text-muted mb-1">Personal Access Token</label>
      <input type="password" bind:value={credFields.pat} placeholder="ghp_..." class="w-full mb-3 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm" />
      <label class="block text-sm text-muted mb-1">Label (optional)</label>
      <input type="text" bind:value={credFields.label} placeholder="My GitHub" class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm" />
    {/if}
    {#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}
    <div class="flex gap-3 justify-end">
      <button onclick={() => { credDialog = null; error = ""; }} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
      <button onclick={submitCredDialog} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded">Connect</button>
    </div>
  </div>
</div>
{/if}

{#if onboardDialog}
<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
  <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-sm">
    <h3 class="text-lg font-semibold mb-3">Connect {providerLabel(onboardDialog.provider)}</h3>

    {#if onboardDialog.step === "phone"}
      <p class="text-sm text-muted mb-3">
        {#if onboardDialog.provider === "whatsapp"}Enter your phone number to get a pairing code.{:else}Enter your phone number to receive a login code via Telegram.{/if}
      </p>
      <input type="tel" bind:value={onboardDialog.phone} placeholder="+1234567890" class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm" />
      {#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}
      <div class="flex justify-end gap-2">
        <button onclick={() => { onboardDialog = null; error = ""; }} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={onboardSubmitPhone} disabled={!!connecting} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">
          {connecting ? "Sending…" : "Next"}
        </button>
      </div>

    {:else if onboardDialog.step === "pair_code"}
      <p class="text-sm text-muted mb-2">Open WhatsApp on your phone:</p>
      <p class="text-sm text-[#c9d1d9] mb-3">Settings → Linked Devices → Link a Device → Enter code</p>
      <div class="bg-[#161b22] border border-[#30363d] rounded-lg p-4 mb-4 text-center">
        <span class="text-2xl font-mono font-bold tracking-[0.3em] text-white">{onboardDialog.pairCode}</span>
      </div>
      <p class="text-xs text-muted mb-3 text-center">Waiting for you to enter the code on your phone…</p>
      <div class="flex justify-center"><div class="animate-spin h-5 w-5 border-2 border-indigo-500 border-t-transparent rounded-full"></div></div>
      <div class="flex justify-end mt-4">
        <button onclick={() => { onboardDialog = null; error = ""; }} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
      </div>

    {:else if onboardDialog.step === "sms_code"}
      <p class="text-sm text-muted mb-3">Enter the code sent to your Telegram app.</p>
      <input type="text" bind:value={onboardDialog.code} placeholder="12345" class="w-full mb-4 px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm font-mono text-center text-lg tracking-widest" />
      {#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}
      <div class="flex justify-end gap-2">
        <button onclick={() => { onboardDialog = null; error = ""; }} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={onboardSubmitCode} disabled={!!connecting} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">
          {connecting ? "Signing in…" : "Sign In"}
        </button>
      </div>

    {:else if onboardDialog.step === "bot_code"}
      {@const parts = (onboardDialog.instructions ?? "").split("\n")}
      {@const botUsername = parts[0] ?? ""}
      {@const connectCmd = parts[1] ?? ""}
      <div class="page-enter space-y-3 mb-4">
        <p class="text-sm text-muted">1. Open this bot in Telegram:</p>
        <a href="https://t.me/{botUsername.replace('@','')}" target="_blank" class="block text-center text-indigo-400 hover:text-indigo-300 font-medium">{botUsername}</a>
        <p class="text-sm text-muted">2. Send this command to the bot or any group/channel it's in:</p>
        <div class="bg-[#161b22] border border-[#30363d] rounded-lg p-3 text-center">
          <code class="text-sm text-white font-mono">{connectCmd}</code>
        </div>
        <p class="text-sm text-muted">3. Click Verify below.</p>
      </div>
      {#if error}<p class="text-red-400 text-sm mb-3">{error}</p>{/if}
      <div class="flex justify-end gap-2">
        <button onclick={() => { onboardDialog = null; error = ""; }} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={onboardSubmitCode} disabled={!!connecting} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">
          {connecting ? "Verifying…" : "Verify"}
        </button>
      </div>
    {/if}
  </div>
</div>
{/if}
