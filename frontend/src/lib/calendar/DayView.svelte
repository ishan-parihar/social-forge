<script lang="ts">
  import { formatDateKey, getDayHours, isToday } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";
  import type { Integration } from "$lib/api/integrations";
  import { makeTouchDragHandler, type TouchDropTarget } from "./touch-drag";
  import { timezone } from "$lib/stores/timezone.svelte";

  let { date, events = [], selected = new Set(), onEventClick, onDateClick, onDrop, onDuplicate, onStats, onDelete, onToggleSelect, integrations = [], integrationFilter = null }: {
    date: Date; events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string, hour?: string) => void;
    onDrop?: (eventId: string, newDate: string, newHour?: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
    integrations?: Integration[];
    /**
     * The channel the view is filtered to, or null for "all".
     *
     * The GHOST SLOTS have to honour it. The real events do - the
     * server filters them - but the ghosts were built from every
     * integration, so a view filtered to LinkedIn still showed
     * "X (Twitter) usually posts at this time" in every row. The
     * founder had explicitly excluded those channels and the grid
     * showed them anyway.
     */
    integrationFilter?: string | null;
  } = $props();

  let key = $derived(formatDateKey(date));
  let dayEvents = $derived(events.filter(e => e.date === key));
  let hours = $derived(getDayHours());

  // Phase 2: ghost slots - per-channel posting time presets that have
  // no real post at that time. These show as dashed-border empty drop
  // targets so the user sees when their channel "usually" posts.
  // Each ghost slot is { hour: string, integrationName: string }.
  let ghostSlots = $derived.by(() => {
    const ghosts: { hour: string; integrationName: string; integrationId: string }[] = [];
    for (const int of integrations) {
      // The CHANNEL FILTER applies to the ghosts too.
      if (integrationFilter && int.id !== integrationFilter) continue;
      if (!int.posting_times || int.posting_times.length === 0) continue;
      for (const pt of int.posting_times) {
        const hourStr = String(Math.floor(pt.time / 60)).padStart(2, '0');
        const minuteStr = String(pt.time % 60).padStart(2, '0');
        const timeStr = `${hourStr}:${minuteStr}`;
        // Only show ghost if no real event exists at this hour for this date.
        const hasReal = dayEvents.some(e => (e.time || '').slice(0, 2) === hourStr);
        if (!hasReal) {
          ghosts.push({ hour: `${hourStr}:00`, integrationName: int.provider_name, integrationId: int.id });
        }
      }
    }
    return ghosts;
  });

  // Group ghost slots by hour for rendering alongside real events.
  let ghostsByHour = $derived.by(() => {
    const map = new Map<string, typeof ghostSlots>();
    for (const g of ghostSlots) {
      const hourKey = g.hour.slice(0, 2);
      const list = map.get(hourKey) || [];
      list.push(g);
      map.set(hourKey, list);
    }
    return map;
  });

  // State summary for the day - shows at a glance how many posts are
  // scheduled vs published vs failed vs draft on this day.
  let stateSummary = $derived.by(() => {
    const s = { draft: 0, queued: 0, published: 0, error: 0 };
    for (const e of dayEvents) {
      if (e.state in s) s[e.state as keyof typeof s]++;
    }
    return s;
  });

  let eventsByHour = $derived.by(() => {
    const map = new Map<string, CEvent[]>();
    for (const e of dayEvents) {
      const hour = (e.time || "00:00").slice(0, 2);
      const list = map.get(hour) || [];
      list.push(e);
      map.set(hour, list);
    }
    return map;
  });

  // Phase 1: hour-precision drop + dragover highlight.
  let dragOverHour = $state<string | null>(null);

  function handleDrop(e: DragEvent, dateStr: string, hour: string) {
    e.preventDefault();
    dragOverHour = null;
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr, hour);
  }

  function handleDragStart(e: DragEvent, eventId: string) {
    e.dataTransfer?.setData("text/plain", eventId);
    e.dataTransfer!.effectAllowed = "move";
  }

  // Phase 10: touch-device DnD fallback.
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
      if (target) {
        dragOverHour = target.hour || null;
      } else {
        dragOverHour = null;
      }
    },
  });
</script>

