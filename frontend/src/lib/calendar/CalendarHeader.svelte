<script lang="ts">
  import type { CalendarView } from "./types";

  let { view, currentDate, onPrev, onNext, onToday, onViewChange }: {
    view: CalendarView; currentDate: Date;
    onPrev: () => void; onNext: () => void; onToday: () => void;
    onViewChange: (v: CalendarView) => void;
  } = $props();

  const views: CalendarView[] = ["month", "week", "day", "list"];

  let title = $derived.by(() => {
    const d = currentDate;
    const m = d.toLocaleDateString("en-US", { month: "long" });
    if (view === "month") return `${m} ${d.getFullYear()}`;
    if (view === "week") return `Week of ${m} ${d.getDate()}`;
    if (view === "day") return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
    return "Upcoming Posts";
  });
</script>

<div class="flex items-center justify-between flex-wrap gap-2">
  <div class="flex items-center gap-2">
    <button onclick={onPrev} aria-label="Previous" class="px-2 py-1 text-sm text-[#6b7280] hover:text-white rounded hover:bg-[#1a1f2e]">&larr;</button>
    <span class="text-lg font-semibold min-w-[200px] text-center">{title}</span>
    <button onclick={onNext} aria-label="Next" class="px-2 py-1 text-sm text-[#6b7280] hover:text-white rounded hover:bg-[#1a1f2e]">&rarr;</button>
    <button onclick={onToday} aria-label="Go to today" class="px-2 py-1 text-xs bg-[#1e2435] hover:bg-[#2a3045] rounded">Today</button>
  </div>
  <div class="flex bg-[#131720] rounded-lg border border-[#1e2435] overflow-hidden">
    {#each views as v}
      <button
        onclick={() => onViewChange(v)}
        aria-label={`${v} view`}
        class="px-3 py-1.5 text-xs font-medium capitalize transition-colors
          {view === v ? 'bg-indigo-600 text-white' : 'text-[#6b7280] hover:text-white hover:bg-[#1a1f2e]'}"
      >{v}</button>
    {/each}
  </div>
</div>
