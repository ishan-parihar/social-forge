<script lang="ts">
  import ProviderIcon from "./ProviderIcon.svelte";
  import ChannelContextMenu from "./ChannelContextMenu.svelte";
  import { integrationsApi, type Integration, type TimeslotEntry } from "$lib/api/integrations";

  let { integration, timeslots, onDisconnect, onRefresh, onToggleDisable }: {
    integration: Integration;
    timeslots?: TimeslotEntry[];
    onDisconnect?: (id: string) => void;
    onRefresh?: (id: string) => void;
    onToggleDisable?: (id: string, disabled: boolean) => void;
  } = $props();

  let currentTimeslots = $derived(
    timeslots ?? (Array.isArray(integration.posting_times) ? integration.posting_times : [])
  );

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
  <ProviderIcon provider={integration.provider_identifier} size="sm" />
  <div class="flex-1 min-w-0">
    <div class="text-sm truncate">{integration.profile_name || integration.provider_name}</div>
    <div class="text-xs text-[#6b7280] truncate">{integration.provider_identifier}</div>
  </div>
  <div class="shrink-0 flex items-center gap-1">
    {#if integration.disabled}
      <span class="w-2 h-2 rounded-full bg-red-500" title="Disabled"></span>
    {:else if integration.refresh_needed}
      <span class="w-2 h-2 rounded-full bg-yellow-500" title="Refresh needed"></span>
    {:else}
      <span class="w-2 h-2 rounded-full bg-green-500" title="Connected"></span>
    {/if}
    <ChannelContextMenu
      integrationId={integration.id}
      integrationName={integration.profile_name || integration.provider_name}
      currentTimeslots={currentTimeslots}
      disabled={integration.disabled}
      onRefreshToken={integration.refresh_needed ? () => onRefresh?.(integration.id) : undefined}
      onRename={handleRename}
      onToggleDisable={handleToggleDisable}
      onCopyId={handleCopyId}
      onDelete={handleDelete}
    />
  </div>
</div>
