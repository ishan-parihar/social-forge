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
//
// Every route the app ships is here. The 2026-10-20 red-team found 16
// pages that had NEVER been rendered: /kanban, /comments, /dms, /search,
// /automation, /campaigns/[id], /posts/[id] and the eight /settings/*
// sub-pages. A page that is never rendered is a page whose defects are
// never found, so the list is the app's route table, not a sample of it.
//
// The two dynamic routes need an id. They are discovered from the API at
// run time (see RESOLVED_ROUTES) because the ids are per-instance.
const STATIC_ROUTES = flag(
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
    '/settings/profile',
    '/settings/notifications',
    '/settings/signatures',
    '/settings/webhooks',
    '/settings/rss',
    '/settings/mcp',
    '/settings/developer',
    '/kanban',
    '/comments',
    '/dms',
    '/search',
    '/automation',
  ].join(',')
).split(',');

// Routes that need a per-instance id, and where to get one.
const RESOLVED_ROUTES = [
  { path: '/campaigns/{id}', from: '/api/campaigns', pick: (rows) => rows[0]?.id },
  { path: '/posts/{id}', from: '/api/posts?limit=50', pick: (rows) => rows.posts?.[0]?.id },
];

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

  // Walk up for the first opaque background, COMPOSITING the translucent
  // layers found on the way.
  //
  // A translucent layer (chip washes, hover tints) composites over whatever
  // is behind it, so the effective backdrop is the nearest opaque ancestor
  // WITH the washes painted on top of it.
  //
  // The washes used to be SKIPPED rather than composited, which made every
  // reading on a tinted chip wrong: a `bg-warning/10` chip measured its
  // warning text against the surface behind it (4.95:1 in the light theme,
  // a pass) instead of against the wash it actually sits on (4.41:1, a
  // real failure the probe was blind to). The layers are collected
  // nearest-first and painted back-to-front, which is the order the
  // browser composites them in.
  const effectiveBg = (el) => {
    const layers = [];
    let node = el;
    while (node && node.nodeType === 1) {
      const c = parseColor(getComputedStyle(node).backgroundColor);
      if (c && c.a >= 0.95) {
        // The opaque backdrop, with the washes painted over it.
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
    // No opaque ancestor: the page background is the backdrop.
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

  // ── Craft checks (from the website-design skill's probe) ──
  //
  // The repo's own probe covers contrast, overflow, headings and overlap.
  // These four catch the defects a dashboard actually ships with: a word
  // split mid-letter by a track that is too narrow, a web font that was
  // never loaded (so the page silently renders in a fallback face), a
  // grid child squeezed to 0px, and a type system that has sprawled past
  // the point where it reads as a system.

  const sel = (e) => {
    if (!e || e.nodeType !== 1) return '?';
    let s = e.tagName.toLowerCase();
    if (e.id) return s + '#' + e.id;
    if (e.classList.length) s += '.' + [...e.classList].slice(0, 2).join('.');
    return s;
  };

  const shown = (e) => {
    if (e.checkVisibility && !e.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) return false;
    for (let n = e; n && n.nodeType === 1; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.display === 'none' || s.visibility === 'hidden' || +s.opacity === 0) return false;
    }
    return true;
  };

  // ── Document basics ──
  if (!document.documentElement.lang) {
    fails.push({ check: 'doc', detail: 'html has no lang attribute', selector: 'html' });
  }
  if (!document.title.trim()) {
    fails.push({ check: 'doc', detail: 'document has no <title>', selector: 'head' });
  }
  if (!document.querySelector('meta[name="viewport"]')) {
    fails.push({ check: 'doc', detail: 'no meta viewport', selector: 'head' });
  }

  // ── Collapsed grid/flex items ──
  // A card with no width is invisible, but the row around it still occupies
  // space, so the page looks like a gap rather than a bug.
  for (const g of document.querySelectorAll('*')) {
    const s = getComputedStyle(g);
    if (!/(grid|flex)/.test(s.display) || !shown(g)) continue;
    for (const c of g.children) {
      if (!shown(c) || !c.textContent.trim() || getComputedStyle(c).position === 'absolute') continue;
      if (c.getBoundingClientRect().width >= 2) continue;
      // A zero-width fill inside a progress track is INTENTIONAL: the bar
      // is at 0%, and the track itself is the visible element. The same is
      // true of a fill whose own width is set inline. Only a genuinely
      // squeezed CONTENT block is a defect, so require the element to be
      // laid out by the parent's track sizing rather than sized by itself.
      const cs = getComputedStyle(c);
      if (cs.width !== 'auto' || cs.inlineSize !== 'auto') continue;
      if (cs.flexBasis && cs.flexBasis !== 'auto' && cs.flexBasis !== '0px') continue;
      fails.push({
        check: 'collapsed-item',
        detail: `${sel(c)} ("${c.textContent.trim().slice(0, 28)}") is squeezed to 0px wide`,
        selector: sel(c),
      });
    }
  }

  // ── Broken words ──
  // A word of 4+ letters at 18px+ that renders on two lines means the track
  // is too narrow and the word is being split mid-letter.
  const textNodes = [];
  const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const t = walker.currentNode;
    if (!t.textContent.trim()) continue;
    const el = t.parentElement;
    if (!el || ['SCRIPT', 'STYLE', 'NOSCRIPT', 'TEMPLATE'].includes(el.tagName)) continue;
    if (!shown(el)) continue;
    textNodes.push({ el, tn: t, t: t.textContent.trim().slice(0, 32) });
  }
  for (const n of textNodes) {
    const fs = parseFloat(getComputedStyle(n.el).fontSize);
    if (fs < 18) continue;
    const re = /\S+/g;
    let m;
    while ((m = re.exec(n.tn.textContent))) {
      const w = m[0];
      if (w.length < 4 || /[-‐‑/]/.test(w)) continue;
      const r = document.createRange();
      r.setStart(n.tn, m.index);
      r.setEnd(n.tn, m.index + w.length);
      const rects = [...r.getClientRects()].filter((q) => q.width > 1);
      if (rects.length > 1 && Math.abs(rects[0].top - rects[rects.length - 1].top) > 2) {
        fails.push({
          check: 'broken-word',
          detail: `"${w}" splits across lines at ${fs}px in ${sel(n.el)} (${Math.round(n.el.getBoundingClientRect().width)}px wide)`,
          selector: sel(n.el),
        });
        break;
      }
    }
  }

  // ── Font fallback ──
  // A named face with no @font-face (or a @font-face that never loaded)
  // silently renders in the fallback, so the type voice the design asked for
  // is not the type voice on screen.
  const norm = (f) => f.trim().replace(/^["']|["']$/g, '').toLowerCase();
  // A family is LOADED when AT LEAST ONE of its faces is loaded.
  //
  // A webfont arrives as many faces - one per weight AND per unicode
  // subset (latin, cyrillic, greek). The browser loads a face only when
  // some text needs it, so a family that IS working still has faces
  // sitting at `unloaded`. The old check compared the family against
  // the set of LOADED faces, so Inter - whose latin face was loaded and
  // whose cyrillic face was not - read as "never loaded", and the
  // probe reported a font failure on every page.
  const loaded = new Set();
  for (const f of document.fonts) {
    if (f.status === 'loaded') loaded.add(norm(f.family));
  }
  const declared = new Set([...document.fonts].map((f) => norm(f.family)));
  const systemFace =
    /^(ui-|system-ui|-apple-system|blinkmacsystemfont|segoe ui|roboto$|helvetica|arial|georgia|times|menlo|monaco|consolas|courier|sfmono|sans-serif|serif|monospace|cursive|fantasy|math|emoji)/;
  const asked = new Map();
  for (const n of textNodes) {
    const f = norm(getComputedStyle(n.el).fontFamily.split(',')[0]);
    asked.set(f, (asked.get(f) || 0) + 1);
  }
  for (const [f, count] of asked) {
    if (systemFace.test(f) || loaded.has(f)) continue;
    fails.push({
      check: 'font-fallback',
      detail: `"${f}" is asked for by ${count} text nodes but ${declared.has(f) ? 'its @font-face never loaded' : 'no @font-face declares it'}`,
      selector: 'html',
    });
  }

  // ── Type and colour sprawl ──
  // Past a point the type system stops being a system. These are advisory:
  // a dense dashboard legitimately carries more sizes than a landing page.
  const sizes = new Set(textNodes.map((n) => Math.round(parseFloat(getComputedStyle(n.el).fontSize))));
  const colors = new Set(textNodes.map((n) => getComputedStyle(n.el).color));
  if (sizes.size > 16) {
    warns.push({
      check: 'type-sprawl',
      detail: `${sizes.size} distinct font sizes (${[...sizes].sort((a, b) => a - b).join(', ')})`,
    });
  }
  if (colors.size > 16) {
    warns.push({
      check: 'color-sprawl',
      detail: `${colors.size} distinct text colors`,
    });
  }

  // ── Tap targets (advisory) ──
  //
  // WCAG 2.2's target-size rule is about the CLICKABLE AREA, not the
  // control's own box. A checkbox inside its own <label> is clicked
  // through the label, so the label is the target - measuring the 16px
  // input instead reports a violation that does not exist. Same for an
  // <input> inside a <button> or a <label>.
  const targetRect = (el) => {
    const own = el.getBoundingClientRect();
    if (own.height >= 44 && own.width >= 44) return own;
    if (el.tagName === 'INPUT') {
      const wrapper = el.closest('label, button, [role="button"]');
      if (wrapper && wrapper !== el) {
        const r = wrapper.getBoundingClientRect();
        if (r.width > 0 && r.height > 0) return r;
      }
    }
    return own;
  };
  for (const el of document.querySelectorAll('button,a[href],[role="button"],input,select')) {
    // A HIDDEN control is not a tap target. The composer uses a
    // `hidden` file input for its uploads, and it measured 199x20 -
    // a violation that does not exist, because nothing can click it.
    // `offsetParent` is null for a `display: none` element, and it is
    // also null for `position: fixed`, so the check is on the computed
    // visibility and the rendered box instead.
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden') continue;
    const r = targetRect(el);
    if (r.width === 0 || r.height === 0) continue;
    if (r.height < 44 || r.width < 44) {
      warns.push({
        check: 'tap-target',
        detail: `${Math.round(r.width)}x${Math.round(r.height)} < 44x44 — "${(el.textContent || el.getAttribute('aria-label') || el.tagName).trim().slice(0, 30)}"`,
        cls: String(el.className || '').slice(0, 160),
        tag: el.tagName,
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

  const summary = { fails: [], warns: [], visits: 0, errors: [], notFound: [] };

  // The route list is resolved ONCE, before the theme loop, because the
  // dynamic routes need a session and the ids are the same in both themes.
  // Resolving per theme would double the API calls and could pick two
  // different ids if the data changed between them.
  let routes = [...STATIC_ROUTES];
  {
    const probeCtx = await browser.newContext({ ignoreHTTPSErrors: true });
    const probePage = await probeCtx.newPage();
    await login(probePage);
    for (const spec of RESOLVED_ROUTES) {
      const res = await probePage.request.get(`${BASE_URL}${spec.from}`);
      let id = null;
      try {
        const body = await res.json();
        id = spec.pick(body);
      } catch {
        id = null;
      }
      if (id) {
        routes.push(spec.path.replace('{id}', id));
      } else {
        summary.fails.push({
          route: spec.path,
          width: '',
          theme: '',
          check: 'unreachable-route',
          detail: `no id from ${spec.from} (HTTP ${res.status()}) - the page cannot be reached`,
        });
      }
    }
    await probeCtx.close();
  }
  const ROUTES = routes;

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
      // The STACK is the finding. "i.find is not a function" names the
      // symptom; the stack names the minified frame, which points back
      // at the source. Without it, a pageerror is a dead end.
      const stack = err && err.stack ? String(err.stack) : '';
      const frames = stack
        .split('\n')
        .slice(1, 4)
        .map((l) => l.trim())
        .filter(Boolean)
        .join(' | ');
      summary.fails.push({
        route: '(page lifecycle)',
        width: 0,
        theme,
        check: 'pageerror',
        detail: `${String(err.message || err).slice(0, 200)}${frames ? ` @ ${frames.slice(0, 400)}` : ''}`,
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
      } else if (res.status() === 404) {
        // A 404 is a defect the visitor can trigger: the page asked for
        // something the server does not serve. Record the URL, not just
        // the count, so the report names the broken call.
        const url = res.url();
        // The favicon is not part of the app surface.
        if (url.includes('/favicon')) return;
        summary.notFound.push({ theme, url });
      }
    });

    await login(page);

    for (const route of ROUTES) {
      for (const width of WIDTHS) {
        await page.setViewportSize({ width, height: 900 });
        const url = `${BASE_URL}${route}`;
        try {
          await page.goto(url, { waitUntil: 'load', timeout: 30_000 });
          // Wait for the APP to render, not for a fixed delay. The `load`
          // event fires before SvelteKit's client boot has run the
          // layout's auth probe, so a fixed `sleep(700)` measured the
          // page mid-boot and reported a blank document as "no <h1> on
          // the page" - on whichever route happened to be slow. The
          // render time ranged from 115ms to 2200ms across the routes,
          // so no fixed sleep is safe.
          await page
            .waitForSelector('h1', { state: 'attached', timeout: 10_000 })
            .catch(() => {
              // A route with genuinely no heading is a real finding, and
              // the audit below reports it. Waiting for the timeout is
              // the slow path but it keeps the report honest.
            });
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
  if (summary.notFound.length) {
    lines.push('');
    lines.push(`404 responses: ${summary.notFound.length}`);
    const seen = new Map();
    for (const n of summary.notFound) {
      const k = n.url;
      seen.set(k, (seen.get(k) || 0) + 1);
    }
    for (const [url, n] of seen) {
      lines.push(`  ? ${n}x ${url}`);
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
      // Break the advisory down by width, so "3002 warnings" becomes
      // "these are all the 375px drawer" or "these are everywhere".
      const byWidth = {};
      for (const w of items) {
        const k = `${w.width}px`;
        byWidth[k] = (byWidth[k] || 0) + 1;
      }
      lines.push(
        `      by width: ${Object.entries(byWidth)
          .map(([k, v]) => `${k}=${v}`)
          .join(' ')}`
      );
      // Group by the offending element so the same control is not counted
      // 78 times as 78 separate findings.
      const byControl = new Map();
      for (const w of items) {
        const label = (w.detail.match(/— "(.*)"/) || [0, "?"])[1];
        const key = `${label}`;
        byControl.set(key, (byControl.get(key) || 0) + 1);
      }
      const ranked = [...byControl.entries()].sort((a, b) => b[1] - a[1]);
      lines.push(`      by control (top 15 of ${ranked.length}):`);
      for (const [label, n] of ranked.slice(0, 15)) {
        lines.push(`        ${n}x ${label}`);
      }
      for (const w of items.slice(0, 6)) {
        lines.push(`      ${w.route} @${w.width}px — ${w.detail}`);
      }
    }

  const report = lines.join('\n');
  fs.writeFileSync(path.join(OUT_DIR, 'report.txt'), report);
  console.log(report);

  // The machine-readable form. The text report groups and TRUNCATES, so
  // it cannot answer "which element, on which route, at which width" -
  // and that is the only question a fix needs answered. Every warning
  // carries the route, the width, the theme and the element's own
  // class, so the fix can be located without re-running the browser.
  fs.writeFileSync(
    path.join(OUT_DIR, 'report.json'),
    JSON.stringify({ fails: summary.fails, warns: summary.warns }, null, 2)
  );

  process.exit(summary.fails.length === 0 ? 0 : 1);
}

main().catch((e) => {
  console.error(`probe: ${e.message}`);
  process.exit(2);
});
