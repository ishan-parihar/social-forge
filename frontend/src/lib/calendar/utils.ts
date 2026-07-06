import type { WeekDay, CalendarEvent } from "./types";

export function getMonthDays(year: number, month: number): Date[] {
  const first = new Date(year, month, 1);
  const last = new Date(year, month + 1, 0);
  const days: Date[] = [];
  // Leading days (from previous month)
  // Sunday-start grid (matches MonthView day headers)
  const leadingDays = first.getDay(); // 0=Sun..6=Sat → how many prev-month days to fill
  for (let i = leadingDays; i > 0; i--) days.push(new Date(year, month, 1 - i));
  for (let d = 1; d <= last.getDate(); d++) days.push(new Date(year, month, d));
  // Trailing days (from next month)
  const trailingDays = 6 - last.getDay();
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
export const days = ["Sun","Mon","Tue","Wed","Thu","Fri","Sat"];
export const monthsFull = ["January","February","March","April","May","June","July","August","September","October","November","December"];

export function formatDateTime(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit" });
}
