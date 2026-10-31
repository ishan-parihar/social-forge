<script lang="ts">
  import { tick } from "svelte";
  import { buildWeekDays, getDayHours, isPast, isToday } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import CellMoreButton from "./CellMoreButton.svelte";
  import { capCell } from "./cell-cap";
  import type { CalendarEvent as CEvent } from "./types";
  import { timezone } from "$lib/stores/timezone.svelte";
  import { makeTouchDragHandler, type TouchDropTarget } from "./touch-drag";

  let { referenceDate, events = [], selected = new Set(), onEventClick, onDateClick, onDrop, onDuplicate, onStats, onDelete, onToggleSelect }: {
    referenceDate: Date; events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDateClick?: (date: string, hour?: string) => void;
    onDrop?: (eventId: string, newDate: string, newHour?: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
  } = $props();

  let weekDays = $derived(buildWeekDays(referenceDate, events));
  let hours = $derived(getDayHours());

  // Show the user's selected timezone abbreviation in the corner instead
  // of a hard-coded "GMT" - matches what the column times actually render in.
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
        // Row keys are the full "HH:00" from getDayHours(); bucket by that,
        // or no chip ever matches its cell (cellKey uses the full hour).
        const hour = (e.time || "00:00").slice(0, 2);
        const key = `${wd.dateStr}-${hour}:00`;
        const list = map.get(key) || [];
        list.push(e);
        map.set(key, list);
      }
    }
    return map;
  });

  // Which cells have revealed their `+N more`. Keyed by cell, so expanding
  // one hour does not expand every hour in the week, and navigating away and
  // back does not leave a day half-expanded.
  let expandedCells = $state(new Set<string>());

  function toggleCell(key: string) {
    const next = new Set(expandedCells);
    if (next.has(key)) next.delete(key);
    else next.add(key);
    expandedCells = next;
  }

  function handleDrop(e: DragEvent, dateStr: string, hour: string) {
    e.preventDefault();
    const id = e.dataTransfer?.getData("text/plain");
    if (id && onDrop) onDrop(id, dateStr, hour);
  }

  function handleDragStart(e: DragEvent, eventId: string) {
    e.dataTransfer?.setData("text/plain", eventId);
    e.dataTransfer!.effectAllowed = "move";
  }

  // v25 F5: this used to be a stub - it preventDefault'd Enter/Space and then
  // did nothing, and because every cell was tabindex="-1" nothing ever reached
  // it anyway, so the whole week grid was keyboard-dead. It now moves focus
  // between cells. Layout is 8 columns (tz gutter + 7 days) and row-major, so
  // horizontal arrows move a day and vertical arrows move an hour - which is
  // what the grid looks like, so the mapping needs no explanation in the UI.
  let gridEl: HTMLDivElement | undefined = $state();
  let activeCell = $state<string>("");

  /** Tab stops are seeded once per rendered week so exactly one cell is tabbable. */
  $effect(() => {
    const first = hours[0] ?? "00:00";
    const firstDay = weekDays[0]?.dateStr;
    if (firstDay) activeCell = `${firstDay}-${first.slice(0, 2)}`;
  });

  // Open the week at the start of the working day (postiz opens at the
  // first slot with content; 07:00 is the sane floor either way). Without
  // this the grid mounts scrolled to midnight and the founder's whole
  // working day is below the fold.
  //
  // Run once per MOUNT (the `didInitialScroll` flag). Re-running on week
  // change would yank the scroller back while the founder is reading a
  // later hour; a fresh mount (navigation) is the only moment it helps.
  let didInitialScroll = $state(false);
  $effect(() => {
    void weekDays;
    if (didInitialScroll || !gridEl) return;
    const withPosts = weekDays.flatMap((wd) => wd.events);
    let targetHour = "07:00";
    if (withPosts.length > 0) {
      const earliest = withPosts
        .map((e) => (e.time || "23:59").slice(0, 2) + ":00")
        .sort()[0];
      targetHour = earliest;
    }
    didInitialScroll = true;
    const idx = hours.indexOf(targetHour);
    if (idx > 0) gridEl.scrollTop = idx * 48;
  });

  function focusCell(cellKey: string) {
    activeCell = cellKey;
    tick().then(() => {
      gridEl?.querySelector<HTMLElement>(`[data-cell="${cellKey}"]`)?.focus();
    });
  }

  function handleKeyDown(e: KeyboardEvent, dateStr: string, hour: string) {
    const dayIdx = weekDays.findIndex((d) => d.dateStr === dateStr);
    const hourIdx = hours.indexOf(hour);
    const at = (d: number, h: number) => {
      if (d < 0 || d > 6 || h < 0 || h > 23) return;
      focusCell(`${weekDays[d].dateStr}-${hours[h].slice(0, 2)}`);
    };
    switch (e.key) {
      case "ArrowLeft": e.preventDefault(); at(dayIdx - 1, hourIdx); break;
      case "ArrowRight": e.preventDefault(); at(dayIdx + 1, hourIdx); break;
      case "ArrowUp": e.preventDefault(); at(dayIdx, hourIdx - 1); break;
      case "ArrowDown": e.preventDefault(); at(dayIdx, hourIdx + 1); break;
      case "Home": e.preventDefault(); at(0, hourIdx); break;
      case "End": e.preventDefault(); at(6, hourIdx); break;
      case "Enter":
      case " ":
        e.preventDefault();
        onDateClick?.(dateStr);
        break;
    }
  }

  /**
   * v25 F5: Alt+Arrow on a focused chip reschedules it, so the week view's
   * main verb is not mouse-only. Routes through the same onDrop the drag path
   * uses, so there is one path to the parent. Returns true when consumed.
   */
  function chipKeydown(e: KeyboardEvent, eventId: string, dateStr: string, hour: string): boolean {
    if (!e.altKey || !onDrop) return false;
    const dayIdx = weekDays.findIndex((d) => d.dateStr === dateStr);
    const hourIdx = hours.indexOf(hour);
    const d = e.key === "ArrowLeft" ? dayIdx - 1 : e.key === "ArrowRight" ? dayIdx + 1 : dayIdx;
    const h = e.key === "ArrowUp" ? hourIdx - 1 : e.key === "ArrowDown" ? hourIdx + 1 : hourIdx;
    if (e.key !== "ArrowLeft" && e.key !== "ArrowRight" && e.key !== "ArrowUp" && e.key !== "ArrowDown") return false;
    // Swallow even when refused, so the cell behind does not also move focus.
    e.preventDefault();
    e.stopPropagation();
    if (d < 0 || d > 6 || h < 0 || h > 23) return true;
    const target = weekDays[d];
    if (isPast(target.date, timezone.value)) return true;
    onDrop(eventId, target.dateStr, hours[h].slice(0, 2));
    return true;
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
   * Past cells get the `calendar-cell-past` class (a dimmed
   * BACKGROUND, not an `opacity` - see the style block at the bottom)
   * plus cursor-not-allowed, and drop is suppressed.
   */
  function isCellPast(date: Date, hour: string): boolean {
    if (isPast(date, timezone.value)) return true;
    if (isToday(date, timezone.value)) {
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

<!-- v25 F5: `overflow-x-auto` + a 640px floor (8 columns x 80px) replaces the
     root's `overflow-hidden`. Previously the 8 tracks compressed to ~39px each
     at a 360px viewport and every chip was clipped into a sliver, with no way to
     scroll to the rest. The vertical scroller stays on the body, so the header
     and body scroll together horizontally. -->
<div role="group" class="week-calendar bg-surface border border-line rounded-xl flex flex-col overflow-x-auto max-h-full" ontouchstart={touchDrag.onTouchStart} ontouchmove={touchDrag.onTouchMove} ontouchend={touchDrag.onTouchEnd}>
  <!-- The header is its own scroller child at `top:0` of the BODY
       scroller below, so the day names stay pinned while the hours
       scroll (postiz pins the same 62px header). -->
  <div class="grid grid-cols-8 min-w-[640px] text-center border-b border-line sticky top-0 z-10 bg-surface" role="row">
    <div class="py-2 px-2 text-xs text-muted border-r border-line min-w-0 truncate" title={timezone.value}>{tzLabel}</div>
    {#each weekDays as wd (wd.dateStr)}
      <!-- v25 F3: today is a filled accent header cell, not just accent-colored
           text. The old version (accent text + a 4px dot) was easy to lose
           against 6 muted columns, which is exactly the moment the user needs
           the cue most. -->
      <div class="py-2 text-xs text-center relative {wd.isToday ? 'bg-accent-fill/10 text-accent-strong' : 'text-muted'}">
        <div>{wd.date.toLocaleDateString("en-US", { weekday: "short" })}</div>
        <div class="font-semibold">{wd.date.getDate()}</div>
        {#if wd.isToday}
          <div class="absolute inset-x-0 bottom-0 h-0.5 bg-accent-fill"></div>
        {/if}
      </div>
    {/each}
  </div>
  <!-- max-h is set by the page to the viewport minus chrome, so the
       hours scroll INSIDE the calendar card with the day header pinned
       - the postiz shape. A fixed 600px wasted the space below the fold
       on a tall screen and clipped the grid on a short one. -->
  <div class="overflow-y-auto flex-1 min-h-0 overscroll-contain" bind:this={gridEl} role="grid" aria-label="Week view. Arrow keys move between hour slots.">
    <!-- Scroll the current hour into view on first render (postiz
         opens at the top of the working day, not at midnight). -->
    {#each hours as hour (hour)}
      <div class="grid grid-cols-8 min-w-[640px] border-b border-line min-h-[48px]" role="row">
        <div class="text-xs text-muted px-2 py-1 border-r border-line min-w-0 truncate">{hour}</div>
        {#each weekDays as wd (wd.dateStr)}
          {@const cellKey = `${wd.dateStr}-${hour}`}
          {@const isDragOver = dragOverKey === cellKey}
          {@const past = isCellPast(wd.date, hour)}
          {@const cellEvents = eventsByDayHour.get(cellKey) || []}
          {@const isExpanded = expandedCells.has(cellKey)}
          {@const capped = capCell(cellEvents, isExpanded)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            data-drop-date={wd.dateStr}
            data-drop-hour={hour.slice(0, 2)}
            data-cell={`${wd.dateStr}-${hour.slice(0, 2)}`}
            class="relative px-1 py-1 border-r border-line min-h-[48px] cursor-pointer hover:bg-surface-hover transition-colors
              {isDragOver ? 'ring-2 ring-accent ring-inset bg-accent-fill/5' : ''}
              {wd.isToday ? 'bg-accent-fill/5' : ''}
              {past ? 'calendar-cell-past cursor-not-allowed' : ''}"
            title={past ? 'Past - cannot schedule here' : `Drop to move to ${wd.date.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' })} at ${hour}`}
            ondragover={(e) => { if (!past) { e.preventDefault(); handleDragEnter(wd.dateStr, hour); } }}
            ondragleave={() => handleDragLeave(wd.dateStr, hour)}
            ondrop={(e) => { if (!past) handleDrop(e, wd.dateStr, hour); }}
            onclick={() => { if (!past) onDateClick?.(wd.dateStr); }}
            role="gridcell"
            aria-label="{wd.date.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' })} at {hour}{past ? ', past' : ''}"
            tabindex={activeCell === `${wd.dateStr}-${hour.slice(0, 2)}` ? 0 : -1}
            onfocus={() => (activeCell = `${wd.dateStr}-${hour.slice(0, 2)}`)}
            onkeydown={(e) => handleKeyDown(e, wd.dateStr, hour)}
          >
            {#each capped.visible as event (event.id)}
              <!-- `flex-1 min-w-0` so the chip keeps the cell's width when
                   the title collapses on hover. Without it the wrapper
                   shrank to the width of its content, which pulled the
                   hover toolbar left and made the cell jump sideways
                   mid-hover. -->
              <div
                data-event-id={event.id}
                class="group/chip relative flex items-center gap-1 flex-1 min-w-0 min-h-11 {event.state === 'published' || past ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
                draggable={event.state !== 'published' && !past}
                ondragstart={(e) => handleDragStart(e, event.id)}
                onclick={() => onEventClick?.(event.id)}
                onkeydown={(e) => {
                  if (chipKeydown(e, event.id, wd.dateStr, hour)) return;
                  if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); onEventClick?.(event.id); }
                }}
                role="button"
                tabindex="0"
                aria-label="{event.content?.slice(0, 60) || 'Post'} at {hour}. Press Enter to open, Alt plus arrow keys to reschedule."
                title={event.state === 'published' || past ? 'Published posts cannot be dragged' : 'Drag to reschedule'}
              >
                {#if onToggleSelect}
                  <!--
                    The LABEL carries the 44px target; the box stays small.

                    The label is an OVERLAY, not a flow sibling. A week
                    cell is 80px wide, and a 44px label in the flow left
                    the chip 23px - the chip showed "06:49" and no title
                    at all. Overlaying it puts the 44px target on top of
                    the chip's left edge instead of beside it, and the
                    chip keeps the whole cell.

                    The chip's own left padding makes room for it, and
                    the box only appears on hover - a bulk-select
                    affordance should not sit permanently over the
                    post's title.
                  -->
                  <label class="absolute left-0 top-0 z-10 flex items-center justify-center shrink-0 min-h-11 min-w-11 cursor-pointer opacity-0 group-hover/chip:opacity-100 focus-within:opacity-100 transition-opacity">
                    <input type="checkbox" checked={selected.has(event.id)} aria-label="Select post for bulk actions" onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded w-4 h-4" />
                  </label>
                {/if}
                <!-- v25 F3: drag affordance (hover-revealed grip). F5 keeps it
                     hover-only: the chip is focusable and Alt+arrows reschedule,
                     so this is a mouse hint, not the only affordance. -->
                {#if event.state !== 'published' && !past}
                  <span class="hidden group-hover/chip:inline text-faint leading-none select-none" aria-hidden="true">⠿</span>
                {/if}
                <CalendarEvent {event} {onDuplicate} {onStats} {onDelete} />
              </div>
            {/each}
            <!-- The cell cap: postiz's `slice(0,3)` + `Show more (N)`. Without
                 it a busy slot is a wall - 23 chips rendered for 8 seeded
                 posts, 6 of them stacked inside this one cell. -->
            {#if capped.hidden > 0}
              <CellMoreButton hidden={capped.hidden} expanded={isExpanded} onToggle={() => toggleCell(cellKey)} />
            {/if}
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  /*
    A PAST cell is not a drop target, and it used to say so with
    `opacity-40` on the cell.

    An `opacity` is a GROUP opacity: it fades the cell's ENTIRE subtree,
    including the post chips inside it. A scheduled post in a past hour
    dropped to 1.33:1 - on the calendar and unreadable. The founder
    still has to be able to see and open what they posted last week.

    So the past-ness is carried by the cell's CHROME - its background
    and its border - and the chips inside it keep their own contrast.
    `cursor-not-allowed` and the cell's title already carry the "you
    cannot schedule here" signal.
  */
  .calendar-cell-past {
    background-color: rgb(var(--bg-rgb) / 0.45);
  }
  .calendar-cell-past:hover {
    background-color: rgb(var(--bg-rgb) / 0.45);
  }
</style>
