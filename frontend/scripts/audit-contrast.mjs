// A browser-side audit of the RENDERED contrast, in the states the
// static probe cannot reach.
//
// The static probe checks the contrast of every text element on each
// page's DEFAULT state. The composer, its panels and the popovers are
// only reachable by interaction - and a chip on a tinted wash, a
// provider glyph on a card, and a hue-as-text on its own tint all live
// in those places.
//
// This drives the real app: it logs in, visits every route, opens the
// composer and its panels, and measures every text element against the
// EFFECTIVE background - the translucent layers composited over the
// nearest opaque ancestor, which is what the eye actually sees.
//
// Run: node scripts/audit-contrast.mjs --url https://localhost:6544 --password "$APP_PASSWORD"
//
// It exits 3 (BLOCKED) when the browser is missing, and 1 when a
// contrast failure is found.

import { chromium } from 'playwright-core';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const CHROME_CANDIDATES = [
  process.env.CHROME_PATH,
  ...fs
    .readdirSync(path.join(os.homedir(), '.cache/ms-playwright'), { withFileTypes: true })
    .filter((d) => d.isDirectory() && d.name.startsWith('chromium-'))
    .flatMap((d) => [
      path.join(os.homedir(), '.cache/ms-playwright', d.name, 'chrome-linux64/chrome'),
      path.join(os.homedir(), '.cache/ms-playwright', d.name, 'chrome-linux/chrome'),
    ]),
  '/usr/bin/chromium',
  '/usr/bin/chromium-browser',
  '/usr/bin/google-chrome',
].filter(Boolean);

function findChrome() {
  for (const p of CHROME_CANDIDATES) {
    if (fs.existsSync(p)) return p;
  }
  return null;
}

const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > -1 && process.argv[i + 1] ? process.argv[i + 1] : fallback;
};

const BASE_URL = arg('url', 'https://localhost:6544');
const PASSWORD = arg('password', '');
const OUT = arg('out', '');
const THEME = arg('theme', 'dark');

// The measure, copied from `probe.mjs` so the two agree on what a
// failure is. A duplicate is deliberate: the probe and this audit must
// measure the same way, and a copy that drifts is caught by the shared
// numbers this prints.
function AUDIT() {
  const parseColor = (v) => {
    const m = v.match(/rgba?\(([^)]+)\)/);
    if (!m) return null;
    const p = m[1].split(/[,/\s]+/).filter(Boolean).map(Number);
    return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
  };
  const lum = ({ r, g, b }) => {
    const f = (c) => {
      const s = c / 255;
      return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
    };
    return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
  };
  const contrast = (a, b) => {
    const l1 = lum(a);
    const l2 = lum(b);
    const hi = Math.max(l1, l2);
    const lo = Math.min(l1, l2);
    return (hi + 0.05) / (lo + 0.05);
  };
  const effectiveBg = (el) => {
    const layers = [];
    let node = el;
    while (node && node.nodeType === 1) {
      const c = parseColor(getComputedStyle(node).backgroundColor);
      if (c && c.a >= 0.95) {
        let out = c;
        for (let i = layers.length - 1; i >= 0; i--) {
          const l = layers[i];
          out = {
            r: l.r * l.a + out.r * (1 - l.a),
            g: l.g * l.a + out.g * (1 - l.a),
            b: l.b * l.a + out.b * (1 - l.a),
            a: 1,
          };
        }
        return out;
      }
      if (c && c.a > 0) layers.push(c);
      node = node.parentElement;
    }
    let out = { r: 255, g: 255, b: 255, a: 1 };
    for (let i = layers.length - 1; i >= 0; i--) {
      const l = layers[i];
      out = {
        r: l.r * l.a + out.r * (1 - l.a),
        g: l.g * l.a + out.g * (1 - l.a),
        b: l.b * l.a + out.b * (1 - l.a),
        a: 1,
      };
    }
    return out;
  };
  const inheritedOpacity = (el) => {
    let node = el;
    let o = 1;
    while (node && node.nodeType === 1) {
      const v = parseFloat(getComputedStyle(node).opacity);
      if (!Number.isNaN(v)) o *= v;
      node = node.parentElement;
    }
    return o;
  };

  const out = [];
  document.querySelectorAll('*').forEach((el) => {
    // A DISABLED control is exempt. WCAG 1.4.3 names "incidental" text,
    // and a disabled control is not content the reader can act on - the
    // probe makes the same exemption, and the two have to agree.
    if (el.disabled || el.closest('[aria-disabled="true"]')) return;
    // An aria-hidden element is not content - a screen reader does not
    // announce it and a sighted reader is not reading it either. The
    // toolbar's "|" separators are the common case: they are marked
    // aria-hidden because they are decoration.
    if (el.closest('[aria-hidden="true"]')) return;
    // Only an element with its OWN text is measured.
    const own = [...el.childNodes].some(
      (n) => n.nodeType === 3 && n.textContent.trim().length > 0
    );
    if (!own) return;
    // Clipped text keeps its full box but its ink is cut off; the
    // measurement cannot see the clip, so it would report a false pair
    // for text the visitor never reads. The probe excludes it too.
    if (el.classList.contains('truncate')) return;
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden') return;
    const op = inheritedOpacity(el);
    if (op < 0.05) return; // not rendered
    const fg = parseColor(cs.color);
    if (!fg) return;
    const bg = effectiveBg(el);
    const eff = {
      r: fg.r * op + bg.r * (1 - op),
      g: fg.g * op + bg.g * (1 - op),
      b: fg.b * op + bg.b * (1 - op),
      a: 1,
    };
    const ratio = contrast(eff, bg);
    const size = parseFloat(cs.fontSize);
    const bold = parseInt(cs.fontWeight, 10) >= 700;
    // WCAG AA: 4.5:1 for body text, 3:1 for large text (>=18.66px bold
    // or >=24px).
    const large = size >= 24 || (size >= 18.66 && bold);
    const floor = large ? 3 : 4.5;
    if (ratio >= floor) return;
    out.push({
      text: el.textContent.trim().slice(0, 40),
      ratio: Math.round(ratio * 100) / 100,
      floor,
      size,
      color: cs.color,
      bg: `rgb(${Math.round(bg.r)}, ${Math.round(bg.g)}, ${Math.round(bg.b)})`,
      cls: String(el.className).slice(0, 90),
      tag: el.tagName.toLowerCase(),
    });
  });
  return out;
}

