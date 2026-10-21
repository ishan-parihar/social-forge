// ─── Wall clock <-> UTC instant ─────────────────────────────────
//
// A schedule is a WALL CLOCK reading in a named zone ("09:00 in
// America/New_York"), and the backend stores an INSTANT (UTC). The
// conversion between the two is not a fixed offset: it depends on the
// zone AND on the date, because of daylight saving.
//
// These helpers are the one place that conversion lives. The composer's
// SchedulePicker, the calendar slot click and the composer preset all
// use them, so a schedule entered one way reads the same as one entered
// another.
//
// The conversion is done with `Intl`, not with a hand-rolled offset
// table: `Intl` knows every zone and every DST rule in the browser's
// ICU data.

/**
 * The UTC instant for `date` + `time` read as a wall clock in `zone`.
 *
 * `date` is `YYYY-MM-DD`, `time` is `HH:mm` or `HH:mm:ss`. The result
 * is an ISO-8601 UTC instant, which is what the API expects.
 *
 * The seconds are carried through when they are given. They used to
 * be dropped and re-appended as `:00`, so a caller that passed
 * `23:59:59` got `23:59:59:00` - a colon in the seconds field, which
 * is NOT RFC 3339. The calendar's range sent that and the server
 * answered "Invalid end date".
 */
export function wallClockToUtc(date: string, time: string, zone: string): string {
  const hasSeconds = /^\d{2}:\d{2}:\d{2}$/.test(time);
  const clock = hasSeconds ? time : `${time}:00`;
  // Anchor: pretend the wall clock IS UTC. That gives a ts whose *fields*
  // are exactly what the user typed, which is what we measure the zone's
  // offset against.
  const anchor = Date.parse(`${date}T${clock}Z`);
  if (Number.isNaN(anchor)) return `${date}T${clock}.000Z`;

  let ts = anchor - zoneOffsetMs(zone, anchor);
  // One correction pass: near a DST transition the offset at the anchor can
  // differ from the offset at the answer, which would land the post an hour
  // out. Re-measuring at the candidate instant fixes it.
  const adjusted = anchor - zoneOffsetMs(zone, ts);
  if (adjusted !== ts) ts = adjusted;
  return new Date(ts).toISOString();
}

/**
 * The zone's UTC offset at an instant, in MILLISECONDS.
 *
 * THE ONE IMPLEMENTATION of "what is this zone's offset" (AGENTS.md
 * §9.18: one implementation per rule). `wallClockToUtc` uses it, and
 * the calendar uses it to tell the server which LOCAL DAY a post
 * belongs to - so a post at 02:00 UTC lands on the day the founder
 * sees, not the UTC day.
 *
 * Ask Intl what the wall clock reads in `zone` at that instant, rebuild
 * it as a UTC instant, and the difference is the zone's offset.
 *
 * hourCycle 'h23' keeps midnight at 00. (With `hour12: false` some ICU
 * builds render it as 24, which would push the whole day forward.)
 */
export function zoneOffsetMs(zone: string, at: number): number {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: zone, hourCycle: 'h23',
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
  }).formatToParts(new Date(at));
  const get = (t: string) => Number(parts.find(p => p.type === t)?.value ?? '0');
  return Date.UTC(get('year'), get('month') - 1, get('day'), get('hour'), get('minute'), get('second')) - at;
}

/**
 * A UTC instant read as a wall clock in `zone`, as {date, time}.
 *
 * The inverse of [`wallClockToUtc`].
 */
export function utcToWallClock(iso: string, zone: string): { date: string; time: string } {
  try {
    const parts = new Intl.DateTimeFormat('en-CA', {
      timeZone: zone, hourCycle: 'h23',
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    }).formatToParts(new Date(iso));
    const get = (t: string) => parts.find(p => p.type === t)?.value ?? '';
    return { date: `${get('year')}-${get('month')}-${get('day')}`, time: `${get('hour')}:${get('minute')}` };
  } catch {
    return { date: iso.slice(0, 10), time: iso.slice(11, 16) };
  }
}
