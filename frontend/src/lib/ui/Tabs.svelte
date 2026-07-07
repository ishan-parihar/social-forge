<script lang="ts">
  // v22 Phase 3: Tabs primitive — segmented control with active state.
  // Replaces ad-hoc tab strips in settings/posts/analytics.
  let {
    tabs,
    value = $bindable(),
  }: {
    tabs: { id: string; label: string }[];
    value: string;
  } = $props();
</script>

<div class="inline-flex bg-surface-hover rounded-lg p-0.5 border border-line">
  {#each tabs as tab (tab.id)}
    <button
      onclick={() => (value = tab.id)}
      class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors {value === tab.id ? 'bg-brand-500 text-white' : 'text-muted hover:text-content'}"
    >
      {tab.label}
    </button>
  {/each}
</div>
