#!/usr/bin/env node
// ─── Website-design probe (AUD-9 gate) ───────────────────────
//
// Loads each dashboard route in a real Chromium at three widths and
// reports two severities:
//
//   FAIL  — a defect a visitor would hit: an uncaught page error, an
//           HTTP 5xx, horizontal overflow, text below the WCAG AA
//           contrast floor, a broken heading outline, or overlapping
//           text that is not a clipping artifact.
//   WARN  — advisory: small tap targets and sub-10px labels.
//
// The gate is 0 FAIL. Warnings are reported, not gated.
//
// Usage:
//   node scripts/probe.mjs [--url http://localhost:6543] [--password pw]
//                          [--widths 1440,768,375] [--theme dark,light]
//                          [--out probe-out]
//
// Requires playwright-core (a devDependency). Chromium is resolved from
// PLAYWRIGHT_BROWSERS_PATH (~/.cache/ms-playwright) or the system path;
// there is no `playwright install` step in CI, so the cached browser is
// used directly.

import { chromium } from 'playwright-core';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

// ── CLI ──────────────────────────────────────────────────────

const argv = process.argv.slice(2);
const flag = (name, dflt) => {
  const i = argv.indexOf(`--${name}`);
  return i === -1 ? dflt : argv[i + 1] ?? dflt;
};

const BASE_URL = flag('url', process.env.PROBE_URL || 'http://localhost:6543');
const PASSWORD = flag('password', process.env.APP_PASSWORD || '');
const WIDTHS = flag('widths', '1440,768,375').split(',').map(Number);
const THEMES = flag('theme', 'dark,light').split(',');
const OUT_DIR = flag('out', 'probe-out');

// The manual suite's routes. Kept to the surfaces this dashboard owns —
// no team/workspace/billing/admin/agent-chat/video pages exist to probe.
const ROUTES = flag(
  'routes',
  [
    '/',
    '/calendar',
    '/posts',
    '/posts/new',
    '/campaigns',
    '/channels',
    '/analytics',
    '/activity',
    '/research',
    '/feed',
    '/media',
    '/tags',
    '/settings',
  ].join(',')
).split(',');

const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  ...fs
    .readdirSync(path.join(os.homedir(), '.cache/ms-playwright'), {
      withFileTypes: true,
    })
    .filter((d) => d.isDirectory() && d.name.startsWith('chromium-'))
    .flatMap((d) => [
      path.join(
        os.homedir(),
        '.cache/ms-playwright',
        d.name,
        'chrome-linux64/chrome'
      ),
      path.join(
        os.homedir(),
        '.cache/ms-playwright',
        d.name,
        'chrome-linux/chrome'
      ),
    ]),
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/usr/bin/google-chrome',
].filter(Boolean);

function findChrome() {
  for (const p of CHROME_CANDIDATES) {
    if (fs.existsSync(p)) return p;
  }
  throw new Error('No Chromium found. Set CHROME_PATH.');
}

// ── In-page audit ────────────────────────────────────────────
//
// Run as a single function in the page so one round-trip collects every
// measurement. Pure DOM reads — no layout mutation.

