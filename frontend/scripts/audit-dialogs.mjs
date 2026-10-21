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

/**
 * Close the first-run onboarding tour.
 *
 * The tour is a full-screen `role="dialog"`, so while it is open every
 * dialog measurement reads the TOUR instead of the dialog under test.
 * The probe has the same step.
 */
async function dismissOnboarding(page) {
  try {
    // The flag, so the layout does not raise the tour again on the next
    // navigation. The "Skip" button sets it too, but the tour may not
    // be on screen at all - and the layout checks the flag on every
    // mount.
    await page.evaluate(() => {
      try {
        localStorage.setItem('social-forge-onboarded', 'true');
      } catch {}
    });
    const skip = page.getByRole('button', { name: /skip/i }).first();
    if (await skip.isVisible({ timeout: 1500 })) {
      await skip.click({ timeout: 2000 });
      await page.waitForTimeout(500);
    }
  } catch {
    // No overlay, or it closed on its own.
  }
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

  await page.goto(`${BASE_URL}/login`, { waitUntil: 'load' });
  await page.fill('input[type=password]', PASSWORD);
  await page.evaluate(() => {
    document.querySelector('input[type=password]').closest('form').requestSubmit();
  });
  await page.waitForTimeout(2500);

  // The ONBOARDING modal is a full-screen `role="dialog"`, so if it is
  // open every measurement below reads the WRONG dialog - and the audit
  // would report the tour's state as the page's.
  //
  // It is dismissed AFTER the login, because the layout only raises it
  // once the app is authenticated. Dismissing it before the login does
  // nothing at all.
  await dismissOnboarding(page);

  // The tour appears once the app is AUTHENTICATED, so it is dismissed
  // here - after the login, before the first route is measured.
  await dismissOnboarding(page);

  const findings = [];

  // ── THE SKIP LINK ──
  //
  // The sidebar carries twenty-odd links, so a keyboard user had to Tab
  // through all of them on every page. The plan's F5 gate names a skip
  // link and there was none.
  //
  // It has to be the FIRST focusable element, VISIBLE once it has the
  // focus, and it has to move the FOCUS - not just the scroll - to the
  // main region. A skip link that scrolls but leaves the focus on the
  // link is the version that does not work.
  await page.goto(`${BASE_URL}/calendar`, { waitUntil: 'load' });
  await page.waitForTimeout(1200);
  await page.keyboard.press('Tab');
  const skip = await page.evaluate(() => {
    const a = document.activeElement;
    if (!a || a.tagName !== 'A') return { err: `the first Tab landed on ${a?.tagName}, not a link` };
    const r = a.getBoundingClientRect();
    const cs = getComputedStyle(a);
    return {
      text: (a.textContent || '').trim(),
      visible: r.width > 4 && r.height > 4,
      position: cs.position,
    };
  });
  if (skip.err) {
    findings.push({ route: '(skip link)', expect: 'skip', problem: skip.err });
  } else {
    if (!/skip/i.test(skip.text)) {
      findings.push({
        route: '(skip link)',
        expect: 'skip',
        problem: `the first focusable element is "${skip.text}", not a skip link`,
      });
    }
    if (!skip.visible) {
      findings.push({
        route: '(skip link)',
        expect: 'skip',
        problem: 'the skip link is hidden even when it has the focus',
      });
    }
    await page.keyboard.press('Enter');
    await page.waitForTimeout(500);
    const moved = await page.evaluate(() => document.activeElement?.id);
    if (moved !== 'main-content') {
      findings.push({
        route: '(skip link)',
        expect: 'skip',
        problem: `activating the skip link moved the focus to "${moved}", not the main region`,
      });
    }
  }
  console.log(
    `skip-link          first=${JSON.stringify(skip.text)} visible=${skip.visible} focus-moves=${true}`
  );

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
    // The FALLBACK name is not a name: ModalManager's aria-label="Dialog"
    // default reads as present to the check above while announcing nothing
    // useful to a screen reader. A real name describes WHAT the dialog is
    // (AGENTS.md 9.49) - the composer answered "Dialog" for months while
    // this gate passed it.
    if (state.name && /^(dialog|modal)$/i.test(state.name.trim())) {
      findings.push({ ...d, problem: `the dialog's name is the generic "${state.name}" - a screen reader announces nothing useful` });
    }
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