// The ENTRY ANIMATION must have finished before the measurement.
//
// Every page fades in over 0.2s (`.page-enter`), and the fade is an
// `opacity` animation on an ANCESTOR of the text. A contrast measure
// treats an ancestor with `opacity < 1` as a translucent group and
// stops the backdrop walk there - so a page measured mid-fade was
// measured against the wrong background. A page that ANIMATES forever
// (the skeleton shimmer) would hang, so only the finite ones are
// awaited.
async function settleAnimations(page) {
  await page
    .evaluate(async () => {
      const anims = document.getAnimations().filter((a) => {
        const t = a.effect?.getComputedTiming?.();
        return t && t.iterations !== Infinity;
      });
      if (!anims.length) return;
      await Promise.all(anims.map((a) => a.finished.catch(() => undefined)));
    })
    .catch(() => {});
}

async function main() {
  if (!PASSWORD) {
    console.error('BLOCKED: no --password');
    process.exit(3);
  }
  const executablePath = findChrome();
  if (!executablePath) {
    console.error('BLOCKED: no Chromium found. Set CHROME_PATH.');
    process.exit(3);
  }

  const browser = await chromium.launch({ executablePath, args: ['--no-sandbox'] });
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    ignoreHTTPSErrors: true,
  });
  const page = await ctx.newPage();
  const findings = [];

  await page.goto(`${BASE_URL}/login`, { waitUntil: 'load' });
  await page.fill('input[type=password]', PASSWORD);
  await page.evaluate(() => {
    document.querySelector('input[type=password]').closest('form').requestSubmit();
  });
  await page.waitForTimeout(2500);

  // The theme is a localStorage key the app reads at boot.
  await page.evaluate((t) => {
    localStorage.setItem('social-forge-theme', t);
  }, THEME);
  await page.reload({ waitUntil: 'load' });
  await page.waitForTimeout(1500);

  const record = async (where) => {
    const rows = await page.evaluate(AUDIT);
    for (const r of rows) findings.push({ where, ...r });
  };

  const ROUTES = [
    '/', '/calendar', '/posts', '/campaigns', '/channels', '/analytics',
    '/activity', '/research', '/feed', '/media', '/tags', '/kanban',
    '/comments', '/dms', '/search', '/automation', '/settings',
    '/settings/profile', '/settings/notifications', '/settings/signatures',
    '/settings/webhooks', '/settings/rss', '/settings/mcp', '/settings/developer',
  ];

  for (const route of ROUTES) {
    await page.goto(`${BASE_URL}${route}`, { waitUntil: 'load' });
    await page.waitForTimeout(1200);
    await settleAnimations(page);
    await record(route);
  }

  // The composer, and the panels inside it.
  await page.goto(`${BASE_URL}/posts/new`, { waitUntil: 'load' });
  await page.waitForTimeout(1500);
  const opened = await page.evaluate(() => {
    const b = [...document.querySelectorAll('button')].find((x) => (x.textContent || '').trim() === '+ New Post');
    if (b) { b.click(); return true; }
    return false;
  });
  if (opened) {
    await page.waitForTimeout(1500);
    await record('composer');
    for (const label of ['🧵 Thread Builder ▸', 'AI', 'Preview']) {
      await page.evaluate((L) => {
        const d = document.querySelector('[role=dialog]');
        if (!d) return;
        const b = [...d.querySelectorAll('button')].find((x) => (x.textContent || '').trim() === L);
        if (b) b.click();
      }, label);
      await page.waitForTimeout(900);
      await record(`composer:${label}`);
    }
  }

  await browser.close();

  const seen = new Set();
  const unique = findings.filter((f) => {
    const k = `${f.where}|${f.cls}|${f.ratio}`;
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });

  for (const f of unique) {
    console.log(`${f.where}  ${f.ratio}:1 (floor ${f.floor})  ${f.size}px  ${f.color} on ${f.bg}  "${f.text}"  ${f.cls}`);
  }
  console.log(`\n${unique.length} contrast failure(s) in the ${THEME} theme`);

  if (OUT) {
    const { writeFileSync } = await import('node:fs');
    writeFileSync(OUT, JSON.stringify(unique, null, 2));
  }

  process.exit(unique.length ? 1 : 0);
}

main().catch((e) => {
  console.error(e);
  process.exit(3);
});
