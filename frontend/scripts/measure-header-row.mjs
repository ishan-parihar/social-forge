// Why does the C-2 header row measure 88px at 1440 when every control in it
// is h-10 (40px)? A flex row that wraps is the symptom; this probe reports
// each child's measured width so the overflowing one is identified by number,
// not by guessing from the class list.
import { chromium } from 'playwright-core';

const EXE = process.env.PW_EXE || '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = process.env.SP_URL || 'http://127.0.0.1:6543';
const CV = process.env.CV;

if (!CV) { console.log('SKIP: no CV'); process.exit(0); }

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });

function measure() {
  // The header row is the div that carries the nav group and the view switch.
  const row = [...document.querySelectorAll('div.page-enter > div')].find(
    (d) => d.querySelector('select') && d.querySelector('[role="group"]'));
  if (!row) return { error: 'no header row' };
  const cs = getComputedStyle(row);
  const kids = [...row.children].map((el) => {
    const r = el.getBoundingClientRect();
    return {
      tag: el.tagName.toLowerCase(),
      w: Math.round(r.width), h: Math.round(r.height),
      top: Math.round(r.top), left: Math.round(r.left),
      text: (el.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 30),
    };
  });
  return {
    rowW: Math.round(row.getBoundingClientRect().width),
    rowH: Math.round(row.getBoundingClientRect().height),
    flexWrap: cs.flexWrap, gapX: cs.columnGap, gapY: cs.rowGap,
    kids,
    // Total width the row needs if nothing wrapped.
    needs: kids.reduce((a, k) => a + k.w, 0) + kids.length * 8,
  };
}

try {
  for (const width of [1440, 1100, 900, 768, 390]) {
    const ctx = await browser.newContext({
      viewport: { width, height: 900 }, colorScheme: 'dark', ignoreHTTPSErrors: true,
    });
    await ctx.addCookies([{ name: 'sf_session', value: CV, domain: new URL(BASE).hostname, path: '/', httpOnly: false }]);
    const page = await ctx.newPage();
    await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
    await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
    await page.goto(BASE + '/calendar?display=week', { waitUntil: 'domcontentloaded', timeout: 40000 });
    await page.waitForTimeout(4500);
    for (let i = 0; i < 4; i++) { await page.keyboard.press('Escape'); await page.waitForTimeout(250); }
    await page.waitForTimeout(300);
    const m = await page.evaluate(measure);
    console.log(`\n===== ${width}px =====`);
    if (m.error) { console.log(m.error); await ctx.close(); continue; }
    console.log(`row: ${m.rowW}x${m.rowH}  wrap=${m.flexWrap} gap=${m.gapX}/${m.gapY}  kids need ~${m.needs}px`);
    for (const k of m.kids) {
      console.log(`  ${String(k.w).padStart(4)}x${String(k.h).padStart(3)}  top=${String(k.top).padStart(4)} left=${String(k.left).padStart(4)}  <${k.tag}> "${k.text}"`);
    }
    await ctx.close();
  }
} finally {
  await browser.close();
}
