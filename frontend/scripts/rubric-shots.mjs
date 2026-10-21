// ─── Real-browser screenshots over browseros-neo's CDP port ───
//
// The visual gates measure mechanical properties (overflow, contrast,
// tap targets) - nobody in the gate suite JUDGES how a page looks.
// The website-design skill's ten-line look rubric and the CRAFT score
// need SCREENSHOTS of the rendered pages in a real browser, and the
// browseros-neo browser is the objective's named vehicle for exactly
// that.
//
// This script bridges the two: it connects playwright-core to the
// browseros-neo browser's CDP port (config: cdp=9171) and captures the
// core surfaces at the desktop width, into a directory a reviewing
// session can open. If the CDP port is down it exits 3 (BLOCKED) - the
// reviewer must launch browseros-neo first; screenshots are never
// faked.
//
// Run: node scripts/rubric-shots.mjs --url https://localhost:6544 --out /tmp/sf-rubric
// Then score each shot against the skill's ten-line rubric; this
// session's model cannot view images, so the RUBRIC runs wherever
// image viewing is available.

import { chromium } from 'playwright-core';
import { mkdirSync } from 'node:fs';

const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > -1 && process.argv[i + 1] ? process.argv[i + 1] : fallback;
};

const BASE_URL = arg('url', 'https://localhost:6544');
const OUT = arg('out', '/tmp/sf-rubric');
const CDP = arg('cdp', 'http://127.0.0.1:9171');

// The surfaces a founder judges: the three calendar views, the
// composer, analytics, research, and the dashboard.
const SHOTS = [
  { name: 'dashboard', path: '/' },
  { name: 'calendar-week', path: '/calendar?display=week' },
  { name: 'calendar-month', path: '/calendar?display=month' },
  { name: 'calendar-day', path: '/calendar?display=day' },
  { name: 'analytics', path: '/analytics' },
  { name: 'research', path: '/research' },
  { name: 'kanban', path: '/kanban' },
];

async function main() {
  let browser;
  try {
    browser = await chromium.connectOverCDP(CDP, { timeout: 5000 });
  } catch {
    console.error(`BLOCKED: browseros-neo CDP not reachable at ${CDP}. Launch browseros-neo first.`);
    process.exit(3);
  }

  const ctx = browser.contexts()[0] || (await browser.newContext());
  // The browseros session may already be authenticated; if not, the
  // shots capture the login page and the reviewer sees that honestly.
  const page = await ctx.newPage();
  mkdirSync(OUT, { recursive: true });

  let authenticated = true;
  for (const s of SHOTS) {
    await page.goto(`${BASE_URL}${s.path}`, { waitUntil: 'load', timeout: 15000 });
    await page.waitForTimeout(1500);
    const onLogin = page.url().includes('/login');
    if (onLogin) authenticated = false;
    await page.screenshot({ path: `${OUT}/${s.name}.png` });
    console.log(`${s.name}: ${onLogin ? 'LOGIN PAGE (not authenticated)' : 'ok'}`);
  }

  if (!authenticated) {
    console.log('\nNOTE: the browseros session is not logged in. Log in through the');
    console.log('browseros browser once (its cookie persists), then re-run.');
  }
  console.log(`\nshots in ${OUT} - score against the ten-line look rubric + CRAFT (a session with image viewing).`);
  await browser.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
