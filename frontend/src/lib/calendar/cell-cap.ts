/**
 * The cell cap, in ONE place.
 *
 * A calendar cell renders at most this many chips; the rest are behind a
 * `+N more` disclosure that expands in place.
 *
 * Postiz does exactly this (`slice(0,3)` then `Show more (N)`), and switchpost
 * did not: with 8 posts seeded on one day, 23 chips rendered in the week grid
 * and 6 of them stacked inside a single 30-minute slot, with no `+N more`
 * affordance anywhere on the page. A wall is not a scan.
 *
 * Why three and not two or five: a week cell is ~130px wide and the chip is
 * 24px tall, so three chips plus the disclosure row still fit inside the
 * grid's `min-h-[48px]` rows in the CAPPED state - which is the state a
 * founder scans, and the only state they are in most of the time.
 *
 * MEASURED 2026-10-31 on the rendered week grid at 1440: a capped row is
 * 165px with all eight cells at 164px. EXPANDING that cell does grow the
 * row to 297px and pushes every row below it down by 132px - so the
 * earlier claim here that expanding does not push a row taller was WRONG.
 * What it does NOT do is desynchronise the grid: the row is a CSS grid and
 * the hour label is the row's own first child, so all eight cells AND the
 * label stretch together and stay aligned. An earlier comment here argued
 * that a taller row would push hour lines out of alignment with their
 * labels, and that reasoning was wrong because the label lives inside the
 * row. The cost of expanding is height, not alignment.
 *
 * Three is also the most a person compares at a glance before they start
 * reading. */
export const CELL_CAP = 3;

/**
 * The chips a cell should render, and the count hidden behind the disclosure.
 *
 * Returns the visible slice plus a `hidden` count (0 when `expanded`, so the
 * disclosure disappears and every chip shows). The count is what the `+N more`
 * row names, so it is exact rather than a guess the reader has to verify by
 * clicking.
 */
export function capCell<T>(items: T[], expanded = false): { visible: T[]; hidden: number } {
  if (expanded || items.length <= CELL_CAP) {
    return { visible: items, hidden: 0 };
  }
  return { visible: items.slice(0, CELL_CAP), hidden: items.length - CELL_CAP };
}
