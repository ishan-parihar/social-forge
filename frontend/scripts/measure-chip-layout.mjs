// Measure the chip's internal layout: which child is too wide, and by how much.
import { chromium } from 'playwright-core';

const EXE = process.env.PW_EXE || '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = process.env.SP_URL || 'https://localhost:6543';
const CV = process.env.CV;

if (!CV) { console.log('SKIP: no CV'); process.exit(0); }

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: 'dark', ignoreHTTPSErrors: true });
await ctx.addCookies([{ name: 'sf_session', value: CV, domain: 'localhost', path: '/', httpOnly: false }]);
const page = await ctx.newPage();
await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
await page.goto(BASE + '/calendar?display=week', { waitUntil: 'domcontentloaded', timeout: 40000 });
await page.waitForTimeout(4500);
await page.keyboard.press('Escape');
await page.waitForTimeout(500);

const out = await page.evaluate(() => {
  // The chips that the probe reported as overlapping.
  const chips = [...document.querySelectorAll('[data-event-id]')].filter((c) => {
    const r = c.getBoundingClientRect();
    if (r.width === 0 || r.top < 60 || r.top > 800) return false;
    const inner = c.querySelector('.event-chip');
    if (!inner) return false;
    return inner.querySelector('.event-tags') && inner.querySelector('.event-time');
  });
  return chips.slice(0, 4).map((c) => {
    const inner = c.querySelector('.event-chip');
    const ir = inner.getBoundingClientRect();
    const tags = inner.querySelector('.event-tags');
    const tagChips = [...tags.querySelectorAll('.tag-chip')];
    const time = inner.querySelector('.event-time');
    const content = inner.querySelector('.event-content');
    const rect = (el) => { const r = el.getBoundingClientRect(); return { x: Math.round(r.x), w: Math.round(r.width), h: Math.round(r.height), top: Math.round(r.top) }; };
    return {
      label: (c.getAttribute('aria-label') || '').slice(0, 30),
      chipBox: rect(inner),
      contentBox: rect(content),
      time: rect(time),
      tagsBlock: rect(tags),
      tagChips: tagChips.map((t) => ({ text: t.textContent, ...rect(t) })),
      scrollW: inner.scrollWidth, clientW: inner.clientWidth,
    };
  });
});
console.log(JSON.stringify(out, null, 1));
await browser.close();
