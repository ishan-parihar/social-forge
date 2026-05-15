<script lang="ts">
  import { formatDateKey, getDayHours } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { date, events = [], onEventClick }: {
    date: Date; events?: CEvent[];
    onEventClick?: (id: string) => void;
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
</script>

<div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
  <div class="text-center py-3 border-b border-[#1e2435]">
    <div class="text-lg font-semibold">
      {date.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" })}
    </div>
    <div class="text-xs text-[#6b7280]">{dayEvents.length} posts scheduled</div>
  </div>
  <div class="overflow-y-auto max-h-[700px]">
    {#each hours as hour}
      <div class="flex border-b border-[#1e2435] min-h-[56px]">
        <div class="w-16 text-xs text-[#6b7280] px-2 py-1 border-r border-[#1e2435] shrink-0">{hour}</div>
        <div class="flex-1 px-2 py-1 space-y-0.5">
          {#each (eventsByHour.get(hour.slice(0, 2)) || []) as event}
            <div onclick={() => onEventClick?.(event.id)}>
              <CalendarEvent {event} />
            </div>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>
