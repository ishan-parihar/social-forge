// With 8 posts seeded on today, what does the busiest week cell render?
// Is there a cap and a "+N more" affordance, or does the cell grow/scroll?
import { chromium } from 'playwright-core';
const EXE = '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = 'http://localhost:6543';
const CV = 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIyMjIyMjIyMi0yMjIyLTIyMjItMjIyMi0yMjIyMjIyMjIyMjIiLCJleHAiOjE3OTU5NTk0NzAsImlhdCI6MTc5MzM2NzQ3MH0.qXwv0jJbiOj32XfVGGkN6gJCIRcsqvYH5Kv2DAKd__Q';

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: 'dark' });
await ctx.addCookies([{ name: 'sf_session', value: CV, domain: 'localhost', path: '/', httpOnly: false }]);
const page = await ctx.newPage();
await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
await page.goto(BASE + '/calendar?display=week', { waitUntil: 'domcontentloaded', timeout: 40000 });
await page.waitForTimeout(4500);
await page.keyboard.press('Escape');
await page.waitForTimeout(600);
await page.screenshot({ path: '/tmp/shots/a6-week-8posts-1440.png' });

const r = await page.evaluate(() => {
  // find the element that directly contains the most chips
  let best = null;
  for (const el of document.querySelectorAll('*')) {
    const n = el.querySelectorAll('.event-chip').length;
    if (n > 0 && (!best || n > best.n)) {
      // prefer the SHALLOWEST such element
      if (!best || n > best.n) best = { el, n };
    }
  }
  // find the innermost container of chips (a cell)
  const chips = [...document.querySelectorAll('.event-chip')];
  // group by offsetTop, the row slots in the week grid
  const byTop = {};
  chips.forEach(c => { const t = Math.round(c.getBoundingClientRect().top / 40); byTop[t] = (byTop[t] || 0) + 1; });
  const slotCounts = Object.values(byTop).sort((a, b) => b - a);
  // a per-cell count: use the day column (the day header cell is a sibling)
  const dayCounts = [...document.querySelectorAll('[class*="flex-col"]')]
    .map(el => el.querySelectorAll('.event-chip').length)
    .filter(n => n > 0).sort((a, b) => b - a).slice(0, 3);

  return {
    maxChipsInOneContainer: best ? best.n : 0,
    chipsPerTimeSlot: slotCounts.slice(0, 5),
    chipsPerColumn: dayCounts,
    totalChips: chips.length,
    anyMoreAffordance: [...document.querySelectorAll('button,span,a,div')]
      .filter(e => /^\s*\+\s*\d+\s*$/.test(e.textContent || '') || /show more|\+\d+ more/i.test(e.textContent || ''))
      .map(e => (e.textContent || '').trim()).slice(0, 5),
    chipHeights: [...new Set(chips.map(c => Math.round(c.getBoundingClientRect().height)))],
    firstThreeTexts: chips.slice(0, 3).map(c => (c.textContent || '').replace(/\s+/g, ' ').slice(0, 50)),
  };
});
console.log(JSON.stringify(r, null, 1));
await ctx.close();
await browser.close();
