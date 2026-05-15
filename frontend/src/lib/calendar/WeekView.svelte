<script lang="ts">
  import { buildWeekDays, getDayHours } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { referenceDate, events = [], onEventClick, onDrop }: {
    referenceDate: Date; events?: CEvent[];
    onEventClick?: (id: string) => void;
    onDrop?: (eventId: string, newDate: string) => void;
  } = $props();

  let weekDays = $derived(buildWeekDays(referenceDate, events));
  let hours = $derived(getDayHours());

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

  function handleDrop(e: DragEvent, dateStr: string) {
    e.preventDefault();
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr);
  }
</script>

<div class="week-calendar bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
  <div class="grid grid-cols-8 text-center border-b border-[#1e2435]">
    <div class="py-2 text-xs text-[#6b7280] border-r border-[#1e2435]">GMT</div>
    {#each weekDays as wd}
      <div class="py-2 text-xs {wd.isToday ? 'text-indigo-400' : 'text-[#6b7280]'}">
        <div>{wd.date.toLocaleDateString("en-US", { weekday: "short" })}</div>
        <div class="font-semibold">{wd.date.getDate()}</div>
      </div>
    {/each}
  </div>
  <div class="overflow-y-auto max-h-[600px]">
    {#each hours as hour}
      <div class="grid grid-cols-8 border-b border-[#1e2435] min-h-[48px]">
        <div class="text-xs text-[#6b7280] px-2 py-1 border-r border-[#1e2435]">{hour}</div>
        {#each weekDays as wd}
          <div
            class="relative px-1 py-1 border-r border-[#1e2435] min-h-[48px]"
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => handleDrop(e, wd.dateStr)}
          >
            {#each (eventsByDayHour.get(`${wd.dateStr}-${hour.slice(0, 2)}`) || []) as event}
              <div onclick={() => onEventClick?.(event.id)}>
                <CalendarEvent {event} compact />
              </div>
            {/each}
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>
