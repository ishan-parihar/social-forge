<script lang="ts">
  import { onMount } from "svelte";
  import { calendarApi } from "$lib/api/calendar";
  import { postsApi } from "$lib/api/posts";
  import { calendarState } from "$lib/stores/calendar";
  import { toCalendarEvent, type CalendarEvent, type CalendarView } from "$lib/calendar/types";
  import { formatDateKey } from "$lib/calendar/utils";
  import CalendarHeader from "$lib/calendar/CalendarHeader.svelte";
  import MonthView from "$lib/calendar/MonthView.svelte";
  import WeekView from "$lib/calendar/WeekView.svelte";
  import DayView from "$lib/calendar/DayView.svelte";
  import ListView from "$lib/calendar/ListView.svelte";
  import PostDetail from "$lib/calendar/PostDetail.svelte";
  import { goto } from "$app/navigation";

  let events = $state<CalendarEvent[]>([]);
  let selectedEvent = $state<CalendarEvent | null>(null);

  async function fetchEvents(start: string, end: string) {
    const r = await calendarApi.get(start, end);
    if (r.data) {
      events = r.data.days.flatMap(d => d.posts.map(toCalendarEvent));
    }
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
  }

  function handleViewChange(v: CalendarView) {
    calendarState.setView(v);
    refresh();
  }
  function handlePrev() { calendarState.goBackward(); refresh(); }
  function handleNext() { calendarState.goForward(); refresh(); }
  function handleToday() { calendarState.goToday(); refresh(); }

  async function handleDrop(eventId: string, newDate: string) {
    const event = events.find(e => e.id === eventId);
    const time = event?.time || "00:00";
    const dateObj = new Date(`${newDate}T${time}:00Z`);
    await postsApi.schedule(eventId, dateObj.toISOString());
    refresh();
  }

  async function handleDuplicate(eventId: string) {
    try {
      // Get full post detail
      const detail = await postsApi.get(eventId);
      if (!detail.data) return;

      const post = detail.data;

      // Find next available slot
      const slot = await postsApi.findSlot(post.integration_id);
      if (!slot.data) return;

      // Create copy
      await postsApi.create({
        integration_ids: [post.integration_id],
        content: post.content,
        title: post.title,
        scheduled_at: slot.data.date,
      });

      refresh();
    } catch (e) {
      console.error("Failed to duplicate post:", e);
    }
  }

  onMount(() => refresh());
</script>

<div class="space-y-4">
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
  />

  {#if calendarState.state.view === "month"}
    <MonthView
      year={calendarState.state.currentDate.getFullYear()}
      month={calendarState.state.currentDate.getMonth()}
      {events}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
      onDateClick={(date) => calendarState.selectDate(date)}
      onDrop={handleDrop}
    />
  {:else if calendarState.state.view === "week"}
    <WeekView
      referenceDate={calendarState.state.currentDate}
      {events}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
      onDrop={handleDrop}
    />
  {:else if calendarState.state.view === "day"}
    <DayView
      date={calendarState.state.currentDate}
      {events}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
    />
  {:else if calendarState.state.view === "list"}
    <ListView
      {events}
      onEventClick={(id) => selectedEvent = events.find(e => e.id === id) || null}
    />
  {/if}

  <PostDetail event={selectedEvent} onclose={() => selectedEvent = null} onDuplicate={handleDuplicate} />
</div>
