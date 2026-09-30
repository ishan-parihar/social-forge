<script lang="ts">
  import { tick } from "svelte";
  import { buildWeekDays, getDayHours, isPast, isToday } from "./utils";
  import CalendarEvent from "./CalendarEvent.svelte";
  import type { CalendarEvent as CEvent } from "./types";
  import { timezone } from "$lib/stores/timezone.svelte";
  import { makeTouchDragHandler, type TouchDropTarget } from "./touch-drag";

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

  // v25 F5: this used to be a stub — it preventDefault'd Enter/Space and then
  // did nothing, and because every cell was tabindex="-1" nothing ever reached
  // it anyway, so the whole week grid was keyboard-dead. It now moves focus
  // between cells. Layout is 8 columns (tz gutter + 7 days) and row-major, so
  // horizontal arrows move a day and vertical arrows move an hour — which is
  // what the grid looks like, so the mapping needs no explanation in the UI.
  let gridEl: HTMLDivElement | undefined = $state();
  let activeCell = $state<string>("");

  /** Tab stops are seeded once per rendered week so exactly one cell is tabbable. */
  $effect(() => {
    const first = hours[0] ?? "00:00";
    const firstDay = weekDays[0]?.dateStr;
    if (firstDay) activeCell = `${firstDay}-${first.slice(0, 2)}`;
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
    if (isPast(target.date)) return true;
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
   * Past cells get opacity-40 + cursor-not-allowed and drop is suppressed.
   */
  function isCellPast(date: Date, hour: string): boolean {
    if (isPast(date)) return true;
    if (isToday(date)) {
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
<div class="week-calendar bg-surface border border-line rounded-xl overflow-x-auto" ontouchstart={touchDrag.onTouchStart} ontouchmove={touchDrag.onTouchMove} ontouchend={touchDrag.onTouchEnd}>
  <div class="grid grid-cols-8 min-w-[640px] text-center border-b border-line" role="row">
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
  <div class="overflow-y-auto max-h-[600px]" bind:this={gridEl} role="grid" aria-label="Week view. Arrow keys move between hour slots.">
    {#each hours as hour (hour)}
      <div class="grid grid-cols-8 min-w-[640px] border-b border-line min-h-[48px]" role="row">
        <div class="text-xs text-muted px-2 py-1 border-r border-line min-w-0 truncate">{hour}</div>
        {#each weekDays as wd (wd.dateStr)}
          {@const cellKey = `${wd.dateStr}-${hour}`}
          {@const isDragOver = dragOverKey === cellKey}
          {@const past = isCellPast(wd.date, hour)}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            data-drop-date={wd.dateStr}
            data-drop-hour={hour.slice(0, 2)}
            data-cell={`${wd.dateStr}-${hour.slice(0, 2)}`}
            class="relative px-1 py-1 border-r border-line min-h-[48px] cursor-pointer hover:bg-surface-hover transition-colors
              {isDragOver ? 'ring-2 ring-accent ring-inset bg-accent-fill/5' : ''}
              {wd.isToday ? 'bg-accent-fill/5' : ''}
              {past ? 'opacity-40 cursor-not-allowed' : ''}"
            title={past ? 'Past — cannot schedule here' : `Drop to move to ${wd.date.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric' })} at ${hour}`}
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
            {#each (eventsByDayHour.get(cellKey) || []) as event (event.id)}
              <div
                data-event-id={event.id}
                class="group/chip flex items-center gap-1 {event.state === 'published' || past ? 'cursor-default' : 'cursor-grab active:cursor-grabbing'}"
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
                  <input type="checkbox" checked={selected.has(event.id)} aria-label="Select post for bulk actions" onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0 w-3 h-3" />
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
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>
