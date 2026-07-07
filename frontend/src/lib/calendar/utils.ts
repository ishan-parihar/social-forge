import type { WeekDay, CalendarEvent } from "./types";

export function getMonthDays(year: number, month: number): Date[] {
  const first = new Date(year, month, 1);
  const last = new Date(year, month + 1, 0);
  const days: Date[] = [];
  // v22 Phase 7: Monday-start grid (matches getWeekDays + postiz).
  // Previously this was Sunday-start, which meant switching from week
  // view (Monday-start) to month view (Sunday-start) shifted the grid
  // by one column — confusing.
  // getDay() returns 0=Sun..6=Sat. Convert to 0=Mon..6=Sun:
  //   Mon=0, Tue=1, Wed=2, Thu=3, Fri=4, Sat=5, Sun=6
  const firstDayMon = (first.getDay() + 6) % 7;
  for (let i = firstDayMon; i > 0; i--) days.push(new Date(year, month, 1 - i));
  for (let d = 1; d <= last.getDate(); d++) days.push(new Date(year, month, d));
  // Trailing days to fill the last row (Monday-start).
  const lastDayMon = (last.getDay() + 6) % 7;
  const trailingDays = 6 - lastDayMon;
  for (let i = 1; i <= trailingDays; i++) days.push(new Date(year, month + 1, i));
  return days;
}

export function getWeekDays(date: Date): Date[] {
  const start = new Date(date);
  const day = start.getDay();
  const diff = day === 0 ? 6 : day - 1;
  start.setDate(start.getDate() - diff);
  const days: Date[] = [];
  for (let i = 0; i < 7; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    days.push(d);
  }
  return days;
}

export function getDayHours(): string[] {
  return Array.from({ length: 24 }, (_, i) =>
    `${String(i).padStart(2, "0")}:00`
  );
}

export function isToday(d: Date): boolean {
  const t = new Date();
  return d.getFullYear() === t.getFullYear() &&
    d.getMonth() === t.getMonth() &&
    d.getDate() === t.getDate();
}

export function isCurrentMonth(d: Date, y: number, m: number): boolean {
  return d.getFullYear() === y && d.getMonth() === m;
}

/**
 * Phase v21: returns true if the given date is strictly before today
 * (i.e., yesterday or earlier). Used by calendar views to grey-out past
 * cells and disable drop targets (postiz-style UX cue: you can't
 * schedule in the past).
 *
 * Compares calendar days only — time-of-day is ignored. A cell for
 * "today" returns false even if the current time is 23:59.
 */
export function isPast(d: Date): boolean {
  const t = new Date();
  // Compare year/month/day only.
  const today = new Date(t.getFullYear(), t.getMonth(), t.getDate());
  const cell = new Date(d.getFullYear(), d.getMonth(), d.getDate());
  return cell.getTime() < today.getTime();
}

export function formatDateKey(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function buildWeekDays(referenceDate: Date, events: CalendarEvent[]): WeekDay[] {
  const refMonth = referenceDate.getMonth();
  const refYear = referenceDate.getFullYear();
  return getWeekDays(referenceDate).map(d => {
    const key = formatDateKey(d);
    return {
      date: d,
      dateStr: key,
      isToday: isToday(d),
      isCurrentMonth: d.getMonth() === refMonth && d.getFullYear() === refYear,
      events: events.filter(e => e.date === key),
    };
  });
}

export const months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
// v22 Phase 7: Monday-start day labels (matches getMonthDays +
// getWeekDays, both of which now produce Monday-start grids).
export const days = ["Mon","Tue","Wed","Thu","Fri","Sat","Sun"];
export const monthsFull = ["January","February","March","April","May","June","July","August","September","October","November","December"];

export function formatDateTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}