<div role="group" class="bg-surface border border-line rounded-xl overflow-hidden flex flex-col max-h-full" ontouchstart={touchDrag.onTouchStart} ontouchmove={touchDrag.onTouchMove} ontouchend={touchDrag.onTouchEnd}>
  <div class="py-3 border-b border-line px-4">
    <div class="text-lg font-semibold text-center">
      {date.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" })}
      <!-- v25 F3: name the day when it IS today, so the day view carries the
           same orientation cue the week view gets from its today column. -->
      {#if isToday(date, timezone.value)}
        <span class="ml-2 align-middle text-[10px] font-medium uppercase tracking-wider px-1.5 py-0.5 rounded bg-accent-fill/10 text-accent-strong">Today</span>
      {/if}
    </div>
    <div class="flex items-center justify-center gap-3 mt-1 text-xs flex-wrap">
      <span class="text-muted">{dayEvents.length} total</span>
      {#if stateSummary.queued > 0}
        <span class="text-accent">{stateSummary.queued} scheduled</span>
      {/if}
      {#if stateSummary.published > 0}
        <span class="text-success">{stateSummary.published} published</span>
      {/if}
      {#if stateSummary.draft > 0}
        <span class="text-muted">{stateSummary.draft} drafts</span>
      {/if}
      {#if stateSummary.error > 0}
        <span class="text-error">{stateSummary.error} failed</span>
      {/if}
    </div>
  </div>
  <!-- Same shape as WeekView: the scroller fills the height the page
       gives the card instead of a fixed 700px, so all of a day's posts
       are reachable by scroll at any window height. `gridEl` is unused
       here (the day grid is single-column, no cell focus needed). -->
  <div class="overflow-y-auto flex-1 min-h-0 overscroll-contain">
    {#each hours as hour (hour)}
      {@const hourStr = hour.slice(0, 2)}
      {@const isDragOver = dragOverHour === hourStr}
      <div class="flex border-b border-line min-h-[56px]">
        <div class="w-16 text-xs text-muted px-2 py-1 border-r border-line shrink-0">{hour}</div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          data-drop-date={key}
          data-drop-hour={hourStr}
          class="flex-1 px-2 py-1 space-y-0.5 cursor-pointer hover:bg-surface-hover transition-colors
            {isDragOver ? 'ring-2 ring-accent ring-inset bg-accent-fill/5' : ''}"
          onclick={() => onDateClick?.(key)}
          onkeydown={(e) => { if (e.key === 'Enter' && e.target === e.currentTarget) onDateClick?.(key); }}
          ondragover={(e) => { e.preventDefault(); dragOverHour = hourStr; }}
          ondragleave={() => { if (dragOverHour === hourStr) dragOverHour = null; }}
          ondrop={(e) => handleDrop(e, key, hourStr)}
          role="gridcell"
          tabindex="-1"
        >
          {#each (eventsByHour.get(hourStr) || []) as event (event.id)}
            <div
              data-event-id={event.id}
              class="group/chip flex items-center gap-1 flex-1 min-w-0 {event.state === 'published' ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
              draggable={event.state !== 'published'}
              ondragstart={(e) => handleDragStart(e, event.id)}
              onclick={() => onEventClick?.(event.id)}
              onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') onEventClick?.(event.id); }}
              role="button"
              tabindex="-1"
              title={event.state === 'published' ? 'Published posts cannot be dragged' : 'Drag to reschedule'}
            >
              {#if onToggleSelect}
                <!-- The LABEL carries the 44px target; the box stays small. -->
                <label class="flex items-center justify-center shrink-0 min-h-11 min-w-11 cursor-pointer">
                  <input type="checkbox" checked={selected.has(event.id)} onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded w-4 h-4" />
                </label>
              {/if}
              <!-- v25 F3: drag affordance (hover-revealed grip). -->
              {#if event.state !== 'published'}
                <span class="hidden group-hover/chip:inline text-faint leading-none select-none" aria-hidden="true">⠿</span>
              {/if}
              <CalendarEvent {event} {onDuplicate} {onStats} {onDelete} />
            </div>
          {/each}
          <!-- Phase 2: ghost slots - per-channel posting time presets.
               v26: 44px target (was min-h-6 = 24px, under the 2.5.8 floor
               once padding shrank the label further) and text-xs so the
               channel name is readable; an aria-label because the visual
               label is a truncated icon+name. -->
          {#each (ghostsByHour.get(hourStr) || []) as ghost}
            <button
              class="flex items-center gap-1 border border-dashed border-line rounded px-1 min-h-11 opacity-50 hover:opacity-100 transition-opacity cursor-pointer"
              onclick={() => onDateClick?.(key, ghost.hour)}
              title="{ghost.integrationName} usually posts at {ghost.hour} - click to create"
              aria-label="Create a post at {ghost.hour} like {ghost.integrationName} usually does"
            >
              <span class="text-xs text-muted truncate">⏰ {ghost.integrationName}</span>
            </button>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>
