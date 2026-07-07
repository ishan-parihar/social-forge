export type CalendarView = "month" | "week" | "day" | "list";

// Phase 1: persist the user's preferred view + last-viewed date to
// localStorage so the calendar remembers its state across reloads.
// Default view is now 'week' (was 'month') — matches postiz-app and
// gives an hourly time grid that's far better for scheduling decisions.
const VIEW_KEY = 'social-forge-calendar-view';
const DATE_KEY = 'social-forge-calendar-date';

function getInitialView(): CalendarView {
  if (typeof localStorage !== 'undefined') {
    const stored = localStorage.getItem(VIEW_KEY);
    if (stored === 'month' || stored === 'week' || stored === 'day' || stored === 'list') {
      return stored;
    }
  }
  return 'week';
}

function getInitialDate(): Date {
  if (typeof localStorage !== 'undefined') {
    const stored = localStorage.getItem(DATE_KEY);
    if (stored) {
      const d = new Date(stored);
      if (!isNaN(d.getTime())) return d;
    }
  }
  return new Date();
}

let _state = $state({
  view: getInitialView(),
  currentDate: getInitialDate(),
  selectedDate: null as string | null,  // YYYY-MM-DD
  selectedPostId: null as string | null,
  // Phase 4: list-view state — state filter + pagination.
  listState: 'all' as 'all' | 'scheduled' | 'draft' | 'published',
  listPage: 1,
  listTotalPages: 1,
  // v23-8: a "now tick" that increments every 2 minutes, so calendar
  // views can re-evaluate isPast() without a full refetch. Postiz uses
  // a 2-2.5min useInterval for this. Components that depend on isPast
  // should read _state.nowTick in a $derived so they recompute when it
  // changes. Without this, a tab left open overnight wouldn't flip
  // yesterday's cells to greyscale until a manual refresh.
  nowTick: 0,
});

// v23-8: start a 2-minute interval that increments nowTick. This
// triggers re-evaluation of isPast-derived state in calendar views.
// The interval is cleaned up automatically when the module is unloaded
// (browser tab close) — no explicit cleanup needed for a singleton.
if (typeof window !== 'undefined') {
  setInterval(() => { _state.nowTick++; }, 120_000);
}

// Persist view + date changes to localStorage.
$effect(() => {
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(VIEW_KEY, _state.view);
    localStorage.setItem(DATE_KEY, _state.currentDate.toISOString());
  }
});

export const calendarState = {
  get state() { return _state; },
  setView(v: CalendarView) { _state.view = v; },
  goForward() {
    const d = new Date(_state.currentDate);
    if (_state.view === "month") d.setMonth(d.getMonth() + 1);
    else if (_state.view === "week") d.setDate(d.getDate() + 7);
    else d.setDate(d.getDate() + 1);
    _state.currentDate = d;
  },
  goBackward() {
    const d = new Date(_state.currentDate);
    if (_state.view === "month") d.setMonth(d.getMonth() - 1);
    else if (_state.view === "week") d.setDate(d.getDate() - 7);
    else d.setDate(d.getDate() - 1);
    _state.currentDate = d;
  },
  goToday() { _state.currentDate = new Date(); },
  /** Phase v21: set an explicit date (used by URL ?date= restore). */
  setDate(d: Date) {
    if (!isNaN(d.getTime())) _state.currentDate = d;
  },
  selectDate(date: string | null) { _state.selectedDate = date; },
  selectPost(id: string | null) { _state.selectedPostId = id; },
  // Phase 4: list-view state setters.
  setListState(s: 'all' | 'scheduled' | 'draft' | 'published') {
    _state.listState = s;
    _state.listPage = 1;  // reset to first page on filter change
  },
  setListPage(p: number) { _state.listPage = p; },
  setListTotalPages(n: number) { _state.listTotalPages = n; },
};
