<script lang="ts">
  let { postsByDay }: { postsByDay: Array<{ date: string; count: number }> } = $props();

  let maxCount = $derived(Math.max(...postsByDay.map(d => d.count), 1));
</script>

<div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4">
  <h3 class="text-sm font-medium text-[#e8edf5] mb-4">Posts Over Time</h3>
  {#if postsByDay.length === 0}
    <p class="text-[#d1d5db] text-sm py-8 text-center">No data for this period</p>
  {:else}
    <div class="flex items-end gap-1 h-40">
      {#each postsByDay as day}
        <div class="flex-1 flex flex-col items-center justify-end h-full">
          <div
            class="w-full bg-indigo-500/80 rounded-t hover:bg-indigo-400 transition-colors min-h-[4px]"
            style="height: {(day.count / maxCount) * 100}%"
            title="{day.date}: {day.count} posts"
          ></div>
        </div>
      {/each}
    </div>
    <div class="flex gap-1 mt-2">
      {#each postsByDay as day}
        <div class="flex-1 text-center">
          <span class="text-[10px] text-[#6b7280]">{day.date.slice(5)}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
