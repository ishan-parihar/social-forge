<script lang="ts">
  // ListView — calendar list view with date grouping, state filter, and
  // pagination (Phase 4, v19).
  //
  // Upgraded from the v18 basic list to match postiz-app's list view:
  //   - Groups posts by date (YYYY-MM-DD), sorted ascending
  //   - State filter segmented control (All / Scheduled / Draft / Published)
  //   - Server-side pagination (◀ Page N of M ▶)
  //   - Date headers with weekday name
  //   - Empty-state messages per filter
  //
  // The state filter + pagination live in the calendarState store so they
  // persist when switching between calendar views.

  import Badge from "$lib/ui/Badge.svelte";
  import { calendarState } from "$lib/stores/calendar.svelte";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";
  import { monthsFull, days } from "./utils";
  import type { CalendarEvent as CEvent } from "./types";

  let { events = [], selected = new Set(), onEventClick, onDuplicate, onStats, onDelete, onToggleSelect, showActions = false }: {
    events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
    showActions?: boolean;
  } = $props();

  // State filter options.
  const stateFilters = [
    { value: 'all', label: 'All' },
    { value: 'scheduled', label: 'Scheduled' },
    { value: 'draft', label: 'Drafts' },
    { value: 'published', label: 'Published' },
  ] as const;

  // Filter events by the selected state.
  let filteredEvents = $derived.by(() => {
    const ls = calendarState.state.listState;
    if (ls === 'all') return events;
    return events.filter(e => {
      if (ls === 'scheduled') return e.state === 'queued';
      if (ls === 'draft') return e.state === 'draft';
      if (ls === 'published') return e.state === 'published';
      return true;
    });
  });

  // Group filtered events by date (YYYY-MM-DD), sorted ascending.
  // Drafts without dates go into a "No date" group at the end.
  let groupedByDate = $derived.by(() => {
    const groups = new Map<string, CEvent[]>();
    const noDate: CEvent[] = [];

    for (const e of filteredEvents) {
      if (!e.date) {
        noDate.push(e);
      } else {
        const list = groups.get(e.date) || [];
        list.push(e);
        groups.set(e.date, list);
      }
    }

    // Sort each group's events by time.
    for (const [date, list] of groups) {
      list.sort((a, b) => (a.time || "00:00").localeCompare(b.time || "00:00"));
    }

    // Sort groups by date ascending.
    const sortedGroups = Array.from(groups.entries()).sort((a, b) => a[0].localeCompare(b[0]));

    // Append the no-date group if any.
    if (noDate.length > 0) {
      noDate.sort((a, b) => a.title.localeCompare(b.title));
      sortedGroups.push(['__no_date__', noDate]);
    }

    return sortedGroups;
  });

  // Format a date string (YYYY-MM-DD) as "Monday, 5 May 2026".
  function formatDateHeader(dateStr: string): string {
    if (dateStr === '__no_date__') return 'No date (drafts)';
    const d = new Date(dateStr + 'T00:00:00');
    const dayName = days[d.getDay()];
    return `${dayName}, ${d.getDate()} ${monthsFull[d.getMonth()]} ${d.getFullYear()}`;
  }

  // Pagination controls.
  let currentPage = $derived(calendarState.state.listPage);
  let totalPages = $derived(calendarState.state.listTotalPages);

  function prevPage() {
    if (currentPage > 1) calendarState.setListPage(currentPage - 1);
  }
  function nextPage() {
    if (currentPage < totalPages) calendarState.setListPage(currentPage + 1);
  }
</script>

