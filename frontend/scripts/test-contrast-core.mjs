// ─── The CONTRAST_CORE value test ─────────────────────────────
//
// The shared contrast measurement (contrast-core.mjs) has ONE
// implementation but ZERO pinning: a channel-alpha typo in copy #3
// produced negative luminances and impossible ratios (-1.80:1) and
// every syntax check passed, because the defect was in the
// ARITHMETIC, not the syntax. This test pins the arithmetic itself:
// a wash-over-opaque case, an opacity-group case, and the two pure
// helpers, against hand-computed values.
//
// It runs the REAL core in a real Chromium page over synthetic DOM,
// the same serialization path the gates use (new Function), so it
// also compiles the assembly the way the audits run it.
//
// Run: node scripts/test-contrast-core.mjs
// Exits 3 (BLOCKED) without a browser, 1 on any value mismatch.

import { chromium } from 'playwright-core';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { CONTRAST_CORE } from './contrast-core.mjs';

const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  ...fs.readdirSync(path.join(os.homedir(), '.cache/ms-playwright'), { withFileTypes: true })
    .filter((d) => d.isDirectory() && d.name.startsWith('chromium-'))
    .flatMap((d) => [
      path.join(os.homedir(), '.cache/ms-playwright', d.name, 'chrome-linux64/chrome'),
      path.join(os.homedir(), '.cache/ms-playwright', d.name, 'chrome-linux/chrome'),
    ]),
].filter(Boolean);
const CHROME = CHROME_CANDIDATES.find((p) => fs.existsSync(p));
if (!CHROME) { console.error('BLOCKED: no Chromium found'); process.exit(3); }

// The page-side probe: build synthetic DOM, run the core's effectiveBg
// on it, and return the three pinned measurements. The core is compiled
// with new Function exactly the way the gates assemble their audits.
const PROBE = new Function(`${CONTRAST_CORE}
  // synthetic DOM must be ATTACHED: getComputedStyle on a detached node
  // returns empty strings, so effectiveBg would see no layers at all.
  const holder = document.createElement('div');
  holder.style.display = 'none'; // no layout, but computed styles resolve
  document.body.appendChild(holder);
  const attach = (html) => {
    holder.innerHTML = html;
    return holder.querySelector('.target');
  };
  return {
    // wash-over-opaque: 10% amber wash over the dark surface.
    // hand: r = 234*0.1 + 19*0.9 = 40.5, g = 179*0.1 + 23*0.9 = 38.6, b = 8*0.1 + 32*0.9 = 29.6
    wash: (() => {
      const text = attach('<div style="background-color: rgb(19, 23, 32)"><div style="background-color: rgba(234, 179, 8, 0.1)"><span class="target">x</span></div></div>');
      const bg = effectiveBg(text);
      return [Math.round(bg.r*10)/10, Math.round(bg.g*10)/10, Math.round(bg.b*10)/10];
    })(),
    // opacity-group: a parent with opacity 0.5 and its own background
    // fades that background into the backdrop behind the GROUP.
    // hand: the walk breaks AT the opacity ancestor pushing its own bg at
    // a = 1.0 * 0.5 = 0.5, then composites over the default white backdrop:
    // r = 0*0.5 + 255*0.5 = 127.5
    group: (() => {
      const text = attach('<div style="background-color: rgb(255, 255, 255)"><div style="background-color: rgb(0, 0, 0); opacity: 0.5"><span class="target">x</span></div></div>');
      const bg = effectiveBg(text);
      return [Math.round(bg.r*10)/10, Math.round(bg.g*10)/10, Math.round(bg.b*10)/10];
    })(),
    // the pure ratio: white on black is 21:1 by definition.
    ratio: (() => {
      const r = contrast({r:255,g:255,b:255}, {r:0,g:0,b:0});
      return Math.round(r*100)/100;
    })(),
    // the muted ink on the dark surface the chips actually use: 7.21:1.
    muted: (() => {
      const r = contrast({r:154,g:165,b:184}, {r:19,g:23,b:32});
      return Math.round(r*100)/100;
    })(),
  };
`);

const browser = await chromium.launch({ executablePath: CHROME });
const page = await browser.newPage();
await page.goto('about:blank');
const got = await page.evaluate(PROBE);
await browser.close();

let failed = 0;
const eq = (name, actual, expected) => {
  const ok = JSON.stringify(actual) === JSON.stringify(expected);
  if (!ok) { console.log(`FAIL ${name}: got ${JSON.stringify(actual)}, want ${JSON.stringify(expected)}`); failed++; }
  else console.log(`ok   ${name} = ${JSON.stringify(actual)}`);
};
eq('wash-over-opaque effBg', got.wash, [40.5, 38.6, 29.6]);
eq('opacity-group effBg', got.group, [127.5, 127.5, 127.5]);
eq('contrast(white,black)', got.ratio, 21);
eq('contrast(muted,surface)', got.muted, 7.21);
console.log(failed ? `${failed} value mismatch(es)` : 'contrast-core pinned: all values match');
process.exit(failed ? 1 : 0);
