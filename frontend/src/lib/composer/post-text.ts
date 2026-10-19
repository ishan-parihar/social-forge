// ─── Post body: HTML -> plain text ─────────────────────────────
//
// ONE implementation of the rule the SERVER also implements
// (`social::common::strip_html_tags`): the post body is plain text, and
// this is where the browser's copy of that conversion lives.
//
// It is a SEPARATE, dependency-free module on purpose. The parity test
// (`tests/html_to_text_test.rs`) loads THIS FILE and runs it under node,
// so the test exercises the code that actually ships. A hand-written
// transcription of it in the test would be a second copy of the rule -
// and a second copy is exactly how the two sides drifted in the first
// place.
//
// The rule:
//   - a string with no EVIDENCE of markup is left alone (a bare `<div>`
//     is a word in angle brackets, and a post about HTML is full of
//     them);
//   - a `<script>`/`<style>` BODY is code, not prose, and is dropped;
//   - entities are decoded, so the count is the count of the TEXT the
//     provider receives;
//   - a block element ends a line.

/** The elements that end a line when their tag is removed. */
function isBlockElement(name: string): boolean {
  return [
    'p', 'div', 'li', 'ul', 'ol',
    'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
    'blockquote', 'pre', 'section', 'article',
    'header', 'footer', 'tr', 'table',
  ].includes(name);
}

export function plainText(html: string): string {
  // The server's rule, exactly: convert only when the string carries
  // EVIDENCE of markup - a closing tag, an attribute, or a void tag.
  // A bare `<div>` is a word in angle brackets, and a post about HTML is
  // full of them.
  //
  // It used to strip on any `<` followed by a letter, so the count the
  // composer showed was the count of a DIFFERENT string than the one
  // the server stored: the composer measured "Use  for layout" (19)
  // while the post said "Use <div> for layout" (23). The two sides of
  // a rule must agree - see `tests/counting_rule_test.rs`.
  if (!looksLikeHtml(html)) return html.trim();

  let out = '';
  let i = 0;
  while (i < html.length) {
    const ch = html[i];

    // A `<script>` or `<style>` BODY is code, not prose: it is dropped
    // whole. The server does the same, so the two agree on the length.
    if (ch === '<') {
      const body = scriptOrStyleBody(html, i);
      if (body !== null) {
        i = body;
        continue;
      }
    }

    // Entities are decoded, so the count is the count of the TEXT the
    // provider receives. The server decodes them too.
    if (ch === '&') {
      const decoded = decodeEntity(html, i);
      if (decoded !== null) {
        out += decoded.text;
        // Advance by the ENTITY's length, not by one character. Advancing
        // by one re-read the tail of the entity as text and never
        // reached the end of the string.
        i += decoded.consumed;
        continue;
      }
    }

    if (ch !== '<') {
      out += ch;
      i += 1;
      continue;
    }
    const next = html[i + 1];
    const isTag =
      next !== undefined &&
      (/[A-Za-z/!?]/.test(next));
    if (!isTag) {
      out += '<';
      i += 1;
      continue;
    }
    const end = html.indexOf('>', i);
    if (end === -1) {
      out += '<';
      i += 1;
      continue;
    }
    const tag = html.slice(i + 1, end).toLowerCase();
    const name = tag.replace(/^[!/]/, '').split(/[\s/>]/)[0];
    if (name === 'br') out += '\n';
    else if (isBlockElement(name)) {
      if (out && !out.endsWith('\n')) out += '\n';
    }
    i = end + 1;
  }
  return out.replace(/\n{3,}/g, '\n\n').trim();
}

/**
 * If `start` opens a `<script>` or `<style>`, return the index just
 * past its closing tag - the body is code, not prose, and is dropped
 * whole. `null` when it does not.
 *
 * The server drops these bodies too (`social::common::strip_html_tags`),
 * so the two sides agree on the length of the text.
 */
