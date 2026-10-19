// Colour helpers for USER-CHOSEN colours: tag colours and campaign
// colours. Both are picked by the operator from a colour input and
// persisted, so their luminance is not under the app's control.
//
// The problem: a chip painted `color: <user hex>` on a wash of the same
// hex lands anywhere from 1.3:1 to 8:1 against the surface, depending on
// the hue. A mid-tone hue (indigo `#6366f1` on the dark calendar cell)
// measures 4.01:1 - under the WCAG AA floor for small text, and the
// probe gates on exactly that.
//
// The fix, and why it is safe: on a SOLID fill of the chosen colour,
// black and white are the only two candidate inks, and one of them
// always clears 4.5:1. The worst case is a mid-luminance grey, where
// both sit at about 4.6:1. So a solid fill with a computed ink is
// readable for EVERY colour the colour input can produce.
//
// (Picking the ink by walking the hue's lightness - the approach
// `providerInk` uses for text on the page background - does not work
// here: no single static hex clears 4.5:1 on both a light and a dark
// fill, so the ink would have to change with the theme, and a chip's
// fill does not change with the theme.)

/// The two candidate inks, as CSS token references. Both are absolute
/// (they do not change with the theme) because the ink is chosen against
/// the chip's OWN fill, which is theme-independent.
const INK_LIGHT = 'rgb(var(--white-rgb))';
const INK_DARK = 'rgb(var(--black-rgb))';

/// WCAG relative luminance of an `#rrggbb` (or `#rgb`) hex.
function luminance(hex: string): number {
  const h = hex.replace('#', '').trim();
  const full =
    h.length === 3
      ? h
          .split('')
          .map((c) => c + c)
          .join('')
      : h;
  if (!/^[0-9a-fA-F]{6}$/.test(full)) return 0;
  const lin = (i: number) => {
    const v = parseInt(full.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(0) + 0.7152 * lin(2) + 0.0722 * lin(4);
}

/**
 * The ink to paint ON a solid fill of `fill`.
 *
 * Black or white, whichever contrasts more. One of the two always clears
 * 4.5:1 against any fill, so a chip built on this is readable for every
 * colour the picker can produce.
 *
 * Returns a CSS colour value usable in an inline `style`.
 */
export function inkOnFill(fill: string): string {
  const l = luminance(fill);
  // WCAG ratio: (lighter + 0.05) / (darker + 0.05).
  const vsWhite = 1.05 / (l + 0.05);
  const vsBlack = (l + 0.05) / 0.05;
  return vsWhite >= vsBlack ? INK_LIGHT : INK_DARK;
}
