<script lang="ts">
  import Dropdown from "$lib/ui/Dropdown.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import type { Integration } from "$lib/api/integrations";

  let { integration, onDisconnect, onRefresh }: {
    integration: Integration;
    onDisconnect?: (id: string) => void;
    onRefresh?: (id: string) => void;
  } = $props();

  const menuItems = $derived([
    ...(integration.refresh_needed
      ? [{ label: "Refresh Token", onclick: () => onRefresh?.(integration.id), variant: "default" as const }]
      : []),
    { label: "Disconnect", onclick: () => onDisconnect?.(integration.id), variant: "danger" as const },
  ]);
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
    <Dropdown items={menuItems} align="right">
      <span class="opacity-0 group-hover:opacity-100 p-1 text-[#6b7280] hover:text-white transition-all" aria-label="Channel actions" role="button">⋮</span>
    </Dropdown>
  </div>
</div>
