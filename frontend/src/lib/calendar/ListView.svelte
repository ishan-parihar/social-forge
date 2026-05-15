<script lang="ts">
  import Badge from "$lib/ui/Badge.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { events = [], onEventClick }: {
    events?: CEvent[];
    onEventClick?: (id: string) => void;
  } = $props();

  let sorted = $derived([...events].sort((a, b) =>
    a.date.localeCompare(b.date) || (a.time || "00:00").localeCompare(b.time || "00:00")
  ));
</script>

<div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
  <div class="text-xs text-[#6b7280] px-4 py-2 border-b border-[#1e2435]">{sorted.length} upcoming posts</div>
  {#if sorted.length === 0}
    <div class="text-center py-12 text-sm text-[#6b7280]">No posts scheduled</div>
  {:else}
    {#each sorted as event}
      <button onclick={() => onEventClick?.(event.id)} class="w-full flex items-center gap-4 px-4 py-3 border-b border-[#1e2435] hover:bg-[#1a1f2e] transition-colors text-left">
        <div class="text-xs text-[#6b7280] w-24 shrink-0">
          {event.date}
          {#if event.time}<br><span class="text-indigo-400">{event.time}</span>{/if}
        </div>
        <div class="flex-1 min-w-0">
          <div class="text-sm truncate">{event.title}</div>
          <div class="text-xs text-[#6b7280]">{event.integrationName}</div>
        </div>
        <Badge state={event.state} />
        {#if event.error}
          <span class="text-xs text-red-400" title={event.error}>⚠</span>
        {/if}
      </button>
    {/each}
  {/if}
</div>
