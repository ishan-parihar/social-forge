<script lang="ts">
  import { formatDateKey, getDayHours } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";
  import type { Integration } from "$lib/api/integrations";

  let { date, events = [], selected = new Set(), onEventClick, onDateClick, onDrop, onDuplicate, onStats, onDelete, onToggleSelect, integrations = [] }: {
    date: Date; events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string) => void;
    onDrop?: (eventId: string, newDate: string, newHour?: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
    integrations?: Integration[];
  } = $props();

  let key = $derived(formatDateKey(date));
  let dayEvents = $derived(events.filter(e => e.date === key));
  let hours = $derived(getDayHours());

  // Phase 2: ghost slots — per-channel posting time presets that have
  // no real post at that time. These show as dashed-border empty drop
  // targets so the user sees when their channel "usually" posts.
  // Each ghost slot is { hour: string, integrationName: string }.
  let ghostSlots = $derived.by(() => {
    const ghosts: { hour: string; integrationName: string; integrationId: string }[] = [];
    for (const int of integrations) {
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
      {@const hourStr = hour.slice(0, 2)}
      {@const isDragOver = dragOverHour === hourStr}
      <div class="flex border-b border-line min-h-[56px]">
        <div class="w-16 text-xs text-muted px-2 py-1 border-r border-line shrink-0">{hour}</div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="flex-1 px-2 py-1 space-y-0.5 cursor-pointer hover:bg-surface-hover transition-colors
            {isDragOver ? 'ring-2 ring-indigo-500 ring-inset bg-indigo-500/5' : ''}"
          onclick={() => onDateClick?.(key)}
          ondragover={(e) => { e.preventDefault(); dragOverHour = hourStr; }}
          ondragleave={() => { if (dragOverHour === hourStr) dragOverHour = null; }}
          ondrop={(e) => handleDrop(e, key, hourStr)}
          role="gridcell"
          tabindex="-1"
        >
          {#each (eventsByHour.get(hourStr) || []) as event (event.id)}
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
          <!-- Phase 2: ghost slots — per-channel posting time presets -->
          {#each (ghostsByHour.get(hourStr) || []) as ghost}
            <div
              class="flex items-center gap-1 border border-dashed border-line rounded px-1 py-0.5 opacity-50 hover:opacity-100 transition-opacity cursor-pointer"
              onclick={() => onDateClick?.(key)}
              title="{ghost.integrationName} usually posts at this time — click to create"
            >
              <span class="text-[10px] text-muted truncate">⏰ {ghost.integrationName}</span>
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>
