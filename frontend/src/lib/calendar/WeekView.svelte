<script lang="ts">
  import { buildWeekDays, getDayHours, isPast, isToday } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";
  import { timezone } from "$lib/stores/timezone.svelte";
  import { makeTouchDragHandler, type TouchDropTarget } from "./touch-drag";

  let { referenceDate, events = [], selected = new Set(), onEventClick, onDateClick, onDrop, onDuplicate, onStats, onDelete, onToggleSelect }: {
    referenceDate: Date; events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string) => void;
    onDrop?: (eventId: string, newDate: string, newHour?: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
  } = $props();

  let weekDays = $derived(buildWeekDays(referenceDate, events));
  let hours = $derived(getDayHours());

  // Show the user's selected timezone abbreviation in the corner instead
  // of a hard-coded "GMT" — matches what the column times actually render in.
  // Intl can resolve the abbreviation (e.g. "EST", "PST", "IST") for the
  // current timezone, falling back to the IANA name on edge cases.
  let tzLabel = $derived.by(() => {
    try {
      const parts = new Intl.DateTimeFormat('en-US', {
        timeZone: timezone.value,
        timeZoneName: 'short',
      }).formatToParts(new Date());
      const tzPart = parts.find(p => p.type === 'timeZoneName');
      return tzPart?.value || timezone.value;
    } catch {
      return 'GMT';
    }
  });

  let eventsByDayHour = $derived.by(() => {
    const map = new Map<string, CEvent[]>();
    for (const wd of weekDays) {
      for (const e of wd.events) {
        const hour = (e.time || "00:00").slice(0, 2);
        const key = `${wd.dateStr}-${hour}`;
        const list = map.get(key) || [];
        list.push(e);
        map.set(key, list);
      }
    }
    return map;
  });

  function handleDrop(e: DragEvent, dateStr: string, hour: string) {
    e.preventDefault();
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr, hour);
  }

  function handleDragStart(e: DragEvent, eventId: string) {
    e.dataTransfer?.setData("text/plain", eventId);
    e.dataTransfer!.effectAllowed = "move";
  }

  function handleKeyDown(e: KeyboardEvent, dateStr: string) {
    if ((e.key === 'Enter' || e.key === ' ') && onDrop) {
      e.preventDefault();
    }
  }

  // Dragover highlight state: track which cell is currently being hovered
  // so we can show a visual affordance (ring) on the drop target.
  let dragOverKey = $state<string | null>(null);
  function handleDragEnter(dateStr: string, hour: string) {
    dragOverKey = `${dateStr}-${hour}`;
  }
  function handleDragLeave(dateStr: string, hour: string) {
    // Only clear if we're leaving the exact cell that was highlighted.
    if (dragOverKey === `${dateStr}-${hour}`) {
      dragOverKey = null;
    }
  }
  function handleDragEnd() {
    dragOverKey = null;
  }

  /**
   * Phase v21: a cell is "past" (and thus a non-drop target) if either:
   *   - the day is before today (whole day is past), OR
   *   - the day is today AND the hour has already ended.
   * Past cells get opacity-40 + cursor-not-allowed and drop is suppressed.
   */
  function isCellPast(date: Date, hour: string): boolean {
    if (isPast(date)) return true;
    if (isToday(date)) {
      const now = new Date();
      const cellHour = parseInt(hour.slice(0, 2), 10);
      return cellHour < now.getHours();
    }
    return false;
  }

  // Phase 10: touch-device DnD fallback.
  // Uses a long-press + elementFromPoint approach so drag-to-reschedule
  // works on mobile browsers where HTML5 DnD doesn't fire.
  let touchHighlight = $state<TouchDropTarget | null>(null);
  const touchDrag = makeTouchDragHandler({
    getEventId: (e: TouchEvent) => {
      const target = e.target as HTMLElement;
      return target.closest('[data-event-id]')?.getAttribute('data-event-id') || null;
    },
    getDropTarget: (x: number, y: number) => {
      const el = document.elementFromPoint(x, y);
      const cell = el?.closest('[data-drop-date]') as HTMLElement | null;
      if (!cell) return null;
      return {
        date: cell.dataset.dropDate!,
        hour: cell.dataset.dropHour,
      };
    },
    onDrop: (eventId: string, date: string, hour?: string) => {
      onDrop?.(eventId, date, hour);
    },
    onHighlight: (target: TouchDropTarget | null) => {
      touchHighlight = target;
      if (target) {
        dragOverKey = target.hour ? `${target.date}-${target.hour}` : target.date;
      } else {
        dragOverKey = null;
      }
    },
  });
