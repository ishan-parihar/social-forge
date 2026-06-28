<script lang="ts">
  import ProviderIcon from "./ProviderIcon.svelte";
  import ChannelContextMenu from "./ChannelContextMenu.svelte";
  import { integrationsApi, type Integration, type TimeslotEntry } from "$lib/api/integrations";
  import { getAuthType } from "./auth-types";

  let { integration, timeslots, onDisconnect, onRefresh, onReconnect, onToggleDisable, isRefreshing }: {
    integration: Integration;
    timeslots?: TimeslotEntry[];
    onDisconnect?: (id: string) => void;
    onRefresh?: (id: string) => void;
    onReconnect?: (id: string) => void;
    onToggleDisable?: (id: string, disabled: boolean) => void;
    isRefreshing?: boolean;
  } = $props();

  let currentTimeslots = $derived(
    timeslots ?? (Array.isArray(integration.posting_times) ? integration.posting_times : [])
  );

  let authType = $derived(getAuthType(integration.provider_identifier));

  let authTypeLabel = $derived.by(() => {
    const m = integration.auth_method;
    if (m === "cookie") return "Cookie";
    if (m === "pat") return "PAT";
    if (m === "api_key") return "API Key";
    switch (authType) {
      case "api_key": return "API Key";
      case "web3": return "Web3";
      case "extension": return "Extension";
      default: return "";
    }
  });

  let authTypeColor = $derived.by(() => {
    const m = integration.auth_method;
    if (m === "cookie") return "text-orange-400 border-orange-400/30 bg-orange-400/10";
    if (m === "pat") return "text-emerald-400 border-emerald-400/30 bg-emerald-400/10";
    if (m === "api_key") return "text-amber-400 border-amber-400/30 bg-amber-400/10";
    switch (authType) {
      case "api_key": return "text-amber-400 border-amber-400/30 bg-amber-400/10";
      case "web3": return "text-purple-400 border-purple-400/30 bg-purple-400/10";
      case "extension": return "text-cyan-400 border-cyan-400/30 bg-cyan-400/10";
      default: return "";
    }
  });

  function handleRename() {
    const newName = prompt("Rename channel:", integration.profile_name || integration.provider_name || "");
    if (newName && newName !== (integration.profile_name || integration.provider_name)) {
      console.log("Rename not yet implemented — would rename to:", newName);
    }
  }

  function handleCopyId() {
    navigator.clipboard.writeText(integration.id);
  }

  async function handleToggleDisable() {
    const newDisabled = !integration.disabled;
    if (onToggleDisable) {
      onToggleDisable(integration.id, newDisabled);
    } else {
      try {
        const r = await integrationsApi.toggleDisable(integration.id, newDisabled);
        if (r.error) console.error("Toggle disable failed:", r.error);
      } catch (e) {
        console.error("Toggle disable error:", e);
      }
    }
  }

  function handleDelete() {
    onDisconnect?.(integration.id);
  }
</script>

<div class="flex items-center gap-3 px-3 py-2.5 hover:bg-[#1a1f2e] rounded-lg transition-colors group">
  {#if integration.profile_picture}
    <img src={integration.profile_picture} alt="" class="w-8 h-8 rounded-full object-cover flex-shrink-0" />
  {:else}
    <ProviderIcon provider={integration.provider_identifier} size="sm" />
  {/if}
  <div class="flex-1 min-w-0">
    <div class="text-sm truncate flex items-center gap-2">
      {integration.profile_name || integration.provider_name}
      {#if integration.root_internal_id}
        <span class="text-[10px] px-1.5 py-0.5 rounded border text-indigo-400 border-indigo-400/30 bg-indigo-400/10">Page</span>
      {/if}
      {#if authTypeLabel}
        <span class="text-[10px] px-1.5 py-0.5 rounded border {authTypeColor}">{authTypeLabel}</span>
      {/if}
    </div>
    <div class="text-xs text-[#6b7280] truncate">
      {#if integration.profile_url}
        {integration.profile_url}
      {:else if integration.internal_id}
        @{integration.internal_id}
      {:else}
        {integration.provider_name}
      {/if}
    </div>
  </div>
  <div class="shrink-0 flex items-center gap-2">
    {#if integration.disabled}
      <span class="w-2 h-2 rounded-full bg-red-500" title="Disabled"></span>
    {:else if integration.refresh_needed}
      <div class="flex items-center gap-1.5">
        <span class="w-2 h-2 rounded-full bg-yellow-500" title="Refresh needed"></span>
        <span class="text-[10px] text-yellow-400 hidden sm:inline">Token needs refresh</span>
      </div>
    {:else}
      <span class="w-2 h-2 rounded-full bg-green-500" title="Connected"></span>
    {/if}
    <ChannelContextMenu
      integrationId={integration.id}
      integrationName={integration.profile_name || integration.provider_name}
      currentTimeslots={currentTimeslots}
      disabled={integration.disabled}
      isRefreshing={isRefreshing}
      onRefreshToken={integration.refresh_needed || authType === "oauth" ? () => onRefresh?.(integration.id) : undefined}
      onReconnect={onReconnect ? () => onReconnect(integration.id) : undefined}
      onRename={handleRename}
      onToggleDisable={handleToggleDisable}
      onCopyId={handleCopyId}
      onDelete={handleDelete}
    />
  </div>
</div>