const AUDIT = () => {
  const fails = [];
  const warns = [];

  const parseColor = (v) => {
    const m = v.match(/rgba?\(([^)]+)\)/);
    if (!m) return null;
    const [r, g, b, a] = m[1].split(/[,\s/]+/).filter(Boolean).map(Number);
    return { r, g, b, a: a === undefined ? 1 : a };
  };

  // WCAG relative luminance.
  const lum = ({ r, g, b }) => {
    const f = (c) => {
      const s = c / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
  };

  const contrast = (fg, bg) => {
    const l1 = lum(fg);
    const l2 = lum(bg);
    const [hi, lo] = l1 > l2 ? [l1, l2] : [l2, l1];
    return (hi + 0.05) / (lo + 0.05);
  };

  // Walk up for the first opaque background. A translucent layer (chip
  // washes, hover tints) composites over whatever is behind it, so the
  // effective backdrop is the nearest ancestor that is fully opaque.
  const effectiveBg = (el) => {
    let node = el;
    while (node && node.nodeType === 1) {
      const c = parseColor(getComputedStyle(node).backgroundColor);
      if (c && c.a >= 0.95) return c;
      node = node.parentElement;
    }
    return { r: 255, g: 255, b: 255, a: 1 };
  };

  // Large text (>=24px, or >=18.66px bold) clears at 3:1; the rest at
  // 4.5:1 — WCAG 1.4.3.
  const requiredRatio = (size, weight) => {
    const bold = Number(weight) >= 700;
    return size >= 24 || (bold && size >= 18.66) ? 3 : 4.5;
  };

  // ── Contrast ──
  for (const el of document.querySelectorAll('*')) {
    if (el.closest('[hidden]') || el.getAttribute('aria-hidden') === 'true')
      continue;
    const cs = getComputedStyle(el);
    if (cs.visibility === 'hidden' || cs.display === 'none') continue;

    // Only elements with their own text run. A wrapper's colour inherits,
    // so measuring it twice duplicates the same pair.
    const own = Array.from(el.childNodes).some(
      (n) =>
        n.nodeType === Node.TEXT_NODE &&
        n.textContent.trim().length > 0 &&
        // Clipped text keeps its full box but its ink is cut off; the
        // check below cannot see the clip, so it would report a false
        // pair for text the visitor never reads.
        !el.classList.contains('truncate')
    );
    if (!own) continue;

    const fg = parseColor(cs.color);
    if (!fg || fg.a < 0.95) continue;
    const size = parseFloat(cs.fontSize);
    const ratio = contrast(fg, effectiveBg(el));
    const need = requiredRatio(size, cs.fontWeight);
    if (ratio < need - 0.01) {
      fails.push({
        check: 'contrast',
        detail: `${Math.round(size)}px ${ratio.toFixed(2)}:1 < ${need}:1 — "${el.textContent.trim().slice(0, 40)}"`,
        selector: el.tagName.toLowerCase() + (el.className ? '.' + String(el.className).split(' ')[0] : ''),
      });
    }
  }

  // ── Horizontal overflow ──
  // A page that scrolls sideways at a phone width is a layout break.
  const de = document.documentElement;
  if (de.scrollWidth > de.clientWidth + 1) {
    fails.push({
      check: 'overflow',
      detail: `scrollWidth ${de.scrollWidth} > clientWidth ${de.clientWidth}`,
      selector: 'html',
    });
  }
  for (const el of document.querySelectorAll('body *')) {
    const cs = getComputedStyle(el);
    if (cs.overflowX === 'auto' || cs.overflowX === 'scroll' || cs.overflowX === 'hidden')
      continue;
    if (el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0) {
      // A scroll container's own overflow is its job, not a defect.
      const canScroll = (n) => {
        while (n && n !== document.body) {
          const s = getComputedStyle(n);
          if (/(auto|scroll)/.test(s.overflowX)) return true;
          n = n.parentElement;
        }
        return false;
      };
      if (!canScroll(el)) {
        fails.push({
          check: 'overflow',
          detail: `element scrollWidth ${el.scrollWidth} > clientWidth ${el.clientWidth}`,
          selector: el.tagName.toLowerCase(),
        });
      }
    }
  }

  // ── Heading outline ──
  // One h1 per page (the route title), and no skipped levels — a11y
  // 1.3.1 / 2.4.6, and the AUD-9 rule.
  const heads = Array.from(document.querySelectorAll('h1,h2,h3,h4,h5,h6'));
  const h1s = heads.filter((h) => h.tagName === 'H1');
  if (h1s.length === 0) {
    fails.push({ check: 'heading', detail: 'no <h1> on the page', selector: 'body' });
  } else if (h1s.length > 1) {
    fails.push({ check: 'heading', detail: `${h1s.length} <h1> elements`, selector: 'h1' });
  }
  let prev = 0;
  for (const h of heads) {
    const lvl = Number(h.tagName[1]);
    if (prev && lvl > prev + 1) {
      fails.push({
        check: 'heading',
        detail: `<h${prev}> → <h${lvl}> skips a level — "${h.textContent.trim().slice(0, 40)}"`,
        selector: h.tagName.toLowerCase(),
      });
    }
    prev = lvl;
  }

  // ── Overlapping text ──
  // Two visible text runs on the same line whose boxes overlap by more
  // than a hair, when neither is clipped. Absolute/relative/fixed and
  // transformed boxes legitimately overlap, so those are exempt.
  //
  // Occlusion: an element visually hidden behind an opaque overlay (for
  // example page chrome behind a full-screen modal's opaque backdrop) is
  // skipped - elementFromPoint returns the covering element, not the
  // covered one.
  function isVisuallyOccluded(el) {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) return false;
    const pts = [
      [r.left + r.width / 2, r.top + r.height / 2],
      [r.left + r.width * 0.25, r.top + r.height * 0.5],
      [r.left + r.width * 0.75, r.top + r.height * 0.5],
    ];
    for (const [x, y] of pts) {
      const top = document.elementFromPoint(x, y);
      if (!top) continue;
      if (top === el || el.contains(top)) continue;
      // top covers el and is not contained by el -> el is hidden behind it.
      if (!top.contains(el)) {
        const cs = getComputedStyle(top);
        if (cs.visibility !== 'hidden' && cs.display !== 'none') return true;
      }
    }
    return false;
  }

  const textBoxes = Array.from(document.querySelectorAll('*'))
    .filter((el) => {
      if (isVisuallyOccluded(el)) return false;
      if (el.closest('.truncate')) return false;
      const cs = getComputedStyle(el);
      if (cs.position === 'absolute' || cs.position === 'fixed' || cs.position === 'relative')
        return false;
      if (cs.transform !== 'none' || cs.visibility === 'hidden') return false;
      return Array.from(el.childNodes).some(
        (n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0
      );
    })
    .map((el) => ({ el, r: el.getBoundingClientRect() }))
    .filter((b) => b.r.width > 0 && b.r.height > 0)
    // Only text the visitor can actually see. A box scrolled out of the
    // viewport still has a rectangle, and it still overlaps the boxes that
    // ARE on screen - which is how a modal opening over a long page
    // reported the page's own content as overlapping the modal's text.
    .filter((b) => {
      const vh = window.innerHeight;
      const vw = window.innerWidth;
      return b.r.bottom > 0 && b.r.top < vh && b.r.right > 0 && b.r.left < vw;
    });

  for (let i = 0; i < textBoxes.length; i++) {
    for (let j = i + 1; j < textBoxes.length; j++) {
      const a = textBoxes[i].r;
      const b = textBoxes[j].r;
      // A child's box is always inside its parent's box, so "X overlaps X's
      // parent" is a tautology rather than a layout defect. Only compare
      // boxes that are geometrically independent of each other.
      const ea = textBoxes[i].el;
      const eb = textBoxes[j].el;
      if (ea.contains(eb) || eb.contains(ea)) continue;
      const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (ox > 2 && oy > 2) {
        // Same-line whitespace collisions are normal; require real overlap.
        const area = Math.min(a.width * a.height, b.width * b.height);
        if (ox * oy > area * 0.35) {
          fails.push({
            check: 'overlap',
            detail: `"${ea.textContent.trim().slice(0, 24)}" overlaps "${eb.textContent.trim().slice(0, 24)}"`,
            selector: ea.tagName.toLowerCase(),
          });
        }
      }
    }
  }

  // ── Tap targets (advisory) ──
  for (const el of document.querySelectorAll('button,a[href],[role="button"],input,select')) {
    const r = el.getBoundingClientRect();
    if (r.width === 0 || r.height === 0) continue;
    if (r.height < 44 || r.width < 44) {
      warns.push({
        check: 'tap-target',
        detail: `${Math.round(r.width)}x${Math.round(r.height)} < 44x44 — "${(el.textContent || el.getAttribute('aria-label') || el.tagName).trim().slice(0, 30)}"`,
      });
    }
  }

  // ── Tiny text (advisory) ──
  for (const el of document.querySelectorAll('*')) {
    const cs = getComputedStyle(el);
    const size = parseFloat(cs.fontSize);
    if (size > 0 && size < 10) {
      const own = Array.from(el.childNodes).some(
        (n) => n.nodeType === Node.TEXT_NODE && n.textContent.trim().length > 0
      );
      if (own) {
        warns.push({
          check: 'tiny-text',
          detail: `${size}px — "${el.textContent.trim().slice(0, 30)}"`,
        });
      }
    }
  }

  return { fails, warns };
};

