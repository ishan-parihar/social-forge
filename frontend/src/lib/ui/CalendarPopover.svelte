<script lang="ts">
  import { tick } from 'svelte';

  // v26-3: CalendarPopover — a Mantine-style date picker popover.
  //
  // Pure Svelte, no new deps. Shows a month grid with weekday headers,
  // prev/next month nav, today highlight, selected-date highlight, and
  // optional past-date disabling. The selected date is displayed in the
  // trigger button using locale-aware format.
  //
  // Usage:
  //   <CalendarPopover bind:value={dateStr} placeholder="Select date" />
  //
  // `value` is a YYYY-MM-DD string (matching native <input type="date">).
  // This keeps the integration with SchedulePicker simple — the rest of
  // the composer works with ISO date strings.

  let {
    value = $bindable(''),
    placeholder = 'Select date',
    min,
    onchange,
    class: className = '',
  }: {
    value?: string;
    placeholder?: string;
    /** Minimum selectable date (YYYY-MM-DD). Dates before this are disabled. */
    min?: string;
    /** Fired when the user selects a date (or clears). */
    onchange?: (value: string) => void;
    class?: string;
  } = $props();

  let open = $state(false);
  let viewYear = $state(0);
  let viewMonth = $state(0); // 0-11
  let containerEl: HTMLDivElement;
  let triggerEl: HTMLButtonElement | undefined = $state();
  let panelEl: HTMLDivElement | undefined = $state();

  // v25 F5 — keyboard. The trigger already declared aria-haspopup/aria-expanded,
  // but the popover had no keydown handler at all: Escape did not close it, the
  // 42 day buttons were reachable only by Tab (43 stops), and focus was never
  // moved in on open nor returned to the trigger on close. Added:
  //   - roving tabindex over the days, so Tab enters the grid once
  //   - Arrow keys ±1/±7 days, Home/End, PageUp/PageDown for months
  //   - Escape closes and returns focus to the trigger
  let focusedDate = $state<string | null>(null);

  function close(returnFocus = true) {
    open = false;
    if (returnFocus) triggerEl?.focus();
  }

  function focusDay(dateStr: string) {
    if (!dateStr || isDisabled(dateStr)) return;
    focusedDate = dateStr;
    tick().then(() => {
      panelEl?.querySelector<HTMLElement>(`[data-day="${dateStr}"]`)?.focus();
    });
  }

  /** Move by whole days inside the fixed 42-cell grid, so arrows never dead-end. */
  function moveDay(delta: number) {
    const i = grid.findIndex((c) => c.dateStr === focusedDate);
    const from = i >= 0 ? i : Math.max(0, grid.findIndex((c) => c.dateStr === value));
    const next = grid[Math.min(grid.length - 1, Math.max(0, from + delta))];
    if (next) focusDay(next.dateStr);
  }

  function onGridKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case "ArrowLeft": e.preventDefault(); moveDay(-1); break;
      case "ArrowRight": e.preventDefault(); moveDay(1); break;
      case "ArrowUp": e.preventDefault(); moveDay(-7); break;
      case "ArrowDown": e.preventDefault(); moveDay(7); break;
      case "Home": e.preventDefault(); focusDay(grid[0]?.dateStr ?? ''); break;
      case "End": e.preventDefault(); focusDay(grid[grid.length - 1]?.dateStr ?? ''); break;
      case "PageUp": e.preventDefault(); prevMonth(); break;
      case "PageDown": e.preventDefault(); nextMonth(); break;
      case "Escape": e.preventDefault(); close(); break;
    }
  }

  // Initialize view month/year from the current value or today.
  $effect(() => {
    if (value && /^\d{4}-\d{2}-\d{2}$/.test(value)) {
      const [y, m] = value.split('-').map(Number);
      if (viewYear === 0 && viewMonth === 0) {
        viewYear = y;
        viewMonth = m - 1;
      }
    }
  });

  // Default to current month if not set.
  $effect(() => {
    if (viewYear === 0 || viewMonth === 0) {
      const now = new Date();
      viewYear = now.getFullYear();
      viewMonth = now.getMonth();
    }
  });

  const WEEKDAYS = ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'];
  const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June',
                   'July', 'August', 'September', 'October', 'November', 'December'];

  // Build the 6-week calendar grid for the current view month.
  // Always 42 cells (6 weeks × 7 days) so the grid doesn't jump in height.
  let grid = $derived.by(() => {
    if (!viewYear || !viewMonth) return [];
    const firstOfMonth = new Date(viewYear, viewMonth, 1);
    const startDay = firstOfMonth.getDay(); // 0=Sun
    const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
    const daysInPrevMonth = new Date(viewYear, viewMonth, 0).getDate();
    const cells: Array<{ day: number; month: number; year: number; isCurrent: boolean; dateStr: string }> = [];
    // Previous month's trailing days.
    for (let i = startDay - 1; i >= 0; i--) {
      const day = daysInPrevMonth - i;
      const m = viewMonth === 0 ? 11 : viewMonth - 1;
      const y = viewMonth === 0 ? viewYear - 1 : viewYear;
      cells.push({ day, month: m, year: y, isCurrent: false, dateStr: formatDateStr(y, m, day) });
    }
    // Current month's days.
    for (let day = 1; day <= daysInMonth; day++) {
      cells.push({ day, month: viewMonth, year: viewYear, isCurrent: true, dateStr: formatDateStr(viewYear, viewMonth, day) });
    }
    // Next month's leading days.
    while (cells.length < 42) {
      const idx = cells.length - startDay - daysInMonth + 1;
      const m = viewMonth === 11 ? 0 : viewMonth + 1;
      const y = viewMonth === 11 ? viewYear + 1 : viewYear;
      cells.push({ day: idx, month: m, year: y, isCurrent: false, dateStr: formatDateStr(y, m, idx) });
    }
    return cells;
  });

  function formatDateStr(y: number, m: number, d: number): string {
    return `${y}-${String(m + 1).padStart(2, '0')}-${String(d).padStart(2, '0')}`;
  }

  let todayStr = $derived.by(() => {
    const now = new Date();
    return formatDateStr(now.getFullYear(), now.getMonth(), now.getDate());
  });

  function isDisabled(dateStr: string): boolean {
    if (!min) return false;
    return dateStr < min;
  }

  function prevMonth() {
    if (viewMonth === 0) { viewMonth = 11; viewYear--; }
    else viewMonth--;
    focusedDate = null;
  }
  function nextMonth() {
    if (viewMonth === 11) { viewMonth = 0; viewYear++; }
    else viewMonth++;
    focusedDate = null;
  }

  function selectDate(dateStr: string) {
    if (isDisabled(dateStr)) return;
    value = dateStr;
    onchange?.(dateStr);
    close();
  }

  function toggle() { open = !open; }

  // Click-outside handler.
  $effect(() => {
    if (!open) return;
    function onDocClick(e: MouseEvent) {
      if (containerEl && !containerEl.contains(e.target as Node)) close(false);
    }
    document.addEventListener('click', onDocClick);
    return () => document.removeEventListener('click', onDocClick);
  });

  // Format the selected value for display: "Jul 8, 2025"
  let displayValue = $derived.by(() => {
    if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return '';
    const [y, m, d] = value.split('-').map(Number);
    const date = new Date(y, m - 1, d);
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  });
</script>

