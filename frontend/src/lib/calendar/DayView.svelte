<script lang="ts">
  import { formatDateKey, getDayHours } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { date, events = [], selected = new Set(), onEventClick, onDateClick, onDrop, onDuplicate, onStats, onDelete, onToggleSelect }: {
    date: Date; events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string) => void;
    onDrop?: (eventId: string, newDate: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
  } = $props();

  let key = $derived(formatDateKey(date));
  let dayEvents = $derived(events.filter(e => e.date === key));
  let hours = $derived(getDayHours());

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

  function handleDrop(e: DragEvent, dateStr: string) {
    e.preventDefault();
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr);
  }

  function handleDragStart(e: DragEvent, eventId: string) {
    e.dataTransfer?.setData("text/plain", eventId);
    e.dataTransfer!.effectAllowed = "move";
  }
</script>

<div class="bg-surface border border-line rounded-xl overflow-hidden">
  <div class="text-center py-3 border-b border-line">
    <div class="text-lg font-semibold">
      {date.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" })}
    </div>
    <div class="text-xs text-muted">{dayEvents.length} posts scheduled</div>
  </div>
  <div class="overflow-y-auto max-h-[700px]">
    {#each hours as hour (hour)}
      <div class="flex border-b border-line min-h-[56px]">
        <div class="w-16 text-xs text-muted px-2 py-1 border-r border-line shrink-0">{hour}</div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="flex-1 px-2 py-1 space-y-0.5 cursor-pointer hover:bg-surface-hover"
          onclick={() => onDateClick?.(key)}
          ondragover={(e) => e.preventDefault()}
          ondrop={(e) => handleDrop(e, key)}
          role="gridcell"
          tabindex="-1"
        >
          {#each (eventsByHour.get(hour.slice(0, 2)) || []) as event (event.id)}
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
      </div>
    {/each}
  </div>
</div>
