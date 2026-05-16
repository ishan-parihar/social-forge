<script lang="ts">
  let { selected, onChange }: { selected: number; onChange: (days: number) => void } = $props();
</script>

<div class="inline-flex bg-[#131720] rounded-lg p-1 border border-[#1e2435]">
  {#each [7, 30, 90] as days}
    <button
      onclick={() => onChange(days)}
      class="px-4 py-1.5 rounded-md text-sm font-medium transition-colors {selected === days ? 'bg-indigo-600 text-white' : 'text-[#d1d5db] hover:text-white hover:bg-[#1e2435]'}"
    >
      {days}d
    </button>
  {/each}
</div>
