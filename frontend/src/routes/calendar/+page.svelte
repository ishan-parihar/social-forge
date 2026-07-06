<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { calendarApi } from "$lib/api/calendar";
  import { postsApi } from "$lib/api/posts";
  import { tagsApi, type Tag } from "$lib/api/tags";
  import { calendarState } from "$lib/stores/calendar.svelte";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";
  import { confirmModal } from "$lib/stores/modals.svelte";
  import { toCalendarEvent, type CalendarEvent, type CalendarView } from "$lib/calendar/types";
  import { formatDateKey } from "$lib/calendar/utils";
  import CalendarHeader from "$lib/calendar/CalendarHeader.svelte";
  import MonthView from "$lib/calendar/MonthView.svelte";
  import WeekView from "$lib/calendar/WeekView.svelte";
  import DayView from "$lib/calendar/DayView.svelte";
  import ListView from "$lib/calendar/ListView.svelte";
  import PostDetail from "$lib/calendar/PostDetail.svelte";
  import PostStatsModal from "$lib/calendar/PostStatsModal.svelte";
  import { goto } from "$app/navigation";

  let events = $state<CalendarEvent[]>([]);
  let allTags = $state<Tag[]>([]);
  let selectedTagId = $state<string | null>(null);
  let filteredEvents = $derived(
    selectedTagId
      ? events.filter(e => e.tags?.some(t => t.id === selectedTagId))
      : events
  );
  let selectedEvent = $state<CalendarEvent | null>(null);
  let duplicating = $state(false);
  let deleting = $state(false);
  let statsPostId = $state<string | null>(null);
  let loading = $state(false);
  let refreshing = $state(false);
  let fetchError = $state<string | null>(null);

  // Bulk selection
  let selected = $state<Set<string>>(new Set());
  let bulkScheduleDate = $state("");
  let bulkScheduleTime = $state("09:00");
  let showBulkSchedule = $state(false);
  let bulkProcessing = $state(false);

  function toggleSelect(id: string, e: Event) {
    e.stopPropagation();
    const s = new Set(selected);
    if (s.has(id)) s.delete(id); else s.add(id);
    selected = s;
  }

  function toggleAll() {
    if (selected.size === events.length) selected = new Set();
    else selected = new Set(events.map(e => e.id));
  }

  async function bulkDelete() {
    if (!confirm(`Delete ${selected.size} post(s)?`)) return;
    bulkProcessing = true;
    for (const id of selected) { await postsApi.delete(id); }
    selected = new Set();
    bulkProcessing = false;
    refresh();
  }

  async function bulkReschedule() {
    if (!bulkScheduleDate) return;
    bulkProcessing = true;
    const iso = `${bulkScheduleDate}T${bulkScheduleTime}:00.000Z`;
    for (const id of selected) { await postsApi.schedule(id, iso); }
    selected = new Set();
    showBulkSchedule = false;
    bulkProcessing = false;
    refresh();
  }

  async function fetchEvents(start: string, end: string) {
    loading = true;
    fetchError = null;
    const r = await calendarApi.get(start, end);
    if (r.data) {
      events = r.data.days.flatMap(d => d.posts.map(toCalendarEvent));
    } else {
      fetchError = r.error || "Failed to load calendar events";
    }
    loading = false;
  }

  function getMonthRange() {
    const d = calendarState.state.currentDate;
    const start = new Date(d.getFullYear(), d.getMonth(), 1);
    const end = new Date(d.getFullYear(), d.getMonth() + 1, 0);
    return { start: formatDateKey(start), end: formatDateKey(end) };
  }

  function getWeekRange() {
    const d = calendarState.state.currentDate;
    const start = new Date(d);
    start.setDate(start.getDate() - ((start.getDay() + 6) % 7)); // Monday
    const end = new Date(start);
    end.setDate(end.getDate() + 6);
    return { start: formatDateKey(start), end: formatDateKey(end) };
  }

  async function refresh() {
    if (refreshing) return;
    refreshing = true;
    try {
      const st = calendarState.state;
      if (st.view === "list") {
        await fetchEvents(
          formatDateKey(new Date()),
          formatDateKey(new Date(Date.now() + 90 * 24 * 60 * 60 * 1000))
        );
      } else {
        const r = st.view === "week" ? getWeekRange() : getMonthRange();
        await fetchEvents(r.start, r.end);
      }
    } finally {
      refreshing = false;
    }
  }

  function handleViewChange(v: CalendarView) {
    calendarState.setView(v);
    refresh();
  }
  function handlePrev() { calendarState.goBackward(); refresh(); }
  function handleNext() { calendarState.goForward(); refresh(); }
  function handleToday() { calendarState.goToday(); refresh(); }

  async function handleDrop(eventId: string, newDate: string, newHour?: string) {
    const event = events.find(e => e.id === eventId);
    if (!event) return;

    // Phase 1: published-post safety modal.
    // Instead of blocking with a toast, ask the user what they want to do.
    if (event.state === 'published') {
      const choice = await confirmModal({
        title: 'This post is already published',
        message: 'What do you want to do?',
        confirmLabel: 'Reschedule the post',
        cancelLabel: 'Just update the post details',
        danger: false,
      });
      // choice === true  → reschedule (re-publish at new time)
      // choice === false → just update (change scheduled_at without re-publishing)
      // Either way we proceed with the reschedule API call; the backend
      // PUT /api/posts/{id}/date will accept an `action` param in a
      // future iteration. For now, both paths do the same thing.
      // (The modal is the UX win; the backend distinction is deferred.)
    }

    // Phase 1: hour-precision. If a newHour was passed (from WeekView),
    // use it; otherwise fall back to the event's existing time.
    const time = newHour || event.time || "09:00";
    const dateObj = new Date(`${newDate}T${time}:00.000Z`);

    let moveGroup = false;
    if (event.groupId) {
      moveGroup = await confirmModal({
        title: 'Move campaign group?',
        message: 'Move all posts in this campaign by the same offset? Cancel to move only this post.',
        confirmLabel: 'Move all',
        cancelLabel: 'Just this post',
      });
    }

    const r = await postsApi.reschedule(eventId, dateObj.toISOString(), moveGroup);
    if (r.error) {
      toast("Failed to reschedule", "error");
    } else {
      const count = r.data?.count;
      if (moveGroup && count) {
        toast("Rescheduled " + count + " posts in group", "success");
      } else {
        toast("Post rescheduled", "success");
      }
      refresh();
    }
  }

  async function handleDuplicate(eventId: string) {
    if (duplicating) return;
    duplicating = true;
    try {
      const detail = await postsApi.get(eventId);
      if (detail.error || !detail.data) {
        toast(`Failed to fetch post: ${detail.error || 'unknown'}`, "error");
        return;
      }
      const post = detail.data;

      const slot = await postsApi.findSlot(post.integration_id);
      if (slot.error || !slot.data?.date) {
        toast(`Failed to find slot: ${slot.error || 'unknown'}`, "error");
        return;
      }

      // Preserve media, tags, and first_comment when duplicating —
      // the previous version only copied content + title + scheduled_at,
      // losing all the rich context the user had attached to the original.
      await postsApi.create({
        integration_ids: [post.integration_id],
        content: post.content,
        title: post.title,
        scheduled_at: slot.data.date,
        tag_ids: post.tags?.map(t => t.id) || [],
        first_comment: post.first_comment || undefined,
        media: (post.media || []).map(m => ({
          id: crypto.randomUUID(),
          url: m.url,
          mime_type: m.mime_type,
          alt: m.alt,
        })),
      });

      refresh();
    } catch (e) {
      toast(`Failed to duplicate post: ${e instanceof Error ? e.message : 'unknown'}`, "error");
    } finally {
      duplicating = false;
    }
  }

  function handleStats(eventId: string) {
    statsPostId = eventId;
  }

  async function handleDelete(eventId: string) {
    if (deleting) return;
    if (!confirm("Delete this post?")) return;
    deleting = true;
    try {
      const r = await postsApi.delete(eventId);
      if (r.error) toast(`Failed to delete post: ${r.error}`, "error");
      else refresh();
    } finally {
      deleting = false;
    }
  }

  let calUnsubscribers: (() => void)[] = [];

  onMount(async () => {
    refresh();
    // Fetch tags for the filter dropdown
    const tagRes = await tagsApi.list();
    if (tagRes.data) allTags = tagRes.data;
    const events = ['post_created', 'post_scheduled', 'post_published', 'post_failed', 'post_deleted'];
    for (const evt of events) {
      calUnsubscribers.push(realtime.on(evt, () => refresh()));
    }
  });

  onDestroy(() => {
    calUnsubscribers.forEach(fn => fn());
  });
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Content Calendar</h2>
    <button onclick={() => goto("/posts/new")} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">
      + New Post
    </button>
  </div>

  <CalendarHeader
    view={calendarState.state.view}
    currentDate={calendarState.state.currentDate}
    onPrev={handlePrev}
    onNext={handleNext}
    onToday={handleToday}
    onViewChange={handleViewChange}
    tags={allTags}
    {selectedTagId}
    onTagFilter={(id) => selectedTagId = id}
  />

  {#if selected.size > 0}
    <div class="flex items-center gap-3 bg-indigo-600/10 border border-indigo-500/30 rounded-lg px-4 py-2">
      <span class="text-sm text-indigo-300">{selected.size} selected</span>
      <button onclick={() => showBulkSchedule = !showBulkSchedule} disabled={bulkProcessing} class="px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">Reschedule</button>
      <button onclick={bulkDelete} disabled={bulkProcessing} class="px-3 py-1 text-xs bg-red-600 hover:bg-red-500 rounded disabled:opacity-50">Delete</button>
      <button onclick={() => selected = new Set()} class="ml-auto text-xs text-muted hover:text-white">Clear</button>
    </div>
    {#if showBulkSchedule}
      <div class="flex items-center gap-2 bg-background-input border border-line rounded-lg p-3">
        <input type="date" bind:value={bulkScheduleDate} class="px-2 py-1 bg-surface border border-line rounded text-sm text-content-secondary" />
        <input type="time" bind:value={bulkScheduleTime} class="px-2 py-1 bg-surface border border-line rounded text-sm text-content-secondary" />
        <button onclick={bulkReschedule} disabled={bulkProcessing || !bulkScheduleDate} class="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 rounded text-xs disabled:opacity-50">Apply</button>
      </div>
    {/if}
  {/if}

  {#if fetchError}
    <div class="text-center py-4 text-sm text-red-400">{fetchError}</div>
  {:else if loading}
    <div class="grid grid-cols-7 gap-px">
      {#each Array(35) as _, i (i)}
        <div class="h-24 bg-surface-hover animate-pulse rounded"></div>
      {/each}
    </div>
  {:else if calendarState.state.view === "month"}
    <MonthView
      year={calendarState.state.currentDate.getFullYear()}
      month={calendarState.state.currentDate.getMonth()}
      events={filteredEvents}
      {selected}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
      onDateClick={(date) => goto(`/posts/new?date=${date}`)}
      onDrop={handleDrop}
      onDuplicate={handleDuplicate}
      onStats={handleStats}
      onDelete={handleDelete}
      onToggleSelect={toggleSelect}
    />
  {:else if calendarState.state.view === "week"}
    <WeekView
      referenceDate={calendarState.state.currentDate}
      events={filteredEvents}
      {selected}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
      onDateClick={(date) => goto(`/posts/new?date=${date}`)}
      onDrop={handleDrop}
      onDuplicate={handleDuplicate}
      onStats={handleStats}
      onDelete={handleDelete}
      onToggleSelect={toggleSelect}
    />
  {:else if calendarState.state.view === "day"}
    <DayView
      date={calendarState.state.currentDate}
      events={filteredEvents}
      {selected}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
      onDateClick={(date) => goto(`/posts/new?date=${date}`)}
      onDrop={handleDrop}
      onDuplicate={handleDuplicate}
      onStats={handleStats}
      onDelete={handleDelete}
      onToggleSelect={toggleSelect}
    />
  {:else if calendarState.state.view === "list"}
    <ListView
      events={filteredEvents}
      {selected}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
      onDuplicate={handleDuplicate}
      onStats={handleStats}
      onDelete={handleDelete}
      onToggleSelect={toggleSelect}
    />
  {/if}

  <PostDetail event={selectedEvent} onclose={() => selectedEvent = null} onDuplicate={handleDuplicate} {duplicating} />

  {#if statsPostId}
    {@const post = events.find(e => e.id === statsPostId)}
    {#if post}
      <PostStatsModal postId={statsPostId} postTitle={post.title} onclose={() => statsPostId = null} />
    {/if}
  {/if}
</div>
