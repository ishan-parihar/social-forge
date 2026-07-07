<script lang="ts" module>
  // v25-2: Sparkline — a tiny inline-SVG trend chart for dashboard widgets.
  //
  // Design goals:
  //   - Pure SVG, no Chart.js / D3 / visx dependency (per AGENTS.md §0.5.3
  //     "no new third-party frontend libs").
  //   - Theme-aware via `currentColor` — set `text-brand-400`, `text-success`,
  //     etc. on the parent and the sparkline inherits it. Works in dark + light.
  //   - Accessible: role="img" + aria-label + <title> for hover tooltip.
  //   - Edge-case safe: handles empty array (renders nothing), single point
  //     (renders a dot), all-equal data (renders a flat baseline).
  //
  // Usage:
  //   <Sparkline data={[1, 3, 2, 5, 4, 6]} class="text-brand-400 w-full h-8" />
  //   <Sparkline data={cadence.by_day.map(d => d.count)} ariaLabel="Posts per day, 30d" />
  //
  // The `class` prop is forwarded to the <svg> so callers can control width,
  // height, and color with standard Tailwind classes. When `class` sets a
  // width/height, the `width`/`height` props are ignored (SVG prefers attributes
  // but CSS wins).

  // Stable counter for unique gradient/clip IDs (avoids SSR hydration
  // mismatches that Math.random() would cause, even though we're SPA-only).
  let idCounter = 0;
  export function nextSparklineId(): string {
    idCounter += 1;
    return `spark-${idCounter}`;
  }
</script>

<script lang="ts">
  let {
    data,
    width = 100,
    height = 28,
    strokeWidth = 1.5,
    fillOpacity = 0.15,
    ariaLabel = 'Trend sparkline',
    min,
    max,
    fill = true,
    showLastDot = true,
    class: className = '',
  }: {
    data: number[];
    width?: number;
    height?: number;
    strokeWidth?: number;
    fillOpacity?: number;
    ariaLabel?: string;
    min?: number;
    max?: number;
    fill?: boolean;
    showLastDot?: boolean;
    class?: string;
  } = $props();

  // Compute polyline points + area polygon. Memoized via $derived so it
  // only re-runs when data/dimensions change.
  let geom = $derived.by(() => {
    const n = data?.length ?? 0;
    if (n === 0) return { line: '', area: '', dots: [] as Array<{ x: number; y: number }>, lastDot: null as null | { x: number; y: number } };

    const lo = min ?? Math.min(...data);
    const hi = max ?? Math.max(...data);
    // Avoid divide-by-zero on flat data (lo === hi): expand the range by 1
    // so the line sits in the vertical middle.
    const range = hi - lo || 1;
    const pad = strokeWidth;
    const w = width - pad * 2;
    const h = height - pad * 2;
    const xAt = (i: number) => pad + (n === 1 ? w / 2 : (i / (n - 1)) * w);
    const yAt = (v: number) => pad + h - ((v - lo) / range) * h;

    if (n === 1) {
      const cy = yAt(data[0]);
      return { line: '', area: '', dots: [{ x: xAt(0), y: cy }], lastDot: { x: xAt(0), y: cy } };
    }

    const coords = data.map((v, i) => ({ x: xAt(i), y: yAt(v) }));
    const line = coords.map(p => `${p.x.toFixed(2)},${p.y.toFixed(2)}`).join(' ');
    const area = `${pad.toFixed(2)},${(pad + h).toFixed(2)} ${line} ${xAt(n - 1).toFixed(2)},${(pad + h).toFixed(2)}`;
    return { line, area, dots: [], lastDot: coords[coords.length - 1] };
  });

  let svgId = $state(nextSparklineId());
</script>

{#if data && data.length > 0}
  <svg
    {width}
    {height}
    viewBox="0 0 {width} {height}"
    role="img"
    aria-label={ariaLabel}
    preserveAspectRatio="none"
    class="sparkline {className}"
  >
    <title>{ariaLabel}</title>
    {#if fill && geom.area}
      <polygon points={geom.area} fill="currentColor" fill-opacity={fillOpacity} stroke="none" />
    {/if}
    {#if geom.line}
      <polyline
        points={geom.line}
        fill="none"
        stroke="currentColor"
        stroke-width={strokeWidth}
        stroke-linecap="round"
        stroke-linejoin="round"
        vector-effect="non-scaling-stroke"
      />
    {/if}
    {#each geom.dots as d (d.x + ',' + d.y)}
      <circle cx={d.x} cy={d.y} r={strokeWidth} fill="currentColor" />
    {/each}
    {#if showLastDot && geom.lastDot}
      <circle
        cx={geom.lastDot.x}
        cy={geom.lastDot.y}
        r={strokeWidth + 0.75}
        fill="currentColor"
        stroke="var(--bg-card)"
        stroke-width="1"
      />
    {/if}
  </svg>
{/if}

<style>
  /* No hardcoded colors here — the sparkline uses `currentColor` for both
     stroke and fill, so it inherits the parent's text-* color and rethemes
     correctly in dark/light mode. */
  .sparkline {
    display: block;
    overflow: visible;
  }
</style>
