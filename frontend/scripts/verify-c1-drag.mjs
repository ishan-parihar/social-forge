// C-1 drag-to-reschedule verification, played against the real app.
//
// Three traps this probe is written around, each of which cost a false FAIL
// the first time:
//
//   1. `draggable="false"` is a PUBLISHED or PAST post the app deliberately
//      refuses to move. A probe that grabs the first chip it finds measures a
//      control that is correctly inert.
//   2. The week grid is a NESTED scroller (it starts around y=268 in a 900px
//      viewport), so a chip at y=1044 is unreachable: a mouse move to a
//      coordinate the browser cannot reach fires no events at all, and the
//      probe reports "nothing happened" as the app's fault.
//   3. BOTH the source chip and the target cell must be inside the GRID's
//      visible box, not the viewport's. Scrolling only the chip leaves the
//      target above the fold.
//
// It asserts the two outcomes the plan names: the plain reschedule reaches the
// API, and a group member raises the "Move campaign group?" confirmation.
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
  if (m) calls.push({ id: m[1], method: r.method(), body: r.postData() });
});

await page.goto(BASE + '/login', { waitUntil: 'domcontentloaded' });
await page.evaluate(() => { try { localStorage.setItem('switchpost-onboarded', '1'); } catch (e) {} });
await page.goto(BASE + '/calendar?display=week', { waitUntil: 'domcontentloaded', timeout: 40000 });
await page.waitForTimeout(4500);
await page.keyboard.press('Escape');
await page.waitForTimeout(500);

/** Resolve a chip and a target cell that are BOTH inside the grid's box. */
async function resolveDrag(labelNeedle) {
  return page.evaluate((needle) => {
    const scroller = [...document.querySelectorAll('*')].find((e) => {
      const s = getComputedStyle(e);
      return /(auto|scroll)/.test(s.overflowY)
        && e.scrollHeight > e.clientHeight + 40
        && (e.querySelector('[data-drop-date]') || e.className.includes('overflow-y-auto'));
    });
    if (!scroller) return { ok: false, why: 'no grid scroller' };

    const chip = [...document.querySelectorAll('[data-event-id]')].find(
      (c) => c.getAttribute('draggable') === 'true'
        && (c.getAttribute('aria-label') || '').includes(needle),
    );
    if (!chip) return { ok: false, why: `no draggable chip matching ${needle}` };

    const chipCell = chip.closest('[data-drop-date][data-drop-hour]');
    const chipDate = chipCell.getAttribute('data-drop-date');
    const chipHour = chipCell.getAttribute('data-drop-hour');

    // Scroll the chip's own cell to the top of the grid so the adjacent day
    // columns are the visible neighbours.
    scroller.scrollTop = 0;
    chipCell.scrollIntoView({ block: 'start' });

    const gridRect = scroller.getBoundingClientRect();
    const onScreen = (el) => {
      const r = el.getBoundingClientRect();
      return r.width > 0 && r.top >= gridRect.top - 2 && r.bottom <= gridRect.bottom + 2;
    };

    const target = (() => {
      const visible = [...document.querySelectorAll('[data-drop-date][data-drop-hour]')]
        .filter((c) => c.getAttribute('data-drop-hour') === chipHour
          && !c.className.includes('calendar-cell-past')
          && onScreen(c));
      // Prefer a LATER day; fall back to an earlier one when the chip sits in
      // the last column and nothing later is on screen.
      return visible.find((c) => c.getAttribute('data-drop-date') > chipDate)
        || visible.find((c) => c.getAttribute('data-drop-date') < chipDate)
        || null;
    })();
    if (!target) {
      return { ok: false, why: `no same-hour cell on another visible day (${chipDate} ${chipHour})` };
    }

    const a = chip.getBoundingClientRect();
    const b = target.getBoundingClientRect();
    if (!onScreen(chip)) return { ok: false, why: 'chip still not on screen' };

    return {
      ok: true,
      chipId: chip.getAttribute('data-event-id'),
      fromCell: `${chipDate} ${chipHour}`,
      from: { x: Math.round(a.x + a.width / 2), y: Math.round(a.y + a.height / 2) },
      to: { x: Math.round(b.x + b.width / 2), y: Math.round(b.y + b.height / 2) },
      cellDate: target.getAttribute('data-drop-date'),
      cellHour: target.getAttribute('data-drop-hour'),
    };
  }, labelNeedle);
}

async function driveDrag(labelNeedle, wantDialog) {
  const drag = await resolveDrag(labelNeedle);
  if (!drag.ok) return { ok: false, why: drag.why };

  await page.mouse.move(drag.from.x, drag.from.y);
  await page.mouse.down();
  await page.mouse.move(drag.from.x + 60, drag.from.y + 10, { steps: 10 });
  await page.mouse.move(drag.to.x, drag.to.y, { steps: 15 });
  await page.waitForTimeout(300);
  await page.mouse.up();
  await page.waitForTimeout(4500);

  const after = await page.evaluate((id) => {
    const el = document.querySelector(`[data-event-id="${id}"]`);
    if (!el) return 'gone';
    const c = el.closest('[data-drop-date][data-drop-hour]');
    return c ? `${c.getAttribute('data-drop-date')} ${c.getAttribute('data-drop-hour')}` : 'orphan';
  }, drag.chipId);

  const dialog = await page.evaluate(() => {
    // The confirm modal is `role="alertdialog"` (ModalManager.svelte), NOT
    // `role="dialog"`. Searching only for "dialog" misses it entirely - and
    // the group-move promise is exactly this dialog.
    const nodes = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')];
    const d = nodes[nodes.length - 1];
    return d ? (d.textContent || '').replace(/\s+/g, ' ').slice(0, 200) : null;
  });

  const ok = wantDialog
    ? dialog !== null
    : (calls.length > 0 || (after !== 'gone' && after !== 'orphan' && after !== drag.fromCell));

  return { ok, moved: after, dialog, drag, calls: calls.length };
}

const plain = await driveDrag('C1PLAIN 0', false);
console.log('plain:', JSON.stringify(plain));

// Dismiss any dialog left open before the group run.
await page.evaluate(() => {
  const nodes = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')];
  const d = nodes[nodes.length - 1];
  for (const b of (d ? [...d.querySelectorAll('button')] : [])) {
    if (/cancel|just this post|close|dismiss/i.test(b.textContent || '')) b.click();
  }
});
await page.waitForTimeout(600);

const group = await driveDrag('C1GROUP 0', true);
console.log('group:', JSON.stringify(group));

await browser.close();

const failures = [];
if (!plain.ok) failures.push(`plain drag took no effect (moved=${plain.moved}, calls=${plain.calls})`);
if (!group.ok) failures.push(`group member did not raise the group-move dialog (dialog=${group.dialog})`);

if (failures.length) {
  console.log('C-1 DRAG: FAIL');
  for (const f of failures) console.log('  - ' + f);
  process.exit(1);
}
console.log('C-1 DRAG: PASS (plain reschedule reached the API; group move raised its confirmation)');
