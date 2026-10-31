// C-1, the FULL group-move promise: drag a group member, answer "Move all",
// and confirm EVERY member shifts by the same offset.
//
// The audit's lesson is that a half-measured promise is worse than none: a
// dialog that appears and then silently moves only the dragged post would
// look identical at the API and be wrong in the data.
import { chromium } from 'playwright-core';

const EXE = process.env.PW_EXE || '/home/ishanp/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome';
const BASE = process.env.SP_URL || 'http://localhost:6543';
const CV = process.env.CV;

if (!CV) {
  console.log('SKIP: no CV (sf_session) in the environment');
  process.exit(0);
}

const browser = await chromium.launch({ executablePath: EXE, args: ['--no-sandbox'] });
const ctx = await browser.newContext({ viewport: { width: 1440, height: 900 }, colorScheme: 'dark' });
await ctx.addCookies([{ name: 'sf_session', value: CV, domain: 'localhost', path: '/', httpOnly: false }]);
const page = await ctx.newPage();

const calls = [];
page.on('request', (r) => {
  const m = /\/api\/posts\/([^/]+)\/date/.exec(r.url());
  if (m) calls.push({ id: m[1], body: r.postData() });
});

await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
await page.goto(BASE + '/calendar?display=week', { waitUntil: 'domcontentloaded', timeout: 40000 });
await page.waitForTimeout(4500);
await page.keyboard.press('Escape');
await page.waitForTimeout(500);

/** Both group members, as {id, cell}. */
async function members() {
  return page.evaluate(() => [...document.querySelectorAll('[data-event-id]')]
    .filter((c) => (c.getAttribute('aria-label') || '').includes('C1GROUP'))
    .map((c) => {
      const cell = c.closest('[data-drop-date][data-drop-hour]');
      return {
        id: c.getAttribute('data-event-id'),
        label: (c.getAttribute('aria-label') || '').slice(0, 22),
        cell: `${cell.getAttribute('data-drop-date')} ${cell.getAttribute('data-drop-hour')}`,
      };
    }));
}

const geom = await page.evaluate(() => {
  const scroller = [...document.querySelectorAll('*')].find((e) => {
    const s = getComputedStyle(e);
    return /(auto|scroll)/.test(s.overflowY)
      && e.scrollHeight > e.clientHeight + 40
      && (e.querySelector('[data-drop-date]') || e.className.includes('overflow-y-auto'));
  });
  const chip = [...document.querySelectorAll('[data-event-id]')].find(
    (c) => c.getAttribute('draggable') === 'true' && (c.getAttribute('aria-label') || '').includes('C1GROUP 0'));
  if (!chip) return { ok: false, why: 'no group chip' };
  const chipCell = chip.closest('[data-drop-date][data-drop-hour]');
  const chipDate = chipCell.getAttribute('data-drop-date');
  const chipHour = chipCell.getAttribute('data-drop-hour');
  scroller.scrollTop = 0;
  chipCell.scrollIntoView({ block: 'start' });
  const gridRect = scroller.getBoundingClientRect();
  const onScreen = (el) => {
    const r = el.getBoundingClientRect();
    return r.width > 0 && r.top >= gridRect.top - 2 && r.bottom <= gridRect.bottom + 2;
  };
  const visible = [...document.querySelectorAll('[data-drop-date][data-drop-hour]')].filter(
    (c) => c.getAttribute('data-drop-hour') === chipHour
      && !c.className.includes('calendar-cell-past')
      && onScreen(c));
  const target = visible.find((c) => c.getAttribute('data-drop-date') > chipDate)
    || visible.find((c) => c.getAttribute('data-drop-date') < chipDate);
  if (!target) return { ok: false, why: 'no target' };
  const a = chip.getBoundingClientRect();
  const b = target.getBoundingClientRect();
  return {
    ok: true,
    chipId: chip.getAttribute('data-event-id'),
    from: { x: Math.round(a.x + a.width / 2), y: Math.round(a.y + a.height / 2) },
    to: { x: Math.round(b.x + b.width / 2), y: Math.round(b.y + b.height / 2) },
    fromCell: `${chipDate} ${chipHour}`,
    toCell: `${target.getAttribute('data-drop-date')} ${target.getAttribute('data-drop-hour')}`,
  };
});

