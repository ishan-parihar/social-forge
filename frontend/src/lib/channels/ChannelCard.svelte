<script lang="ts">
  import ProviderIcon from "./ProviderIcon.svelte";
  import ChannelContextMenu from "./ChannelContextMenu.svelte";
  import Icon from "$lib/ui/Icon.svelte";
  import { integrationsApi, type Integration, type TimeslotEntry } from "$lib/api/integrations";
  import { getAuthType } from "./auth-types";
  import { channelStatus } from "./channel-status";
  import { toast } from "$lib/stores/toast";

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

  // v25 F4: one source of truth for the status chip, shared with the page-level
  // summary and any future status filter. See ./channel-status.ts for the
  // precedence rules.
  let status = $derived(channelStatus(integration));

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
    if (m === "cookie") return "text-warning border-warning/30 bg-warning/10";
    if (m === "pat") return "text-success border-success/30 bg-success/10";
    if (m === "api_key") return "text-warning border-warning/30 bg-warning/10";
    switch (authType) {
      case "api_key": return "text-warning border-warning/30 bg-warning/10";
      case "web3": return "text-viz-view border-hue-violet/30 bg-hue-violet/10";
      case "extension": return "text-viz-click border-hue-cyan/30 bg-hue-cyan/10";
      default: return "";
    }
  });

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
        if (r.error) toast(`Toggle disable failed: ${r.error}`, "error");
      } catch (e) {
        toast(`Toggle disable error: ${e instanceof Error ? e.message : "unknown"}`, "error");
      }
    }
  }

  function handleDelete() {
    onDisconnect?.(integration.id);
  }
</script>

<!--
  v25 F4: the row is an <li> because the parent renders these inside a group
  list. The status is folded into the row's aria-label so a screen reader
  hears the connection state together with the account name, instead of
  having to find and read the chip separately.
-->
<li
  class="flex items-center gap-3 px-3 py-2.5 hover:bg-surface-hover rounded-lg transition-colors group"
  aria-label="{integration.profile_name || integration.provider_name}, {status.label}"
>
  {#if integration.profile_picture}
    <img src={integration.profile_picture} alt="" class="w-8 h-8 rounded-full object-cover flex-shrink-0" />
  {:else}
    <ProviderIcon provider={integration.provider_identifier} size="sm" />
  {/if}

  <div class="flex-1 min-w-0">
    <div class="text-sm truncate flex items-center gap-2">
      {integration.profile_name || integration.provider_name}
      {#if integration.root_internal_id}
        <span class="text-[10px] px-1.5 py-0.5 rounded border text-accent border-accent/30 bg-accent-fill/10">Page</span>
      {/if}
      {#if authTypeLabel}
        <span class="text-[10px] px-1.5 py-0.5 rounded border {authTypeColor}">{authTypeLabel}</span>
      {/if}
    </div>
    <div class="text-xs text-muted truncate">
      {#if integration.profile_url}
        {integration.profile_url}
      {:else if integration.internal_id}
        @{integration.internal_id}
      {:else}
        {integration.provider_name}
      {/if}
    </div>

    <!--
      F4: the reason a channel is unhealthy is stated in words, not implied by
      a color. It only renders when there is something to fix; a healthy channel
      does not need a reassuring sentence on every row.
    -->
    {#if status.actionable}
      <p class="text-xs text-faint truncate mt-0.5">{status.hint}</p>
    {/if}
  </div>

  <div class="shrink-0 flex items-center gap-2">
    <!--
      F4 replaces the 2px dot + `title`. `title` only appears on hover, is
      unreachable by touch, and gave no hint about the remedy. This is a real
      chip: colored icon + word, with the full explanation as the accessible
      title on top of the visible label.
    -->
    <span
      role="status"
      title={status.hint}
      class="hidden sm:inline-flex items-center gap-1 px-1.5 py-0.5 rounded border text-[10px] font-medium {status.chip}"
    >
      <Icon name={status.icon} class="w-3 h-3" />
      {status.label}
    </span>
    <!-- Below sm the chip is dropped for space, so the icon carries the state
         and the row's aria-label keeps it available to a screen reader. -->
    <Icon name={status.icon} class="sm:hidden w-3.5 h-3.5 {status.textClass}" />

    <ChannelContextMenu
      integrationId={integration.id}
      integrationName={integration.profile_name || integration.provider_name}
      currentTimeslots={currentTimeslots}
      disabled={integration.disabled}
      isRefreshing={isRefreshing}
      onRefreshToken={status.actionable || authType === "oauth" ? () => onRefresh?.(integration.id) : undefined}
      onReconnect={onReconnect ? () => onReconnect(integration.id) : undefined}
      onRename={undefined}
      onToggleDisable={handleToggleDisable}
      onCopyId={handleCopyId}
      onDelete={handleDelete}
    />
  </div>
</li>