function scriptOrStyleBody(html: string, start: number): number | null {
  const m = /^<(script|style)\b[^>]*>/i.exec(html.slice(start));
  if (!m) return null;
  const tag = m[1].toLowerCase();
  const close = html.toLowerCase().indexOf(`</${tag}>`, start + m[0].length);
  if (close === -1) return html.length;
  return close + tag.length + 3;
}

/**
 * Decode the entity at `start`, if there is one.
 *
 * Returns the decoded text and the number of characters consumed, or
 * `null` when there is no entity there. The server decodes the same
 * set, so the two count the same text.
 */
function decodeEntity(
  html: string,
  start: number,
): { text: string; consumed: number } | null {
  const m = /^&(amp|lt|gt|quot|apos|nbsp|hellip|mdash|ndash|#\d+|#[xX][0-9a-fA-F]+);/.exec(
    html.slice(start),
  );
  if (!m) return null;
  const body = m[1];
  let text: string;
  switch (body) {
    case 'amp': text = '&'; break;
    case 'lt': text = '<'; break;
    case 'gt': text = '>'; break;
    case 'quot': text = '"'; break;
    case 'apos': text = "'"; break;
    // A PLAIN space, not U+00A0. The counting rule is UTF-8 BYTES on
    // both sides, and U+00A0 is two bytes where U+0020 is one - so a
    // non-breaking space here would make the composer's count differ
    // from the server's by one per entity.
    case 'nbsp': text = '\u0020'; break;
    case 'hellip': text = '\u2026'; break;
    case 'mdash': text = '\u2014'; break;
    case 'ndash': text = '\u2013'; break;
    default: {
      const code = body.startsWith('#x') || body.startsWith('#X')
        ? parseInt(body.slice(2), 16)
        : parseInt(body.slice(1), 10);
      // An invalid code point (above U+10FFFF) is left as literal text,
      // which is what the server does - `String.fromCodePoint` THROWS
      // for it, and that would take the composer down mid-typing.
      if (!Number.isFinite(code) || code < 0 || code > 0x10ffff) return null;
      text = String.fromCodePoint(code);
    }
  }
  return { text, consumed: m[0].length };
}

/**
 * Does this string carry evidence of markup?
 *
 * The mirror of the server's `social::common::looks_like_html`. A
 * closing tag, an attribute, or a void tag is evidence; a bare
 * `<word>` is not.
 */
function looksLikeHtml(s: string): boolean {
  const VOID_TAGS = ['br', 'hr', 'img', 'input', 'meta', 'link', 'wbr', 'source'];
  const KNOWN_TAGS = [
    'p', 'div', 'span', 'br', 'hr', 'b', 'i', 'u', 'strong', 'em', 's', 'strike', 'a', 'ul', 'ol',
    'li', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'blockquote', 'pre', 'code', 'section', 'article',
    'header', 'footer', 'main', 'aside', 'nav', 'figure', 'figcaption', 'table', 'thead', 'tbody',
    'tr', 'td', 'th', 'img', 'picture', 'source', 'video', 'audio', 'iframe', 'script', 'style',
    'small', 'sub', 'sup', 'mark', 'del', 'ins', 'abbr', 'cite', 'q', 'kbd', 'samp', 'var', 'wbr',
  ];
  let i = 0;
  while (i < s.length) {
    if (s[i] !== '<') {
      i += 1;
      continue;
    }
    const next = s[i + 1];
    if (next === '/' || next === '!' || next === '?') return true;
    const rest = s.slice(i + 1).toLowerCase();
    for (const tag of VOID_TAGS) {
      if (rest.startsWith(tag)) {
        const after = rest.charAt(tag.length);
        if (after === '>' || after === ' ' || after === '/') return true;
      }
    }
    for (const tag of KNOWN_TAGS) {
      if (rest.startsWith(tag)) {
        const after = rest.charAt(tag.length);
        if (after === ' ' || after === '\t' || after === '\n') return true;
      }
    }
    i += 1;
  }
  return false;
}
