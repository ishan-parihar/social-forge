<script lang="ts">
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
  }
  function nextMonth() {
    if (viewMonth === 11) { viewMonth = 0; viewYear++; }
    else viewMonth++;
  }

  function selectDate(dateStr: string) {
    if (isDisabled(dateStr)) return;
    value = dateStr;
    onchange?.(dateStr);
    open = false;
  }

  function toggle() { open = !open; }

  // Click-outside handler.
  $effect(() => {
    if (!open) return;
    function onDocClick(e: MouseEvent) {
      if (containerEl && !containerEl.contains(e.target as Node)) open = false;
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
    onclick={toggle}
    class="flex-1 px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary hover:border-brand-500/50 transition-colors text-left {className}"
    aria-haspopup="dialog"
    aria-expanded={open}
  >
    {#if displayValue}
      <span class="text-content">{displayValue}</span>
    {:else}
      <span class="text-muted">{placeholder}</span>
    {/if}
    <span class="text-muted ml-2 float-right">📅</span>
  </button>

  {#if open}
    <div
      class="absolute z-50 mt-1 p-3 bg-surface border border-line rounded-lg shadow-xl"
      style="min-width: 18rem;"
      role="dialog"
      aria-label="Date picker"
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
          <div class="text-center text-[10px] text-muted-dark font-medium py-1">{wd}</div>
        {/each}
      </div>

      <!-- Calendar grid -->
      <div class="grid grid-cols-7 gap-0.5">
        {#each grid as cell (cell.dateStr)}
          <button
            type="button"
            onclick={() => selectDate(cell.dateStr)}
            disabled={isDisabled(cell.dateStr)}
            class="aspect-square flex items-center justify-center rounded text-xs transition-colors
              {cell.isCurrent ? 'text-content-secondary' : 'text-muted-dark'}
              {cell.dateStr === value ? 'bg-brand-500 text-white font-bold' : ''}
              {cell.dateStr === todayStr && cell.dateStr !== value ? 'ring-1 ring-brand-400' : ''}
              {isDisabled(cell.dateStr) ? 'opacity-30 cursor-not-allowed' : 'hover:bg-surface-hover'}
            "
            aria-label={cell.dateStr}
            aria-pressed={cell.dateStr === value}
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
          class="text-xs text-brand-400 hover:underline"
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
