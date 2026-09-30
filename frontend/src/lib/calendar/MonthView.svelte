<script lang="ts">
  import { tick } from "svelte";
  import { getMonthDays, isToday, isCurrentMonth, isPast, formatDateKey, days, monthsFull } from "./utils";
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

  // v25 F3: track the hovered day so the drop target lights up. WeekView and
  // DayView already had this; the month grid silently accepted drops with no
  // feedback, so the user could not tell whether a cell was a valid target.
  let dragOverDate = $state<string | null>(null);

  function handleDragStart(e: DragEvent, eventId: string) {
    if (!e.dataTransfer) return;
    e.dataTransfer.setData("text/plain", eventId);
    e.dataTransfer.effectAllowed = "move";
  }

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

  // ---------------------------------------------------------------------
  // v25 F5 — keyboard reachability + the 360px pass.
  //
  // Before this the grid was reachable by NOBODY on the keyboard: every cell
  // carried tabindex="-1" and nothing ever called .focus() on one, so the
  // Enter/Space handler on the cell and on each event chip was dead code. The
  // calendar could be scrolled and clicked, but not operated.
  //
  // Three additions, and they are the whole of it:
  //   1. Roving tabindex over the day cells. One cell is tabbable (tabindex 0)
  //      and the rest are -1, so Tab enters the grid once instead of stopping
  //      35+ times; arrows then move between cells. This is the standard grid
  //      contract, and it is why role="grid"/"row"/"gridcell" appear below
  //      instead of the previous flat gridcell list (invalid structure that
  //      screen readers silently flatten).
  //   2. Alt+Arrow on a focused event chip reschedules it. Drag-and-drop had
  //      no keyboard equivalent at all, so rescheduling — the calendar's main
  //      verb — was mouse-only. It routes through the SAME onDrop prop the
  //      drag path uses, so there is one code path to the parent and no second
  //      "just update vs reschedule" decision to get wrong.
  //   3. A horizontal scroller with a width floor, replacing the root's
  //      `overflow-hidden`. Seven columns at 360px is ~40px each, which clipped
  //      every chip into an unreadable sliver.
  // ---------------------------------------------------------------------

  // Chunk into rows: `role="grid"` requires role="row" children, and
  // role="gridcell" inside those.
  let calRows = $derived.by(() => {
    const rows: Date[][] = [];
    for (let i = 0; i < calDays.length; i += 7) rows.push(calDays.slice(i, i + 7));
    return rows;
  });

  let gridEl: HTMLDivElement | undefined = $state();

  /** The one tabbable cell. Prefers today, then the first upcoming day. */
  let activeKey = $state<string>("");
  $effect(() => {
    // Re-seed whenever the visible month changes, so the tabbable cell always
    // exists inside the rendered grid.
    void year;
    void month;
    const today = calDays.find(isToday);
    const nextUp = calDays.find((d) => !isPast(d));
    activeKey = formatDateKey(today ?? nextUp ?? calDays[0]);
  });

  function focusCell(key: string) {
    activeKey = key;
    tick().then(() => {
      gridEl?.querySelector<HTMLElement>(`[data-date="${key}"]`)?.focus();
    });
  }

  function moveCell(key: string, delta: number) {
    const i = calDays.findIndex((d) => formatDateKey(d) === key);
    const target = calDays[i + delta];
    if (target) focusCell(formatDateKey(target));
  }

  function cellKeydown(e: KeyboardEvent, key: string, rowStart: number) {
    switch (e.key) {
      case "ArrowLeft":
        e.preventDefault();
        moveCell(key, -1);
        break;
      case "ArrowRight":
        e.preventDefault();
        moveCell(key, 1);
        break;
      case "ArrowUp":
        e.preventDefault();
        moveCell(key, -7);
        break;
      case "ArrowDown":
        e.preventDefault();
        moveCell(key, 7);
        break;
      case "Home":
        e.preventDefault();
        focusCell(formatDateKey(calDays[rowStart]));
        break;
      case "End":
        e.preventDefault();
        focusCell(formatDateKey(calDays[rowStart + 6]));
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        onDateClick?.(key);
        break;
    }
  }

  /** Alt+Arrow reschedules a chip. Returns true when it consumed the key. */
  function chipKeydown(e: KeyboardEvent, eventId: string, dateStr: string): boolean {
    if (!e.altKey || !onDrop) return false;
    const delta =
      e.key === "ArrowLeft" ? -1 :
      e.key === "ArrowRight" ? 1 :
      e.key === "ArrowUp" ? -7 :
      e.key === "ArrowDown" ? 7 : 0;
    if (!delta) return false;
    // Always swallow, even when the move is refused (past date), so the cell
    // behind does not ALSO read it as "move focus a week".
    e.preventDefault();
    e.stopPropagation();
    const [y, m, d] = dateStr.split("-").map(Number);
    const next = new Date(y, m - 1, d + delta);
    if (isPast(next)) return true;
    onDrop(eventId, formatDateKey(next));
    return true;
  }
