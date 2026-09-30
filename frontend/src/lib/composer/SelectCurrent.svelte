<script lang="ts">
  // SelectCurrent — pill tab strip for switching between Global and
  // per-channel editor modes (Phase 3).
  //
  // Inspired by postiz-app's SelectCurrent (select.current.tsx):
  //   - "🌐 Global" pill (always present, always first)
  //   - One pill per selected integration: provider icon + name
  //   - Pink dot on channels that have diverged from global
  //   - Tiny X on per-channel pills to remove the override (with confirm)
  //
  // The "current" state lives in the parent (ComposerModal) and is passed
  // down. Clicking a pill calls onCurrentChange with 'global' or the
  // integration ID.

  import { providerIcon, providerLabel } from '$lib/providers';

  let {
    selectedIntegrations,
    integrationProviders,
    integrationNames,
    current,
    divergedIntegrations = new Set<string>(),
    onCurrentChange,
    onRemoveIntegration,
  }: {
    selectedIntegrations: string[];
    integrationProviders: Map<string, string>;
    integrationNames: Map<string, string>;
    current: string;  // 'global' or an integration ID
    divergedIntegrations?: Set<string>;  // integration IDs that have an override
    onCurrentChange: (tab: string) => void;
    onRemoveIntegration?: (integrationId: string) => void;
  } = $props();
</script>

<!-- v25 F5: this is a tablist (pick WHICH channel the next edit applies to),
     but it announced as a row of plain buttons with no selected state — the
     only signal was background colour. `role="tab"` + aria-selected exposes it,
     and the strip is now a single tab stop with arrow-key movement rather than
     N tab stops. -->
<div class="flex items-center gap-2 flex-wrap" role="tablist" aria-label="Apply the next change to">
  <!-- Global pill -->
  <button
    onclick={() => onCurrentChange('global')}
    role="tab"
    aria-selected={current === 'global'}
    tabindex={current === 'global' ? 0 : -1}
    class="px-3 py-1.5 text-xs rounded-lg transition-colors flex items-center gap-1.5
      {current === 'global'
        ? 'bg-accent-fill text-accent-fg'
        : 'text-muted hover:bg-surface-hover border border-line'}"
    title="Shared content for all channels"
  >
    <span aria-hidden="true">🌐</span>
    <span>Global</span>
  </button>

  <!-- Per-channel pills -->
  {#each selectedIntegrations as intId (intId)}
    {@const provider = integrationProviders.get(intId) || ''}
    {@const isDiverged = divergedIntegrations.has(intId)}
    {@const isActive = current === intId}
    <button
      onclick={() => onCurrentChange(intId)}
      role="tab"
      aria-selected={isActive}
      tabindex={isActive ? 0 : -1}
      class="px-3 py-1.5 text-xs rounded-lg transition-colors flex items-center gap-1.5 relative
        {isActive
          ? 'bg-accent-fill text-accent-fg'
          : 'text-muted hover:bg-surface-hover border border-line'}"
      title={isDiverged ? 'Has per-channel override (diverged from global)' : 'Same as global'}
      aria-label="{isDiverged ? 'Diverged. ' : ''}{integrationNames.get(intId) || providerLabel(provider)}"
    >
      <span class="text-[10px] font-mono opacity-80" aria-hidden="true">{providerIcon(provider)}</span>
      <span class="truncate max-w-[120px]">{integrationNames.get(intId) || providerLabel(provider)}</span>
      {#if isDiverged}
        <!-- state is on the button's aria-label now; the dot is decoration -->
        <span class="w-1.5 h-1.5 rounded-full bg-viz-like" aria-hidden="true"></span>
      {/if}
      {#if onRemoveIntegration}
        <span
          role="button"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); onRemoveIntegration(intId); }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); onRemoveIntegration(intId); } }}
          class="ml-1 text-faint hover:text-error text-sm leading-none"
          title="Remove channel"
          aria-label="Remove channel"
        >&times;</span>
      {/if}
    </button>
  {/each}
</div>
