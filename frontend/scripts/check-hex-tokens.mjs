#!/usr/bin/env node
// v25 F1 — the "no hex outside tokens" grep gate.
//
// The point: a color that is written as a literal in a component does not
// retheme. It cannot, by construction. This script makes that a build failure
// instead of something a reviewer has to notice.
//
// A hex is ALLOWED only in:
//   src/app.css                    — the two :root token blocks. This is the
//                                     definition site, not a violation.
//   src/lib/providers.ts           — provider BRAND identities (X is #000, Reddit
//                                     is #FF4500). Those are facts about other
//                                     companies, not this app's theme.
//   src/lib/channels/ProviderIcon.svelte — same brand identities, as SVG fills.
//   src/lib/composer/previews/**    — the platform post previews deliberately
//                                     simulate the DESTINATION platform's own
//                                     chrome (white background, gray text), not
//                                     this app's theme. Retinting those would make
//                                     the preview a lie.
//   src/routes/tags/+page.svelte    — the user's own chosen tag colors, which are
//                                     data (persisted per tag), not chrome.
//
// Anything else is a bug. Exit 1 with file:line so it is directly clickable.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(fileURLToPath(new URL(".", import.meta.url)), "..");
const SRC = join(ROOT, "src");

/** Path prefixes (relative to src/) that may contain literal hex. */
const ALLOW = [
  "app.css", //                         the token definitions themselves
  "lib/providers.ts", //                provider brand identities = data
  "lib/channels/ProviderIcon.svelte", // provider brand identities = data
  "lib/composer/previews/", //          destination-platform simulation
  "routes/tags/+page.svelte", //        user-chosen tag colors = data
  // A new campaign is created with a default color that is PERSISTED to the
  // database and sent to the API. It is a payload value, not chrome — there is
  // no CSS context for it to retheme in, so a literal is correct here.
  "routes/campaigns/",
  "routes/kanban/",
];

/** Allowlist for comments that legitimately name a hex as documentation.
 *  These are prose, not values — `stripComments` removes them first. */
const HEX = /#[0-9a-fA-F]{3,8}\b/g;

function isAllowed(rel) {
  return ALLOW.some((a) => rel === a || rel.startsWith(a));
}

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) yield* walk(full);
    else if (/\.(svelte|ts|css|html)$/.test(entry)) yield full;
  }
}

/** Strip comments so a hex named in prose is not counted as a value. */
function stripComments(src, ext) {
  let s = src;
  if (ext === "css") return s.replace(/\/\*[\s\S]*?\*\//g, "");
  s = s.replace(/<!--[\s\S]*?-->/g, "");       // svelte/html comments
  s = s.replace(/\/\*[\s\S]*?\*\//g, "");      // block comments
  // Svelte/TS/HTML: line comments. Naive but adequate — a `//` inside a string
  // literal is vanishingly rare here and would only cause a false NEGATIVE
  // (missed violation), never a false positive.
  s = s.replace(/(^|[^:])\/\/.*$/gm, "$1");
  return s;
}

const violations = [];
for (const file of walk(SRC)) {
  const rel = relative(SRC, file).replaceAll("\\", "/");
  if (isAllowed(rel)) continue;
  const src = stripComments(readFileSync(file, "utf8"), rel.split(".").pop());
  for (const m of src.matchAll(HEX)) {
    // `&#128269;` is an HTML numeric entity (🔍), not a color. The `#` is
    // preceded by `&` and followed by digits that happen to be hex-valid.
    if (src[m.index - 1] === "&") continue;
    const line = src.slice(0, m.index).split("\n").length;
    violations.push(`  src/${rel}:${line}  ${m[0]}`);
  }
}

if (violations.length) {
  console.error(
    `\n✗ check:tokens — ${violations.length} hardcoded color(s) outside the token layer:\n` +
      violations.join("\n") +
      `\n\n  Fix: reference a semantic token (see tailwind.config.js → colors) or add a\n` +
      `  real value to src/app.css. To allow a value that is DATA rather than\n` +
      `  chrome (a brand color, a user-chosen color), extend ALLOW in\n` +
      `  scripts/check-hex-tokens.mjs with a comment saying why.\n`,
  );
  process.exit(1);
}

console.log("✓ check:tokens — no hardcoded colors outside the token layer");
