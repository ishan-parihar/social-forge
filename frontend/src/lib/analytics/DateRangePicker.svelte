<script lang="ts">
  let { selected, onChange }: { selected: number; onChange: (days: number) => void } = $props();
</script>

<div class="inline-flex bg-surface rounded-lg p-1 border border-line" role="group" aria-label="Date range">
  {#each [7, 30, 90] as days (days)}
    <button
      onclick={() => onChange(days)}
      class="px-4 py-1.5 rounded-md text-sm font-medium transition-colors {selected === days ? 'bg-accent-fill text-accent-fg' : 'text-content-secondary hover:text-content hover:bg-surface-hover'}"
      aria-pressed={selected === days}
      aria-label="Last {days} days"
    >
      {days}d
    </button>
  {/each}
</div>

