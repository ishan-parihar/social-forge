export type CalendarView = "month" | "week" | "day" | "list";

let _state = $state({
  view: "month" as CalendarView,
  currentDate: new Date(),        // reference date for the view
  selectedDate: null as string | null,  // YYYY-MM-DD
  selectedPostId: null as string | null,
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
  selectDate(date: string | null) { _state.selectedDate = date; },
  selectPost(id: string | null) { _state.selectedPostId = id; },
};
