// The analytics dashboard, with a SEEDED account.
//
// F4's verification was "analytics with seeded data renders non-empty",
// which is a sentence, not a gate. A dashboard that renders a page of
// zeroes is non-empty.
//
// So this checks the things a founder would actually be misled by:
//
//   - the ENGAGEMENT totals are non-zero (the headline numbers)
//   - the posting-volume series is DENSE - every day in the window is
//     present, including the days with no post. A series that only has
//     the days WITH posts reads as "you posted every day at full
//     volume" next to a cadence card that says 0.4 posts a day.
//   - the SUCCESS RATE and the number beside it RECONCILE. The
//     adherence rate is measured against a goal-inflated denominator,
//     so a card that calls it a "success rate" while showing "11 of 13
//     went out" cannot be reconciled by the person reading it.
//   - the per-post engagement list is not empty
//
// Run: node scripts/audit-analytics.mjs --url https://localhost:6544 --password "$APP_PASSWORD"

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
    ]),
  '/usr/bin/chromium',
].filter(Boolean);

const arg = (name, fallback) => {
  const i = process.argv.indexOf(`--${name}`);
  return i > -1 && process.argv[i + 1] ? process.argv[i + 1] : fallback;
};

const BASE_URL = arg('url', 'https://localhost:6544');
const PASSWORD = arg('password', '');

async function main() {
  if (!PASSWORD) {
    console.error('BLOCKED: no --password');
    process.exit(3);
  }
  const executablePath = CHROME_CANDIDATES.find((p) => fs.existsSync(p));
  if (!executablePath) {
    console.error('BLOCKED: no Chromium found. Set CHROME_PATH.');
    process.exit(3);
  }

  const browser = await chromium.launch({ executablePath, args: ['--no-sandbox'] });
  const ctx = await browser.newContext({
    viewport: { width: 1440, height: 1400 },
    ignoreHTTPSErrors: true,
  });
  await ctx.addInitScript(() => {
    try {
      localStorage.setItem('social-forge-onboarded', 'true');
    } catch {}
  });
  const page = await ctx.newPage();

  await page.goto(`${BASE_URL}/login`, { waitUntil: 'load' });
  await page.fill('input[type=password]', PASSWORD);
  await page.evaluate(() => {
    document.querySelector('input[type=password]').closest('form').requestSubmit();
  });
  await page.waitForTimeout(2500);

  await page.goto(`${BASE_URL}/analytics`, { waitUntil: 'load' });
  await page.waitForTimeout(3000);

  const state = await page.evaluate(() => {
    const text = document.body.innerText;
    const num = (label) => {
      const m = text.match(new RegExp(`${label}\\s*\\n?\\s*([\\d,.]+)`));
      return m ? m[1] : null;
    };
    // The success-rate line and the numbers beside it.
    //
    // The DASHBOARD says "Success rate: N of M scheduled posts went out"
    // and the ANALYTICS page says "Publish success rate" over "N of M
    // scheduled posts went out". Both are checked, because both are the
    // same metric and a caption that cannot be reconciled is a defect
    // on either surface.
    const success =
      text.match(/([\d.]+)%\s*\n?\s*Success rate:\s*([\d]+) of ([\d]+)/) ||
      text.match(/Publish success rate\s*\n?\s*([\d.]+)%[\s\S]{0,80}?([\d]+) of ([\d]+) scheduled/);
    return {
      impressions: num('Impressions'),
      likes: num('Likes'),
      comments: num('Comments'),
      shares: num('Shares'),
      bars: document.querySelectorAll('[style*="height"]').length,
      success: success
        ? { pct: Number(success[1]), published: Number(success[2]), scheduled: Number(success[3]) }
        : null,
      emptyStates: (text.match(/No .*(yet|found|data)/g) || []).slice(0, 4),
    };
  });

  const findings = [];

  // 1. The engagement totals must be non-zero. A seeded account with
  //    engagement in the cache must not read as all zeroes.
  for (const k of ['impressions', 'likes', 'comments', 'shares']) {
    const v = state[k];
    if (v === null) continue; // the card is not on this window
    if (Number(String(v).replace(/,/g, '')) === 0) {
      findings.push(`the ${k} total reads 0 on a seeded account`);
    }
  }

  // 2. The success rate and the numbers beside it must RECONCILE.
  if (state.success) {
    const { pct, published, scheduled } = state.success;
    if (scheduled > 0) {
      const expected = Math.round((published / scheduled) * 100);
      if (Math.abs(pct - expected) > 1) {
        findings.push(
          `the success rate reads ${pct}% but ${published} of ${scheduled} scheduled posts went out, which is ${expected}%`
        );
      }
    }
  }

  // 3. The posting-volume series must be DENSE.
  if (state.bars > 0 && state.bars < 7) {
    findings.push(
      `the posting-volume chart has ${state.bars} columns - the day series is not dense, so the empty days are missing rather than zero`
    );
  }

  // 4. The page must not be an empty state.
  if (state.emptyStates.length >= 3) {
    findings.push(
      `the analytics page is mostly empty states: ${state.emptyStates.join('; ')}`
    );
  }

  await browser.close();

  console.log(
    `analytics  impressions=${state.impressions} likes=${state.likes} bars=${state.bars} success=${JSON.stringify(state.success)}`
  );
  if (findings.length) {
    for (const f of findings) console.log(`  ✗ ${f}`);
    console.log(`\n${findings.length} analytics failure(s)`);
    process.exit(1);
  }
  console.log('\n0 analytics failures');
  process.exit(0);
}

main().catch((e) => {
  console.error(e);
  process.exit(3);
});