</script>

<svelte:window ondragend={() => (dragOverDate = null)} />

<!-- v25 F5: `overflow-x-auto` + a min-width floor on the grids, replacing the
     root's `overflow-hidden`. The floor is 560px = 80px/day, which keeps the
     grid legible; on desktop the container is wider, so nothing scrolls. -->
<div class="month-calendar bg-surface border border-line rounded-xl overflow-x-auto">
  <div class="grid grid-cols-7 min-w-[560px] text-center text-xs text-muted py-2.5 border-b border-line" aria-hidden="true">
    {#each days as d}<span>{d}</span>{/each}
  </div>
  <div
    class="grid grid-cols-7 min-w-[560px]"
    bind:this={gridEl}
    role="grid"
    aria-label="Calendar for {monthsFull[month]} {year}. Arrow keys move between days."
  >
    {#each calRows as row, rowIdx (row[0].getTime())}
      <div class="contents" role="row">
        {#each row as date (formatDateKey(date))}
          {@const key = formatDateKey(date)}
          {@const dayEvents = eventsByDate.get(key) || []}
          {@const past = isPast(date)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            data-date={key}
            ondragover={(e) => { if (!past) handleDragOver(e, key); }}
            ondragleave={() => handleDragLeave(key)}
            ondrop={(e) => { if (!past) handleDrop(e, key); }}
            onclick={() => onDateClick?.(key)}
            role="gridcell"
            aria-label="{date.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' })}{dayEvents.length ? `, ${dayEvents.length} post${dayEvents.length > 1 ? 's' : ''}` : ', no posts'}"
            tabindex={activeKey === key ? 0 : -1}
            onfocus={() => (activeKey = key)}
            onkeydown={(e) => cellKeydown(e, key, rowIdx * 7)}
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
                  onkeydown={(e) => {
                    if (chipKeydown(e, event.id, key)) return;
                    if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); onEventClick?.(event.id); }
                  }}
                  role="button"
                  tabindex="0"
                  aria-label="{event.content?.slice(0, 60) || 'Post'}. Press Enter to open, Alt plus arrow keys to reschedule."
                  title={event.state === 'published' || past ? 'Published posts cannot be dragged' : 'Drag to reschedule'}
                  class="group/chip flex items-center gap-1 focus-visible:outline-2 focus-visible:outline-accent {event.state === 'published' || past ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
                >
                  {#if onToggleSelect}
                    <input type="checkbox" checked={selected.has(event.id)} aria-label="Select post for bulk actions" onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0 w-3 h-3" />
                  {/if}
                  <!-- v25 F3: drag affordance. A six-dot grip that only appears on
                       hover tells the user which chips are movable; without it the
                       whole cell reads as click-to-open. F5 keeps it hover-only
                       because the chip is now focusable and Alt+arrows reschedule,
                       so the grip is a mouse hint, not the only affordance. -->
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
    {/each}
  </div>
</div>