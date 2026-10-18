// Text colour for a solid fill: white or black, whichever clears more contrast
// on it. Expects a six-digit `#rrggbb`. The better of the two is always at
// least 4.58:1 (√21), so the result meets 4.5:1 for any fill.
export function inkFor(hex: string): 'white' | 'black' {
  const lin = (i: number) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  const lum = 0.2126 * lin(1) + 0.7152 * lin(3) + 0.0722 * lin(5);
  return 1.05 / (lum + 0.05) >= (lum + 0.05) / 0.05 ? 'white' : 'black';
}
