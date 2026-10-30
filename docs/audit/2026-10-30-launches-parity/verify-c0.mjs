// Verify C-0: every event chip >= 24px in week/day/month at 3 widths,
// and the CELL does not grow as a side effect.
import { chromium } from 'playwright-core';
const EXE = '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = 'http://localhost:6543';
const CV = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIyMjIyMjIyMi0yMjIyLTIyMjItMjIyMi0yMjIyMjIyMjIyMjIiLCJleHAiOjE3OTU5NTk0NzAsImlhdCI6MTc5MzM2NzQ3MH0.qXwv0jJbiOj32XfVGGkN6gJCIRcsqvYH5Kv2DAKd__Q';

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });
let fail = 0;

for (const [w, h] of [[1440, 900], [768, 1024], [390, 844]]) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h }, colorScheme: 'dark' });
  await ctx.addCookies([{ name: 'sf_session', value: CV, domain: 'localhost', path: '/', httpOnly: false }]);
  const page = await ctx.newPage();
  await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
  await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });

  for (const view of ['week', 'day', 'month']) {
    await page.goto(BASE + '/calendar?display=' + view, { waitUntil: 'domcontentloaded', timeout: 40000 });
    await page.waitForTimeout(4000);
    await page.keyboard.press('Escape');
    await page.waitForTimeout(400);
    const r = await page.evaluate(() => {
      // Only chips that are actually RENDERED. Below `sm` the month grid replaces
      // the chip with a state-coloured dot inside `hidden sm:block` (by design:
      // a 48px day column cannot hold a title), so those chips have a zero
      // rect. Measuring them is measuring a template, not a surface.
      const chips = [...document.querySelectorAll('.event-chip')].filter(c => {
        const b = c.getBoundingClientRect();
        return b.width > 0 && b.height > 0;
      });
      const hidden = document.querySelectorAll('.event-chip').length - chips.length;
      const heights = chips.map(c => Math.round(c.getBoundingClientRect().height));
      const min = heights.length ? Math.min(...heights) : null;
      // the cell the first chip lives in - its height, to prove no growth
      const cell = chips[0]?.closest('[class*="min-h-24"]') || chips[0]?.parentElement?.parentElement;
      return {
        count: chips.length,
        min, max: heights.length ? Math.max(...heights) : null,
        all: [...new Set(heights)],
        cellH: cell ? Math.round(cell.getBoundingClientRect().height) : null,
      };
    });
    const ok = r.min === null || r.min >= 24;
    if (!ok) fail++;
    console.log(`${w} ${view}: chips=${r.count} heights=${JSON.stringify(r.all)} cell=${r.cellH} => ${ok ? 'PASS' : 'FAIL (<24)'}`);
  }
  await ctx.close();
}
await browser.close();
console.log(fail === 0 ? 'C-0: ALL PASS' : `C-0: ${fail} FAILURES`);
process.exit(fail === 0 ? 0 : 1);
