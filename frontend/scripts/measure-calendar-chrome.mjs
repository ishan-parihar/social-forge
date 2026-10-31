// Measure the calendar's chrome: what is above the grid, row by row, in pixels.
//
// The plan says the chrome is 218px and should be ~60px, while the page's own
// comment says ~168px and the CSS fallback says 218px. Three numbers for one
// thing is the defect the plan names. This probe prints what is actually there,
// per element, at three widths, so the target is a measurement and not a guess.
import { chromium } from 'playwright-core';

const EXE = process.env.PW_EXE || '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = process.env.SP_URL || 'http://127.0.0.1:6543';
const CV = process.env.CV;

if (!CV) { console.log('SKIP: no CV'); process.exit(0); }

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });

function measure() {
  const grid = document.querySelector('.calendar-view-shell');
  if (!grid) return { error: 'no .calendar-view-shell' };
  const rect = (el) => {
    const r = el.getBoundingClientRect();
    return { top: Math.round(r.top), h: Math.round(r.height), w: Math.round(r.width) };
  };

  // Everything between the top bar and the grid, as a list of rows.
  const rows = [];
  let node = document.querySelector('.page-enter')?.firstElementChild;
  while (node && !node.classList.contains('calendar-view-shell')) {
    const r = node.getBoundingClientRect();
    rows.push({
      tag: node.tagName.toLowerCase(),
      cls: (node.className || '').toString().slice(0, 70),
      h: Math.round(r.height),
      top: Math.round(r.top),
      text: (node.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 40),
    });
    node = node.nextElementSibling;
  }

  const topbar = document.querySelector('header, .top-bar, [class*="sticky"][class*="top-0"]');
  return {
    grid: rect(grid),
    topbar: topbar ? rect(topbar) : null,
    cssVar: getComputedStyle(document.documentElement).getPropertyValue('--cal-shell-top').trim(),
    shellHeight: getComputedStyle(grid).height,
    rows,
    // The single number the plan wants to shrink.
    chromeAboveGrid: Math.round(grid.getBoundingClientRect().top),
  };
}

try {
  for (const width of [1440, 768, 390]) {
    const ctx = await browser.newContext({
      viewport: { width, height: 900 }, colorScheme: 'dark', ignoreHTTPSErrors: true,
    });
    await ctx.addCookies([{ name: 'sf_session', value: CV, domain: new URL(BASE).hostname, path: '/', httpOnly: false }]);
    const page = await ctx.newPage();
    await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
    await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
    await page.goto(BASE + '/calendar?display=week', { waitUntil: 'domcontentloaded', timeout: 40000 });
    await page.waitForTimeout(4500);
    // Dismiss anything the app raises on first load (the onboarding tour,
    // a whats-new card) - and press Escape a few times, because a modal can
    // raise another modal when it closes.
    for (let i = 0; i < 4; i++) { await page.keyboard.press('Escape'); await page.waitForTimeout(300); }
    await page.waitForTimeout(400);
    const m = await page.evaluate(measure);
    if (m.error) { console.log(`${width}px: ${m.error} — body text:`, await page.evaluate(() => document.body.innerText.slice(0, 200).replace(/\n+/g, ' | '))); await ctx.close(); continue; }
    console.log(`\n===== ${width}px =====`);
    console.log('chrome above grid:', m.chromeAboveGrid, 'px   (css --cal-shell-top:', m.cssVar || '(unset)', ')');
    console.log('shell height:', m.shellHeight);
    if (m.topbar) console.log('top bar:', JSON.stringify(m.topbar));
    for (const r of m.rows) {
      console.log(`  ${String(r.h).padStart(4)}px  top=${String(r.top).padStart(4)}  <${r.tag} class="${r.cls}"> "${r.text}"`);
    }
    await ctx.close();
  }
} finally {
  await browser.close();
}