if (!geom.ok) {
  console.log('SKIP: ' + geom.why + ' (re-run scripts/seed_c1_fixtures.py)');
  await browser.close();
  process.exit(0);
}

const before = await members();
console.log('before:', JSON.stringify(before));

const beforeServer = await page.evaluate(async () => {
  const r = await fetch('/api/posts?limit=200', { credentials: 'include' });
  const d = await r.json();
  return (d.posts || d || []).filter((p) => (p.content || '').startsWith('C1GROUP'))
    .map((p) => ({ id: p.id, scheduled_at: p.scheduled_at }));
});
console.log('before (server):', JSON.stringify(beforeServer));

await page.mouse.move(geom.from.x, geom.from.y);
await page.mouse.down();
await page.mouse.move(geom.from.x + 60, geom.from.y + 10, { steps: 10 });
await page.mouse.move(geom.to.x, geom.to.y, { steps: 15 });
await page.waitForTimeout(300);
await page.mouse.up();
await page.waitForTimeout(2500);

const dialog = await page.evaluate(() => {
  const nodes = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')];
  const d = nodes[nodes.length - 1];
  return d ? (d.textContent || '').replace(/\s+/g, ' ').slice(0, 200) : null;
});
console.log('dialog:', dialog);

const answer = await page.evaluate(() => {
  const nodes = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')];
  const d = nodes[nodes.length - 1];
  if (!d) return 'no dialog';
  const btn = [...d.querySelectorAll('button')].find((b) => /move all/i.test(b.textContent || ''));
  if (!btn) return 'no Move all button: ' + (d.textContent || '').slice(0, 80);
  btn.click();
  return 'clicked';
});
console.log('answer:', answer);
await page.waitForTimeout(4500);

const after = await members();
// The DOM only shows what the current scroll renders, so a member can drop
// out of the rendered window without having failed to move. The AUTHORITATIVE
// answer is the server's own calendar payload.
const serverAfter = await page.evaluate(async () => {
  const r = await fetch('/api/posts?limit=200', { credentials: 'include' });
  const d = await r.json();
  return (d.posts || d || []).filter((p) => (p.content || '').startsWith('C1GROUP'))
    .map((p) => ({ id: p.id, content: p.content, scheduled_at: p.scheduled_at }));
});
console.log('after (dom):', JSON.stringify(after));
console.log('after (server):', JSON.stringify(serverAfter));
console.log('calls:', JSON.stringify(calls, null, 1));
await browser.close();

// ONE call with `move_group: true` moves the whole group server-side - a
// second call per member would be a second implementation of the rule. The
// correct assertion is: the call said move_group, and BOTH members shifted by
// the SAME offset.
//
// Measured against the SERVER's payload, not the DOM: the grid renders only
// the rows its current scroll covers, so "the member is gone from the DOM"
// is a statement about the scroll, not about the move.
const dayOf = (iso) => String(iso || '').slice(0, 10);
const offsets = beforeServer.map((b) => {
  const a = serverAfter.find((x) => x.id === b.id);
  if (!a) return NaN;
  return (new Date(dayOf(a.scheduled_at)).getTime()
    - new Date(dayOf(b.scheduled_at)).getTime()) / 86400000;
});
const sameOffset = offsets.length === 2
  && offsets.every((o) => Number.isFinite(o))
  && offsets[0] === offsets[1]
  && offsets[0] !== 0;
const saidMoveGroup = calls.some((c) => /"move_group":true/.test(c.body || ''));
const ok = /move all/i.test(dialog || '') && saidMoveGroup && sameOffset;

console.log(ok
  ? 'C-1 GROUP: PASS (dialog offered Move all; one call with move_group=true; both members shifted by the same offset)'
  : `C-1 GROUP: FAIL (dialog=${!!dialog}, saidMoveGroup=${saidMoveGroup}, offsets=${JSON.stringify(offsets)})`);
process.exit(ok ? 0 : 1);
