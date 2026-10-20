#!/usr/bin/env node
// Every Tailwind class the source uses must EXIST in the built CSS.
//
// A truncated class name is a SILENT failure. `hover:text-accent-strong`
// cut to `hover:text-accent-stro` is not a build error, not a type
// error, and not a svelte-check warning: Tailwind's JIT simply does
// not generate a rule for a class it does not recognise, and the
// element loses the style.
//
// The two that shipped on the research page were `text-accent-stro`
// and `items-centerng` - a link with no hover colour and a link whose
// content was not vertically centred. Nothing reported them.
//
// So the check is: collect the class names the source uses, and
// confirm each one is in the generated stylesheet.
//
// Run: node scripts/check-tailwind-classes.mjs
//
// It exits 1 with file:line for every class that is missing.

import { readFileSync, readdirSync, statSync, existsSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(fileURLToPath(new URL('.', import.meta.url)), '..');
const SRC = join(ROOT, 'src');
const BUILD = join(ROOT, 'build');

// The utility prefixes a class can legitimately start with. A class
// that does not start with one of these is a custom class, and it is
// checked against the CSS as a whole - which catches a custom class
// that was never written.
const TAILWIND_PREFIXES = [
  'text-', 'bg-', 'border-', 'ring-', 'shadow-', 'opacity-', 'p-', 'px-',
  'py-', 'pt-', 'pb-', 'pl-', 'pr-', 'm-', 'mx-', 'my-', 'mt-', 'mb-',
  'ml-', 'mr-', 'gap-', 'space-x-', 'space-y-', 'w-', 'h-', 'min-w-',
  'min-h-', 'max-w-', 'max-h-', 'top-', 'right-', 'bottom-', 'left-',
  'inset-', 'z-', 'grid-', 'col-', 'row-', 'flex-', 'order-', 'items-',
  'justify-', 'place-', 'self-', 'content-', 'rounded-', 'font-',
  'leading-', 'tracking-', 'whitespace-', 'break-', 'truncate-', 'uppercase',
  'lowercase-', 'capitalize-', 'italic-', 'underline-', 'line-through-',
  'no-underline-', 'overflow-', 'overscroll-', 'object-', 'aspect-',
  'transition', 'duration-', 'ease-', 'delay-', 'animate-', 'cursor-',
  'select-', 'pointer-events-', 'resize-', 'appearance-', 'fill-', 'stroke-',
  'sr-only', 'not-sr-only', 'invisible-', 'visible-', 'static-', 'fixed-',
  'absolute-', 'relative-', 'sticky-', 'isolate-', 'box-', 'divide-',
  'space-', 'basis-', 'grow-', 'shrink-', 'transform-', 'translate-',
  'rotate-', 'scale-', 'skew-', 'origin-', 'backdrop-', 'filter-',
  'blur-', 'brightness-', 'contrast-', 'grayscale-', 'invert-', 'saturate-',
  'sepia-', 'shadow', 'outline-', 'will-change-', 'list-', 'align-',
  'whitespace-', 'break-', 'table-', 'caption-', 'border-collapse-',
];

// A class that is a custom component class, not a Tailwind utility.
// The app's own classes live in app.css and in <style> blocks.
function isUtility(name) {
  const base = name.replace(/^(hover|focus|focus-visible|active|disabled|group-hover|peer-hover|sm|md|lg|xl|2xl|dark|first|last|odd|even|checked|required|invalid|read-only|placeholder)-/g, '');
  return TAILWIND_PREFIXES.some((p) => base.startsWith(p));
}

function* walk(dir) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) yield* walk(full);
    else if (/\.(svelte|ts|js)$/.test(entry)) yield full;
  }
}

// ── The generated CSS ──
if (!existsSync(BUILD)) {
  console.error('BLOCKED: frontend/build does not exist. Run `pnpm build` first.');
  process.exit(3);
}
const css = readdirSync(join(BUILD, '_app/immutable/assets'))
  .filter((f) => f.endsWith('.css'))
  .map((f) => readFileSync(join(BUILD, '_app/immutable/assets', f), 'utf8'))
  .join('\n');

// The class names the CSS actually defines.
//
// Tailwind escapes the special characters in a class name: `:` becomes
// `\:`, `[` becomes `\[`, and the `.` inside an arbitrary value
// becomes `\.`. So a rule for `min-w-[1.125rem]` is written
// `.min-w-\[1\.125rem\]`, and reading it back means UNESCAPING it.
//
// The regex has to allow the escaped forms, which is why it is written
// against the escaped alphabet rather than the plain one.
const defined = new Set();
// The class body.
//
// Tailwind escapes the characters that are special in a CSS selector:
// `:` becomes `\:`, `[` and `]` become `\[` and `\]`, `(` and `)`
// become `\(` and `\)`, a literal `.` becomes `\.`, a `,` becomes `\,`,
// a `/` becomes `\/`, and a space inside an arbitrary value becomes
// `_`.
//
// So the body is: letters, digits, `-`, `_`, and any ESCAPED
// character. The escaped character is a backslash followed by
// anything, and the pair is what the regex has to match.
const CLASS_IN_CSS = /\.((?:[a-z0-9_-]|\\.)+)(?=[\s,{:.[>+~])/gi;
for (const m of css.matchAll(CLASS_IN_CSS)) {
  // Unescape: `\x` -> `x`, so `hover\:text-accent` -> `hover:text-accent`.
  const name = m[1].replace(/\\(.)/g, '$1');
  defined.add(name);
}

const missing = [];
for (const file of walk(SRC)) {
  const rel = relative(ROOT, file);
  const text = readFileSync(file, 'utf8');
  const lines = text.split('\n');
  lines.forEach((line, i) => {
    // Only look at class ATTRIBUTES.
    for (const m of line.matchAll(/class="([^"{}]*)"/g)) {
      const value = m[1];
      for (const raw of value.split(/\s+/)) {
        const name = raw.trim();
        if (!name) continue;
        if (name.includes('{') || name.includes('}')) continue;
        // Svelte interpolation: skip anything that is not a literal.
        if (name.includes('$') || name.includes("'")) continue;
        if (!isUtility(name)) continue;
        if (defined.has(name)) continue;
        missing.push({ file: rel, line: i + 1, name });
      }
    }
  });
}

if (missing.length) {
  console.error(`${missing.length} Tailwind class(es) used in the source but NOT in the built CSS:`);
  console.error('  A class that is not in the stylesheet is a SILENT failure -');
  console.error('  the element simply does not get the style, and nothing reports it.\n');
  for (const m of missing) {
    console.error(`  ${m.file}:${m.line}  ${m.name}`);
  }
  process.exit(1);
}

console.log('✓ check:tailwind - every Tailwind class in the source exists in the built CSS');