</script>

<svelte:window ondragend={handleDragEnd} />

<div class="week-calendar bg-surface border border-line rounded-xl overflow-hidden" ontouchstart={touchDrag.onTouchStart} ontouchmove={touchDrag.onTouchMove} ontouchend={touchDrag.onTouchEnd}>
  <div class="grid grid-cols-8 text-center border-b border-line">
    <div class="py-2 text-xs text-muted border-r border-line">{tzLabel}</div>
    {#each weekDays as wd (wd.dateStr)}
      <div class="py-2 text-xs {wd.isToday ? 'text-brand-400' : 'text-muted'} relative">
        <div>{wd.date.toLocaleDateString("en-US", { weekday: "short" })}</div>
        <div class="font-semibold">{wd.date.getDate()}</div>
        {#if wd.isToday}
          <!-- Phase 7: today indicator — a small dot under the date -->
          <div class="absolute bottom-0 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-brand-400"></div>
        {/if}
      </div>
    {/each}
  </div>
  <div class="overflow-y-auto max-h-[600px]">
    {#each hours as hour (hour)}
      <div class="grid grid-cols-8 border-b border-line min-h-[48px]">
        <div class="text-xs text-muted px-2 py-1 border-r border-line">{hour}</div>
        {#each weekDays as wd (wd.dateStr)}
          {@const cellKey = `${wd.dateStr}-${hour}`}
          {@const isDragOver = dragOverKey === cellKey}
          {@const past = isCellPast(wd.date, hour)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            data-drop-date={wd.dateStr}
            data-drop-hour={hour.slice(0, 2)}
            class="relative px-1 py-1 border-r border-line min-h-[48px] cursor-pointer hover:bg-surface-hover transition-colors
              {isDragOver ? 'ring-2 ring-brand-500 ring-inset bg-brand-500/5' : ''}
              {wd.isToday ? 'bg-brand-500/5' : ''}
              {past ? 'opacity-40 cursor-not-allowed' : ''}"
            ondragover={(e) => { if (!past) { e.preventDefault(); handleDragEnter(wd.dateStr, hour); } }}
            ondragleave={() => handleDragLeave(wd.dateStr, hour)}
            ondrop={(e) => { if (!past) handleDrop(e, wd.dateStr, hour); }}
            onclick={() => { if (!past) onDateClick?.(wd.dateStr); }}
            role="gridcell"
            tabindex="-1"
            onkeydown={(e) => handleKeyDown(e, wd.dateStr)}
          >
            {#each (eventsByDayHour.get(cellKey) || []) as event (event.id)}
              <div
                data-event-id={event.id}
                class="flex items-center gap-1 {event.state === 'published' || past ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
                draggable={event.state !== 'published' && !past}
                ondragstart={(e) => handleDragStart(e, event.id)}
                onclick={() => onEventClick?.(event.id)}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onEventClick?.(event.id); }}
                role="button"
                tabindex="-1"
              >
                {#if onToggleSelect}
                  <input type="checkbox" checked={selected.has(event.id)} onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0 w-3 h-3" />
                {/if}
                <CalendarEvent {event} {onDuplicate} {onStats} {onDelete} />
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>
