// The dialog contract, checked over the real app.
//
// Every dialog in the app must:
//
//   - have an accessible NAME (a label, or an id pointing at a heading)
//   - carry aria-modal="true", so the rest of the page is inert
//   - NOT be inside an aria-hidden subtree - that makes it invisible to
//     a screen reader AND stops the shared focus trap from finding a
//     focusable child, so the focus never enters it
//   - TAKE the focus when it opens
//   - close on Escape
//
// The last two are the ones that catch the real defect: a dialog that
// opens but never receives the focus is a dialog the founder cannot
// type into.
//
// Run: node scripts/audit-dialogs.mjs --url https://localhost:6544 --password "$APP_PASSWORD"
//
// It exits 3 (BLOCKED) when the browser is missing, and 1 when a
// dialog breaks the contract.

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

// The trigger is matched by its accessible name, exactly as a screen
// reader would read it.
const DIALOGS = [
  { route: '/automation', trigger: '+ New Rule', expect: 'automation-rule' },
  { route: '/campaigns', trigger: '+ New Campaign', expect: 'create-campaign' },
  { route: '/comments', trigger: '↩', expect: 'comment-reply' },
  { route: '/search', trigger: 'Save', expect: 'save-search' },
  { route: '/settings/webhooks', trigger: '+ Add Webhook', expect: 'webhook' },
  { route: '/posts/new', trigger: '+ New Post', expect: 'composer' },
];

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

  await page.goto(`${BASE_URL}/login`, { waitUntil: 'load' });
  await page.fill('input[type=password]', PASSWORD);
  await page.evaluate(() => {
    document.querySelector('input[type=password]').closest('form').requestSubmit();
  });
  await page.waitForTimeout(2500);

  const findings = [];

  for (const d of DIALOGS) {
    await page.goto(`${BASE_URL}${d.route}`, { waitUntil: 'load' });
    await page.waitForTimeout(1500);

    // A trigger can be disabled until the page has something to act on
    // (the search page's "Save" needs a query). Fill the obvious input
    // first so the trigger is live.
    await page.evaluate(() => {
      const inp = [...document.querySelectorAll('input')].find(
        (i) => /search|query/i.test(i.placeholder || '') && !i.disabled
      );
      if (!inp) return;
      const set = Object.getOwnPropertyDescriptor(
        window.HTMLInputElement.prototype,
        'value'
      ).set;
      set.call(inp, 'rust');
      inp.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await page.waitForTimeout(800);

    const opened = await page.evaluate((T) => {
      const all = [...document.querySelectorAll('button, a')];
      const b =
        all.find((x) => (x.textContent || '').trim() === T) ||
        all.find((x) => (x.getAttribute('title') || '').trim() === T) ||
        all.find((x) => (x.getAttribute('aria-label') || '').trim() === T);
      if (!b) return { err: `no trigger "${T}"` };
      b.click();
      return { ok: true };
    }, d.trigger);

    if (opened.err) {
      findings.push({ ...d, problem: opened.err });
      continue;
    }
    await page.waitForTimeout(1200);

    const state = await page.evaluate(() => {
      const ds = [...document.querySelectorAll('[role="dialog"]')];
      if (!ds.length) return { err: 'no dialog opened' };
      const d = ds[ds.length - 1];
      let name = d.getAttribute('aria-label');
      const labelledby = d.getAttribute('aria-labelledby');
      if (!name && labelledby) {
        const h = document.getElementById(labelledby);
        name = h ? (h.textContent || '').trim() : null;
        if (!h) return { err: `aria-labelledby="${labelledby}" points at nothing` };
      }
      return {
        name: name || null,
        ariaModal: d.getAttribute('aria-modal'),
        inAriaHidden: !!d.closest('[aria-hidden="true"]'),
        focusInside: d.contains(document.activeElement),
        active: document.activeElement
          ? document.activeElement.tagName +
            '|' +
            (document.activeElement.getAttribute('aria-label') ||
              document.activeElement.id ||
              (document.activeElement.textContent || '').trim().slice(0, 24))
          : null,
      };
    });

    if (state.err) {
      findings.push({ ...d, problem: state.err });
      continue;
    }
    if (!state.name) findings.push({ ...d, problem: 'the dialog has no accessible NAME' });
    if (state.ariaModal !== 'true')
      findings.push({ ...d, problem: 'aria-modal is not "true"' });
    if (state.inAriaHidden)
      findings.push({ ...d, problem: 'the dialog is inside an aria-hidden subtree' });
    if (!state.focusInside)
      findings.push({
        ...d,
        problem: `the focus never entered the dialog (activeElement: ${state.active})`,
      });

    // Escape closes it - or it ASKS first, which is the same contract.
    //
    // A dialog with unsaved work asks "Discard this post?" rather than
    // closing outright. That is the CORRECT behaviour, and a contract
    // that only accepts a closed dialog would call it a failure.
    await page.keyboard.press('Escape');
    await page.waitForTimeout(900);
    const after = await page.evaluate(() => {
      const dialogs = [...document.querySelectorAll('[role="dialog"]')];
      const alerts = [...document.querySelectorAll('[role="alertdialog"]')];
      return {
        dialogs: dialogs.length,
        alerts: alerts.length,
        alertText: alerts.map((a) => (a.textContent || '').trim().slice(0, 60)),
      };
    });
    if (after.dialogs > 0 && after.alerts === 0)
      findings.push({ ...d, problem: 'Escape did not close the dialog' });
    if (after.alerts > 0) {
      // A confirm answered with "cancel" must leave the dialog open.
      const kept = await page.evaluate(() => {
        const b = [...document.querySelectorAll('button')].find((x) =>
          /keep|cancel|not now/i.test((x.textContent || '').trim())
        );
        if (b) {
          b.click();
          return true;
        }
        return false;
      });
      if (kept) {
        await page.waitForTimeout(700);
        const stillOpen = await page.evaluate(
          () => document.querySelectorAll('[role="dialog"]').length
        );
        if (stillOpen === 0)
          findings.push({
            ...d,
            problem: 'the Escape confirm discarded the work when it was cancelled',
          });
      }
    }

    console.log(
      `${d.expect.padEnd(22)} name=${JSON.stringify(state.name)} modal=${state.ariaModal} focusInside=${state.focusInside}`
    );
  }

  await browser.close();

  if (findings.length) {
    console.log('');
    for (const f of findings) {
      console.log(`  ✗ ${f.route} (${f.expect}) — ${f.problem}`);
    }
    console.log(`\n${findings.length} dialog contract failure(s)`);
    process.exit(1);
  }
  console.log('\n0 dialog contract failures');
  process.exit(0);
}

main().catch((e) => {
  console.error(e);
  process.exit(3);
});
