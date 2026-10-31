// Reproduce the calendar overlap failures and report the two boxes.
//
// The probe's own overlap check already told us the TEXT ("buildinpublic"
// overlaps "13:30", "launch" overlaps "Overflow probe post 1 -"), but not
// which ELEMENTS carry them. A fix needs the elements, not the strings.
import { chromium } from 'playwright-core';

const EXE = process.env.PW_EXE || '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = process.env.SP_URL || 'http://localhost:6543';
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
  const res = { pairs: [], chipBoxes: [], moreBoxes: [] };

  // Every element that directly holds text, for the overlap pair walk.
  const textBoxes = Array.from(document.querySelectorAll('*')).filter((el) => {
    if (el.closest('.truncate')) return false;
    const cs = getComputedStyle(el);
    if (cs.position === 'absolute' || cs.position === 'fixed' || cs.position === 'relative') return false;
    if (cs.transform !== 'none' || cs.visibility === 'hidden') return false;
    return Array.from(el.childNodes).some((n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0);
  }).map((el) => ({ el, r: el.getBoundingClientRect() }))
    .filter((b) => b.r.width > 0 && b.r.height > 0)
    .filter((b) => b.r.bottom > 0 && b.r.top < window.innerHeight && b.r.right > 0 && b.r.left < window.innerWidth);

  for (let i = 0; i < textBoxes.length; i++) {
    for (let j = i + 1; j < textBoxes.length; j++) {
      const ea = textBoxes[i].el, eb = textBoxes[j].el;
      if (ea.contains(eb) || eb.contains(ea)) continue;
      const a = textBoxes[i].r, b = textBoxes[j].r;
      const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (ox > 2 && oy > 2) {
        const area = Math.min(a.width * a.height, b.width * b.height);
        if (ox * oy > area * 0.35) {
          res.pairs.push({
            a: describe(ea), b: describe(eb),
            ax: Math.round(a.x), ay: Math.round(a.y), aw: Math.round(a.width), ah: Math.round(a.height),
            bx: Math.round(b.x), by: Math.round(b.y), bw: Math.round(b.width), bh: Math.round(b.height),
          });
        }
      }
    }
  }

  // Per-chip geometry: is the text taller than its chip?
  for (const chip of document.querySelectorAll('[data-event-id]')) {
    const r = chip.getBoundingClientRect();
    if (r.width === 0) continue;
    let textH = 0;
    for (const t of chip.querySelectorAll('*')) {
      const tr = t.getBoundingClientRect();
      if (tr.height > textH) textH = tr.height;
    }
    res.chipBoxes.push({
      label: (chip.getAttribute('aria-label') || '').slice(0, 34),
      h: Math.round(r.height), w: Math.round(r.width),
      textH: Math.round(textH),
      scrollH: chip.scrollHeight, clientH: chip.clientHeight,
      tag: chip.className.toString().slice(0, 60),
    });
  }

  for (const btn of document.querySelectorAll('[data-cell-more]')) {
    const r = btn.getBoundingClientRect();
    if (r.width === 0) continue;
    res.moreBoxes.push({ h: Math.round(r.height), w: Math.round(r.width), text: (btn.textContent || '').slice(0, 12) });
  }

  function describe(el) {
    const path = [];
    let n = el;
    while (n && n !== document.body && path.length < 5) {
      path.push(n.tagName.toLowerCase() + (n.className ? '.' + String(n.className).split(' ').slice(0, 3).join('.') : ''));
      n = n.parentElement;
    }
    return { text: (el.textContent || '').replace(/\s+/g, ' ').trim().slice(0, 40), path: path.join(' < ') };
  }
  return res;
});

console.log('=== OVERLAP PAIRS ===');
console.log(JSON.stringify(out.pairs, null, 1));
console.log('=== CHIP BOXES (overflowing chips only) ===');
console.log(JSON.stringify(out.chipBoxes.filter((c) => c.textH > c.h), null, 1));
console.log('=== CHIP BOXES (all, count ' + out.chipBoxes.length + ') ===');
console.log(JSON.stringify(out.chipBoxes.slice(0, 8), null, 1));
console.log('=== MORE BUTTONS ===');
console.log(JSON.stringify(out.moreBoxes, null, 1));
await browser.close();
