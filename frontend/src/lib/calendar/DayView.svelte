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

  // State summary for the day — shows at a glance how many posts are
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
  <div class="py-3 border-b border-line px-4">
    <div class="text-lg font-semibold text-center">
      {date.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" })}
    </div>
    <div class="flex items-center justify-center gap-3 mt-1 text-xs">
      <span class="text-muted">{dayEvents.length} total</span>
      {#if stateSummary.queued > 0}
        <span class="text-indigo-400">{stateSummary.queued} scheduled</span>
      {/if}
      {#if stateSummary.published > 0}
        <span class="text-green-400">{stateSummary.published} published</span>
      {/if}
      {#if stateSummary.draft > 0}
        <span class="text-muted">{stateSummary.draft} drafts</span>
      {/if}
      {#if stateSummary.error > 0}
        <span class="text-red-400">{stateSummary.error} failed</span>
      {/if}
    </div>
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
