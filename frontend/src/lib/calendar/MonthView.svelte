<script lang="ts">
  import { getMonthDays, isToday, isCurrentMonth, formatDateKey, days } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { year, month, events = [], onEventClick, onDateClick, onDrop }: {
    year: number; month: number;
    events?: CEvent[];
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string) => void;
    onDrop?: (eventId: string, newDate: string) => void;
  } = $props();

  let eventsByDate = $derived.by(() => {
    const m = new Map<string, CEvent[]>();
    for (const e of events) {
      const existing = m.get(e.date) || [];
      existing.push(e);
      m.set(e.date, existing);
    }
    return m;
  });

  let calDays = $derived(getMonthDays(year, month));

  function handleDragStart(e: DragEvent, eventId: string) {
    e.dataTransfer?.setData("text/plain", eventId);
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function handleDrop(e: DragEvent, dateStr: string) {
    e.preventDefault();
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr);
  }
</script>

<div class="month-calendar bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
  <div class="grid grid-cols-7 text-center text-xs text-[#6b7280] py-2.5 border-b border-[#1e2435]">
    {#each days as d}<span>{d}</span>{/each}
  </div>
  <div class="grid grid-cols-7">
    {#each calDays as date}
      {@const key = formatDateKey(date)}
      {@const dayEvents = eventsByDate.get(key) || []}
      <div
        ondragover={handleDragOver}
        ondrop={(e) => handleDrop(e, key)}
        onclick={() => onDateClick?.(key)}
        class="min-h-24 p-1.5 border-b border-r border-[#1e2435] transition-colors hover:bg-[#1a1f2e] cursor-pointer"
        class:opacity-30={!isCurrentMonth(date, year, month)}
      >
        <span class="text-xs w-6 h-6 flex items-center justify-center rounded-full mb-0.5"
          class:bg-indigo-600!={isToday(date)}
          class:text-white!={isToday(date)}
          class:text-[#6b7280]={!isToday(date)}
        >{date.getDate()}</span>
        <div class="space-y-0.5">
          {#each dayEvents.slice(0, 3) as event}
            <div
              draggable="true"
              ondragstart={(e) => handleDragStart(e, event.id)}
              onclick={(e) => { e.stopPropagation(); onEventClick?.(event.id); }}
            >
              <CalendarEvent {event} compact />
            </div>
          {/each}
          {#if dayEvents.length > 3}
            <div class="text-[10px] text-[#6b7280] px-1">+{dayEvents.length - 3} more</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
