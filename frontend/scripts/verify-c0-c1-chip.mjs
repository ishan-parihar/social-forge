// C-0/C-1 acceptance: the chip's rendered box, at every width, in every view.
//
// The lesson that produced this probe: `min-height` with PADDING is not the
// tap target, and a probe that measures the WRAPPER measures the cell's
// `min-h-11`, not the chip. The chip's own box is what WCAG 2.2 2.5.8 measures.
//
// It also reports the overlap pairs, because the C-0 attempt introduced a
// real overlap defect (the tag chip and the time collided inside a clipped
// box) that only the probe could see. A change to the chip is not done until
// this reports both: the box >= 24px, and no clipped content.
import { chromium } from 'playwright-core';

const EXE = process.env.PW_EXE || '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = process.env.SP_URL || 'https://localhost:6543';
const CV = process.env.CV;

if (!CV) {
  console.log('SKIP: no CV (sf_session) in the environment');
  process.exit(0);
}

/** Count overlapping text pairs, using the probe's own rule. */
function overlapCount() {
  const boxes = Array.from(document.querySelectorAll('*')).filter((el) => {
    if (el.closest('.truncate')) return false;
    const cs = getComputedStyle(el);
    if (cs.position === 'absolute' || cs.position === 'fixed' || cs.position === 'relative') return false;
    if (cs.transform !== 'none' || cs.visibility === 'hidden') return false;
    return Array.from(el.childNodes).some((n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0);
  }).map((el) => ({ el, r: el.getBoundingClientRect() }))
    .filter((b) => b.r.width > 0 && b.r.height > 0)
    .filter((b) => b.r.bottom > 0 && b.r.top < window.innerHeight
      && b.r.right > 0 && b.r.left < window.innerWidth);

  let count = 0;
  const pairs = [];
  for (let i = 0; i < boxes.length; i++) {
    for (let j = i + 1; j < boxes.length; j++) {
      const ea = boxes[i].el;
      const eb = boxes[j].el;
      if (ea.contains(eb) || eb.contains(ea)) continue;
      const a = boxes[i].r;
      const b = boxes[j].r;
      const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (ox > 2 && oy > 2) {
        const area = Math.min(a.width * a.height, b.width * b.height);
        if (ox * oy > area * 0.35) {
          count += 1;
          if (pairs.length < 6) {
            pairs.push(`${ea.textContent.trim().slice(0, 18)} overlaps ${eb.textContent.trim().slice(0, 18)}`);
          }
        }
      }
    }
  }
  return { count, pairs };
}

/** Measure the chip's OWN box, the +N more button, and the cell. */
function measure() {
  const out = { chips: [], mores: [], cells: [] };
  for (const chip of document.querySelectorAll('[data-event-id]')) {
    const r = chip.getBoundingClientRect();
    if (r.width === 0) continue;
    const inner = chip.querySelector('.event-chip');
    const ir = inner ? inner.getBoundingClientRect() : null;
    out.chips.push({
      outer: Math.round(r.height),
      chip: ir ? Math.round(ir.height) : null,
      // The clipped-content signature: the chip's own overflow.
      clipped: inner ? inner.scrollHeight > inner.clientHeight + 1 : false,
    });
  }
  for (const btn of document.querySelectorAll('[data-cell-more]')) {
    const r = btn.getBoundingClientRect();
    if (r.width === 0) continue;
    out.mores.push({ h: Math.round(r.height), w: Math.round(r.width) });
  }
  const cell = document.querySelector('[data-drop-date]');
  if (cell) {
    const r = cell.getBoundingClientRect();
    out.cells.push({ h: Math.round(r.height), w: Math.round(r.width) });
  }
  return out;
}

const WIDTHS = [1440, 768, 390];
const VIEWS = ['week', 'day', 'month'];

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });
const failures = [];

try {
  for (const width of WIDTHS) {
    const ctx = await browser.newContext({
      viewport: { width, height: 900 }, colorScheme: 'dark', ignoreHTTPSErrors: true,
    });
    await ctx.addCookies([{ name: 'sf_session', value: CV, domain: 'localhost', path: '/', httpOnly: false }]);
    const page = await ctx.newPage();

    for (const view of VIEWS) {
      await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
      await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
      await page.goto(`${BASE}/calendar?display=${view}`, { waitUntil: 'domcontentloaded', timeout: 40000 });
      await page.waitForTimeout(4000);
      await page.keyboard.press('Escape');
      await page.waitForTimeout(400);

      const m = await page.evaluate(measure);
      const o = await page.evaluate(overlapCount);

      for (const c of m.chips) {
        if (c.chip !== null && c.chip < 24) {
          failures.push(`${view}@${width}: chip is ${c.chip}px tall (< 24)`);
        }
        if (c.clipped) {
          failures.push(`${view}@${width}: chip content is clipped`);
        }
      }
      for (const b of m.mores) {
        if (b.h < 24) failures.push(`${view}@${width}: +N more is ${b.h}px tall (< 24)`);
      }

      const chipH = [...new Set(m.chips.map((c) => c.chip))].join(',') || '-';
      const moreH = [...new Set(m.mores.map((b) => b.h))].join(',') || '-';
      const cellH = m.cells.map((c) => c.h).join(',') || '-';
      console.log(`${view}@${width}: chips=${m.chips.length} (h=${chipH})`
        + ` mores=${m.mores.length} (h=${moreH}) cellH=${cellH} overlaps=${o.count}`);
      if (o.count) console.log('    ' + o.pairs.join(' | '));
    }
    await ctx.close();
  }
} finally {
  await browser.close();
}

if (failures.length) {
  console.log('C-0/C-1 CHIP: FAIL');
  for (const f of [...new Set(failures)]) console.log('  - ' + f);
  process.exit(1);
}
console.log('C-0/C-1 CHIP: PASS (chip box >= 24px everywhere, no clipped content, +N more >= 24px)');
