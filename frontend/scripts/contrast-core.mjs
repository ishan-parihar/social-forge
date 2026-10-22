// The ONE implementation of the contrast rule (AGENTS.md §9.18: "A rule
// that both the browser and the server enforce needs ONE implementation
// on each side" — and one per side of the SCRIPTS too: probe.mjs,
// audit-contrast.mjs and state-hunt.mjs each carried a private copy, and
// copy #3 shipped with a channel-alpha typo that produced negative
// luminances and impossible ratios. That class of drift is what this
// module exists to close.)
//
// The compositing is the probe's (superset of audit-contrast's): it
// paints translucent washes over the nearest opaque ancestor and
// treats an `opacity < 1` ancestor as a composited GROUP whose own
// background is faded into the backdrop behind it.
//
// Each script still owns its own REPORT policy (what counts as a FAIL
// vs a WARN, which exemptions apply); this module owns only the
// measurement.
export const CONTRAST_CORE = `
  const parseColor = (v) => {
    const m = v.match(/rgba?\\(([^)]+)\\)/);
    if (!m) return null;
    const [r, g, b, a] = m[1].split(/[,\\s/]+/).filter(Boolean).map(Number);
    return { r, g, b, a: a === undefined ? 1 : a };
  };
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
  const effectiveBg = (el) => {
    const layers = [];
    let node = el;
    while (node && node.nodeType === 1) {
      const opacity = parseFloat(getComputedStyle(node).opacity);
      if (node !== el && !Number.isNaN(opacity) && opacity < 0.999) {
        const c = parseColor(getComputedStyle(node).backgroundColor);
        if (c && c.a > 0) {
          layers.push({ r: c.r, g: c.g, b: c.b, a: c.a * opacity });
        }
        break;
      }
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
`;