<div class="relative inline-block" bind:this={containerEl}>
  <button
    type="button"
    bind:this={triggerEl}
    onclick={toggle}
    class="flex-1 px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary hover:border-accent/50 transition-colors text-left {className}"
    aria-haspopup="dialog"
    aria-expanded={open}
  >
    {#if displayValue}
      <span class="text-content">{displayValue}</span>
    {:else}
      <span class="text-muted">{placeholder}</span>
    {/if}
    <span class="text-muted ml-2 float-right" aria-hidden="true">📅</span>
  </button>

  {#if open}
    <!-- F5: min-width capped against the viewport. 18rem = 288px was fine at
         desktop but overflowed a 360px phone once the layout gutter was added
         back. Arrow keys / Escape are handled on the panel. -->
    <div
      bind:this={panelEl}
      class="absolute z-50 mt-1 p-3 bg-surface border border-line rounded-lg shadow-lg w-[min(18rem,calc(100vw-2rem))]"
      role="dialog"
      aria-label="Date picker"
      tabindex="-1"
      onkeydown={onGridKeydown}
    >
      <!-- Month nav -->
      <div class="flex items-center justify-between mb-3">
        <button
          type="button"
          onclick={prevMonth}
          class="w-7 h-7 flex items-center justify-center rounded text-muted hover:text-content hover:bg-surface-hover transition-colors"
          aria-label="Previous month"
        >‹</button>
        <span class="text-sm font-medium text-content">{MONTHS[viewMonth]} {viewYear}</span>
        <button
          type="button"
          onclick={nextMonth}
          class="w-7 h-7 flex items-center justify-center rounded text-muted hover:text-content hover:bg-surface-hover transition-colors"
          aria-label="Next month"
        >›</button>
      </div>

      <!-- Weekday headers -->
      <div class="grid grid-cols-7 gap-0.5 mb-1">
        {#each WEEKDAYS as wd}
          <div class="text-center text-[10px] text-faint font-medium py-1">{wd}</div>
        {/each}
      </div>

      <!-- Calendar grid. F5: role=grid/row/gridcell so the roving tabindex has a
           valid structure to live in — a bare button list announces as 42
           unrelated buttons. -->
      <div class="grid grid-cols-7 gap-0.5" role="grid" aria-label="{MONTHS[viewMonth]} {viewYear}">
        {#each grid as cell (cell.dateStr)}
          <button
            type="button"
            data-day={cell.dateStr}
            onclick={() => selectDate(cell.dateStr)}
            onfocus={() => (focusedDate = cell.dateStr)}
            disabled={isDisabled(cell.dateStr)}
            class="aspect-square flex items-center justify-center rounded text-xs transition-colors
              {cell.isCurrent ? 'text-content-secondary' : 'text-faint'}
              {cell.dateStr === value ? 'bg-accent-fill text-accent-fg font-bold' : ''}
              {cell.dateStr === todayStr && cell.dateStr !== value ? 'ring-1 ring-accent' : ''}
              {isDisabled(cell.dateStr) ? 'opacity-30 cursor-not-allowed' : 'hover:bg-surface-hover'}
            "
            role="gridcell"
            tabindex={focusedDate === cell.dateStr ? 0 : -1}
            aria-label={new Date(cell.year, cell.month, cell.day).toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' })}
            aria-selected={cell.dateStr === value}
          >
            {cell.day}
          </button>
        {/each}
      </div>

      <!-- Today shortcut -->
      <div class="mt-2 pt-2 border-t border-line flex justify-between items-center">
        <button
          type="button"
          onclick={() => selectDate(todayStr)}
          class="text-xs text-accent hover:underline"
        >Today</button>
        {#if value}
          <button
            type="button"
            onclick={() => { value = ''; onchange?.(''); open = false; }}
            class="text-xs text-muted hover:text-error"
          >Clear</button>
        {/if}
      </div>
    </div>
  {/if}
</div>
