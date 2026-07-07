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

<div class="flex items-center gap-2 flex-wrap">
  <!-- Global pill -->
  <button
    onclick={() => onCurrentChange('global')}
    class="px-3 py-1.5 text-xs rounded-lg transition-colors flex items-center gap-1.5
      {current === 'global'
        ? 'bg-indigo-600 text-white'
        : 'text-muted hover:bg-surface-hover border border-line'}"
    title="Shared content for all channels"
  >
    <span>🌐</span>
    <span>Global</span>
  </button>

  <!-- Per-channel pills -->
  {#each selectedIntegrations as intId (intId)}
    {@const provider = integrationProviders.get(intId) || ''}
    {@const isDiverged = divergedIntegrations.has(intId)}
    {@const isActive = current === intId}
    <button
      onclick={() => onCurrentChange(intId)}
      class="px-3 py-1.5 text-xs rounded-lg transition-colors flex items-center gap-1.5 relative
        {isActive
          ? 'bg-indigo-600 text-white'
          : 'text-muted hover:bg-surface-hover border border-line'}"
      title={isDiverged ? 'Has per-channel override (diverged from global)' : 'Same as global'}
    >
      <span class="text-[10px] font-mono opacity-80">{providerIcon(provider)}</span>
      <span class="truncate max-w-[120px]">{integrationNames.get(intId) || providerLabel(provider)}</span>
      {#if isDiverged}
        <span class="w-1.5 h-1.5 rounded-full bg-pink-400" title="Diverged from global"></span>
      {/if}
      {#if onRemoveIntegration}
        <span
          role="button"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); onRemoveIntegration(intId); }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); onRemoveIntegration(intId); } }}
          class="ml-1 text-muted-dark hover:text-red-400 text-sm leading-none"
          title="Remove channel"
          aria-label="Remove channel"
        >&times;</span>
      {/if}
    </button>
  {/each}
</div>
