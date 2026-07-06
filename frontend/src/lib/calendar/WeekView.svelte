<script lang="ts">
  import { buildWeekDays, getDayHours } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";
  import { timezone } from "$lib/stores/timezone.svelte";

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
</script>

<svelte:window ondragend={handleDragEnd} />

<div class="week-calendar bg-surface border border-line rounded-xl overflow-hidden">
  <div class="grid grid-cols-8 text-center border-b border-line">
    <div class="py-2 text-xs text-muted border-r border-line">{tzLabel}</div>
    {#each weekDays as wd (wd.dateStr)}
      <div class="py-2 text-xs {wd.isToday ? 'text-indigo-400' : 'text-muted'}">
        <div>{wd.date.toLocaleDateString("en-US", { weekday: "short" })}</div>
        <div class="font-semibold">{wd.date.getDate()}</div>
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
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="relative px-1 py-1 border-r border-line min-h-[48px] cursor-pointer hover:bg-surface-hover transition-colors
              {isDragOver ? 'ring-2 ring-indigo-500 ring-inset bg-indigo-500/5' : ''}"
            ondragover={(e) => { e.preventDefault(); handleDragEnter(wd.dateStr, hour); }}
            ondragleave={() => handleDragLeave(wd.dateStr, hour)}
            ondrop={(e) => handleDrop(e, wd.dateStr, hour)}
            onclick={() => onDateClick?.(wd.dateStr)}
            role="gridcell"
            tabindex="-1"
            onkeydown={(e) => handleKeyDown(e, wd.dateStr)}
          >
            {#each (eventsByDayHour.get(cellKey) || []) as event (event.id)}
              <div
                class="flex items-center gap-1 {event.state === 'published' ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
                draggable={event.state !== 'published'}
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