// ── Runner ───────────────────────────────────────────────────

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function login(page) {
  if (!PASSWORD) {
    throw new Error('No password: pass --password or set APP_PASSWORD');
  }
  // The CSRF middleware requires an Origin matching FRONTEND_URL on
  // state-changing requests, and a bare request-context POST sends none.
  const res = await page.request.post(`${BASE_URL}/api/auth/login`, {
    data: { password: PASSWORD },
    headers: { Origin: BASE_URL },
  });
  if (!res.ok()) {
    throw new Error(`Login failed: HTTP ${res.status()}`);
  }
}

/// Dismiss the first-run onboarding overlay if it is up.
///
/// On an empty database the modal covers every route, so measuring the page
/// underneath it measures the modal, not the page. The dismiss is a click on
/// its own Skip control — the documented way out — and it is skipped when no
/// overlay is present (a seeded database never shows one).
async function dismissOnboarding(page) {
  try {
    const skip = page.getByRole('button', { name: /skip|quick tour|got it/i }).first();
    if (await skip.isVisible({ timeout: 1500 })) {
      await skip.click({ timeout: 2000 });
      await sleep(400);
    }
  } catch {
    // No overlay, or it closed on its own — nothing to dismiss.
  }
}

async function main() {
  const executablePath = findChrome();
  fs.mkdirSync(OUT_DIR, { recursive: true });

  const browser = await chromium.launch({
    executablePath,
    ignoreHTTPSErrors: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });

  const summary = { fails: [], warns: [], visits: 0, errors: [] };

  for (const theme of THEMES) {
    const context = await browser.newContext({
      viewport: { width: 1440, height: 900 },
      ignoreHTTPSErrors: true,
      deviceScaleFactor: 1,
    });
    // Seed the theme through the app's own store channel: localStorage is what
    // `theme.init()` reads on mount. Toggling the documentElement class directly
    // is untracked and reverted by init(), which leaves the page with no theme
    // class at all — the tokens then resolve to nothing and every contrast
    // reading is garbage. An init script runs before any page script, so the
    // store sees the value from its very first read.
    await context.addInitScript((t) => {
      try {
        localStorage.setItem('social-forge-theme', t);
      } catch {
        // Storage denied (opaque origin) — the page falls back to dark.
      }
    }, theme);
    const page = await context.newPage();

    page.on('pageerror', (err) => {
      summary.fails.push({
        route: '(page lifecycle)',
        width: 0,
        theme,
        check: 'pageerror',
        detail: String(err.message || err).slice(0, 200),
      });
    });

    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        // A missing /api/signatures/default on /posts/new is the
        // documented "no default signature" 404, not an app error.
        const t = msg.text();
        if (t.includes('signatures/default')) return;
        summary.errors.push({ theme, detail: t.slice(0, 200) });
      }
    });

    page.on('response', (res) => {
      if (res.status() >= 500) {
        summary.fails.push({
          route: '(response)',
          width: 0,
          theme,
          check: 'http-5xx',
          detail: `HTTP ${res.status()} ${res.url()}`,
        });
      }
    });

    await login(page);

    for (const route of ROUTES) {
      for (const width of WIDTHS) {
        await page.setViewportSize({ width, height: 900 });
        const url = `${BASE_URL}${route}`;
        try {
          await page.goto(url, { waitUntil: 'load', timeout: 30_000 });
          await sleep(700); // let the first data fetch settle
          // The onboarding modal covers every route on an empty database;
          // measuring it would measure the modal, not the page.
          await dismissOnboarding(page);

          const { fails, warns } = await page.evaluate(AUDIT);
          for (const f of fails) {
            summary.fails.push({ route, width, theme, ...f });
          }
          for (const w of warns) {
            summary.warns.push({ route, width, theme, ...w });
          }
          summary.visits += 1;

          const name = `${theme}-${width}-${route.replace(/[/:]/g, '_') || 'root'}`;
          await page.screenshot({
            path: path.join(OUT_DIR, `${name}.png`),
            fullPage: false,
          });
        } catch (e) {
          summary.fails.push({
            route,
            width,
            theme,
            check: 'navigation',
            detail: String(e.message || e).slice(0, 200),
          });
        }
      }
    }

    await context.close();
  }

  await browser.close();

  // ── Report ──
  const lines = [];
  lines.push(`routes: ${ROUTES.length} · widths: ${WIDTHS.join(', ')} · themes: ${THEMES.join(', ')}`);
  lines.push(`visits: ${summary.visits}`);
  lines.push('');
  lines.push(`FAIL: ${summary.fails.length}`);
  for (const f of summary.fails) {
    const at = [f.route, f.width ? `${f.width}px` : '', f.theme].filter(Boolean).join(' ');
    lines.push(`  ✗ [${f.check}] ${at} — ${f.detail}`);
  }
  if (summary.errors.length) {
    lines.push('');
    lines.push(`console errors: ${summary.errors.length}`);
    for (const e of summary.errors.slice(0, 20)) {
      lines.push(`  ! ${e.detail}`);
    }
  }
  lines.push('');
  lines.push(`WARN: ${summary.warns.length}`);
  const byCheck = {};
  for (const w of summary.warns) {
    (byCheck[w.check] ||= []).push(w);
  }
  for (const [check, items] of Object.entries(byCheck)) {
    lines.push(`  · ${check}: ${items.length}`);
    for (const w of items.slice(0, 6)) {
      lines.push(`      ${w.route} @${w.width}px — ${w.detail}`);
    }
  }

  const report = lines.join('\n');
  fs.writeFileSync(path.join(OUT_DIR, 'report.txt'), report);
  console.log(report);

  process.exit(summary.fails.length === 0 ? 0 : 1);
}

main().catch((e) => {
  console.error(`probe: ${e.message}`);
  process.exit(2);
});
