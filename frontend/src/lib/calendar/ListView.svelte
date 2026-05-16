<script lang="ts">
  import Badge from "$lib/ui/Badge.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { events = [], onEventClick, page = 1, totalPages = 1, totalItems = 0, onPageChange }: {
    events?: CEvent[];
    onEventClick?: (id: string) => void;
    page?: number;
    totalPages?: number;
    totalItems?: number;
    onPageChange?: (p: number) => void;
  } = $props();

  let sorted = $derived([...events].sort((a, b) =>
    a.date.localeCompare(b.date) || (a.time || "00:00").localeCompare(b.time || "00:00")
  ));
</script>

<div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
  <div class="text-xs text-[#6b7280] px-4 py-2 border-b border-[#1e2435]">Upcoming posts</div>
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

  {#if totalPages > 1}
    <div class="flex items-center justify-between px-4 py-3 border-t border-[#1e2435]">
      <span class="text-sm text-[#6b7280]">
        Showing {(page - 1) * 20 + 1}–{Math.min(page * 20, totalItems)} of {totalItems}
      </span>
      <div class="flex gap-2">
        <button
          onclick={() => onPageChange?.(page - 1)}
          disabled={page <= 1}
          class="px-3 py-1 text-sm rounded bg-[#1e2435] text-[#d1d5db] disabled:opacity-50 hover:bg-[#2a3045] transition-colors"
        >← Previous</button>
        <button
          onclick={() => onPageChange?.(page + 1)}
          disabled={page >= totalPages}
          class="px-3 py-1 text-sm rounded bg-[#1e2435] text-[#d1d5db] disabled:opacity-50 hover:bg-[#2a3045] transition-colors"
        >Next →</button>
      </div>
    </div>
  {/if}
</div>