<div class="bg-surface border border-line rounded-xl overflow-hidden">
  <!-- Header: state filter + pagination -->
  <div class="px-4 py-2.5 border-b border-line flex items-center justify-between gap-2 flex-wrap">
    <!-- State filter segmented control -->
    <div class="flex gap-1 bg-background-input rounded-lg p-0.5">
      {#each stateFilters as f}
        <button
          onclick={() => calendarState.setListState(f.value)}
          class="px-2.5 py-1 text-[11px] rounded-md transition-colors {calendarState.state.listState === f.value ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover'}"
        >{f.label}</button>
      {/each}
    </div>

    <!-- Pagination -->
    {#if totalPages > 1}
      <div class="flex items-center gap-2 text-xs text-muted">
        <button
          onclick={prevPage}
          disabled={currentPage <= 1}
          class="px-2 py-1 rounded bg-surface-hover disabled:opacity-30 hover:bg-line transition-colors"
          aria-label="Previous page"
        >◀</button>
        <span>Page {currentPage} of {totalPages}</span>
        <button
          onclick={nextPage}
          disabled={currentPage >= totalPages}
          class="px-2 py-1 rounded bg-surface-hover disabled:opacity-30 hover:bg-line transition-colors"
          aria-label="Next page"
        >▶</button>
      </div>
    {/if}
  </div>

  <!-- List body -->
  {#if groupedByDate.length === 0}
    <div class="text-center py-12 text-sm text-muted">
      {#if calendarState.state.listState === 'scheduled'}
        No scheduled posts
      {:else if calendarState.state.listState === 'draft'}
        No draft posts
      {:else if calendarState.state.listState === 'published'}
        No published posts
      {:else}
        No posts found
      {/if}
    </div>
  {:else}
    {#each groupedByDate as [dateStr, dayEvents] (dateStr)}
      <!-- Date header -->
      <div class="px-4 py-2 bg-surface-hover border-b border-line text-xs font-semibold text-muted uppercase tracking-wider">
        {formatDateHeader(dateStr)}
      </div>
      <!-- Events for this date -->
      {#each dayEvents as event (event.id)}
        <div class="group relative w-full flex items-center gap-4 px-4 py-3 border-b border-line hover:bg-surface-hover transition-colors">
          {#if onToggleSelect}
            <input type="checkbox" checked={selected.has(event.id)} onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0" />
          {/if}
          <button onclick={() => onEventClick?.(event.id)} class="flex-1 flex items-center gap-4 text-left min-w-0">
            <div class="text-xs text-muted w-20 shrink-0">
              {#if event.time}
                <span class="text-indigo-400 font-mono">{event.time}</span>
              {:else if dateStr !== '__no_date__'}
                <span class="text-muted-dark">—</span>
              {/if}
            </div>
            <div class="flex-1 min-w-0">
              <div class="text-sm truncate">{event.title}</div>
              <div class="text-xs text-muted">{event.integrationName}</div>
              {#if event.likes != null || event.comments != null || event.impressions != null}
                <div class="flex gap-2 mt-0.5">
                  {#if event.likes != null}<span class="text-[10px] text-pink-400/60" title="{engagementLabel('likes', event.platform)}">{engagementIcon('likes', event.platform)} {formatMetricCount(event.likes)}</span>{/if}
                  {#if event.comments != null}<span class="text-[10px] text-yellow-400/60" title="{engagementLabel('comments', event.platform)}">{engagementIcon('comments', event.platform)} {formatMetricCount(event.comments)}</span>{/if}
                  {#if event.impressions != null}<span class="text-[10px] text-indigo-400/60" title="{engagementLabel('impressions', event.platform)}">{engagementIcon('impressions', event.platform)} {formatMetricCount(event.impressions)}</span>{/if}
                </div>
              {/if}
            </div>
            <Badge state={event.state as "draft" | "queued" | "published" | "error"} />
            {#if event.error}
              <span class="text-xs text-red-400" title={event.error}>⚠</span>
            {/if}
          </button>
          <div class="invisible group-hover:visible group-focus-within:visible transition-all duration-150 flex items-center gap-1 shrink-0">
            {#if event.state === 'published' && event.postUrl}
              <a href={event.postUrl} target="_blank" rel="noopener noreferrer"
                 class="text-indigo-400 hover:text-indigo-300 px-1.5 py-0.5 rounded text-xs"
                 title="View original post">🔗</a>
            {/if}
            <button onclick={() => onDuplicate?.(event.id)} class="text-muted hover:text-content px-1.5 py-0.5 rounded text-xs" title="Duplicate" aria-label="Duplicate post">📋</button>
            <button onclick={() => onStats?.(event.id)} class="text-muted hover:text-content px-1.5 py-0.5 rounded text-xs" title="Statistics" aria-label="View post statistics">📊</button>
            <button onclick={() => onDelete?.(event.id)} class="text-muted hover:text-error px-1.5 py-0.5 rounded text-xs" title="Delete" aria-label="Delete post">🗑️</button>
          </div>
        </div>
      {/each}
    {/each}
  {/if}
</div>
