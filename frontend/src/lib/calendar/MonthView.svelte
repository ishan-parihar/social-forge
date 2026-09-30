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
    e.dataTransfer.effectAllowed = "move";
  }

  // v25 F3: track the hovered day so the drop target lights up. WeekView and
  // DayView already had this; the month grid silently accepted drops with no
  // feedback, so the user could not tell whether a cell was a valid target.
  let dragOverDate = $state<string | null>(null);

  function handleDragOver(e: DragEvent, dateStr: string) {
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = "move";
    dragOverDate = dateStr;
  }

  function handleDragLeave(dateStr: string) {
    // dragleave fires when moving between children of the same cell, so only
    // clear when the cell we are leaving is the one that lit up.
    if (dragOverDate === dateStr) dragOverDate = null;
  }

  function handleDrop(e: DragEvent, dateStr: string) {
    e.preventDefault();
    dragOverDate = null;
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr);
  }
</script>

<svelte:window ondragend={() => (dragOverDate = null)} />

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
        ondragover={(e) => { if (!past) handleDragOver(e, key); }}
        ondragleave={() => handleDragLeave(key)}
        ondrop={(e) => { if (!past) handleDrop(e, key); }}
        onclick={() => onDateClick?.(key)}
        role="gridcell"
        tabindex="-1"
        onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onDateClick?.(key); } }}
        class="min-h-24 p-1.5 border-b border-r border-line transition-colors hover:bg-surface-hover cursor-pointer relative {past ? 'opacity-40' : ''} {dragOverDate === key ? 'ring-2 ring-accent ring-inset bg-accent-fill/5' : ''}"
        class:opacity-30={!isCurrentMonth(date, year, month)}
        class:cursor-not-allowed={past}
      >
        <!-- v25 F3: today gets a left accent rail on the cell itself, so the
             current day is findable while scanning a 5-row grid (the day-number
             pill alone reads as "just another highlighted number"). -->
        {#if isToday(date)}
          <span class="absolute inset-y-0 left-0 w-0.5 bg-accent-fill" aria-hidden="true"></span>
        {/if}
        <span class="text-xs w-6 h-6 flex items-center justify-center rounded-full mb-0.5 {isToday(date) ? 'bg-accent-fill text-accent-fg' : 'text-muted'}"
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
              title={event.state === 'published' || past ? 'Published posts cannot be dragged' : 'Drag to reschedule'}
              class="group/chip flex items-center gap-1 {event.state === 'published' || past ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
            >
              {#if onToggleSelect}
                <input type="checkbox" checked={selected.has(event.id)} onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0 w-3 h-3" />
              {/if}
              <!-- v25 F3: drag affordance. A six-dot grip that only appears on
                   hover tells the user which chips are movable; without it the
                   whole cell reads as click-to-open. -->
              {#if event.state !== 'published' && !past}
                <span class="hidden group-hover/chip:inline text-faint leading-none select-none" aria-hidden="true">⠿</span>
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
