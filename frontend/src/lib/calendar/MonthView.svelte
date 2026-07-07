<script lang="ts">
  import { getMonthDays, isToday, isCurrentMonth, isPast, formatDateKey, days } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { year, month, events = [], selected = new Set(), onEventClick, onDateClick, onDrop, onDuplicate, onStats, onDelete, onToggleSelect }: {
    year: number; month: number;
    events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string) => void;
    onDrop?: (eventId: string, newDate: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
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
    if (!e.dataTransfer) return;
    e.dataTransfer.setData("text/plain", eventId);
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

<div class="month-calendar bg-surface border border-line rounded-xl overflow-hidden">
  <div class="grid grid-cols-7 text-center text-xs text-muted py-2.5 border-b border-line">
    {#each days as d}<span>{d}</span>{/each}
  </div>
  <div class="grid grid-cols-7">
    {#each calDays as date (formatDateKey(date))}
      {@const key = formatDateKey(date)}
      {@const dayEvents = eventsByDate.get(key) || []}
      {@const past = isPast(date)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        ondragover={(e) => { if (!past) handleDragOver(e); }}
        ondrop={(e) => { if (!past) handleDrop(e, key); }}
        onclick={() => onDateClick?.(key)}
        role="gridcell"
        tabindex="-1"
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onDateClick?.(key); } }}
        class="min-h-24 p-1.5 border-b border-r border-line transition-colors hover:bg-surface-hover cursor-pointer {past ? 'opacity-40' : ''}"
        class:opacity-30={!isCurrentMonth(date, year, month)}
        class:cursor-not-allowed={past}
      >
        <span class="text-xs w-6 h-6 flex items-center justify-center rounded-full mb-0.5"
          class:bg-brand-500!={isToday(date)}
          class:text-white!={isToday(date)}
          class:text-muted={!isToday(date)}
        >{date.getDate()}</span>
        <div class="space-y-0.5">
          {#each dayEvents.slice(0, 3) as event (event.id)}
            <div
              draggable={event.state !== 'published' && !past}
              ondragstart={(e) => handleDragStart(e, event.id)}
              onclick={(e) => { e.stopPropagation(); onEventClick?.(event.id); }}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); onEventClick?.(event.id); } }}
              role="button"
              tabindex="-1"
              class="flex items-center gap-1 {event.state === 'published' || past ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
            >
              {#if onToggleSelect}
                <input type="checkbox" checked={selected.has(event.id)} onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0 w-3 h-3" />
              {/if}
              <CalendarEvent {event} compact {onDuplicate} {onStats} {onDelete} />
            </div>
          {/each}
          {#if dayEvents.length > 3}
            <div class="text-[10px] text-muted px-1">+{dayEvents.length - 3} more</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>
