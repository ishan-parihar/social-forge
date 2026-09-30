<script lang="ts">
  // Loading placeholder primitive.
  //
  // v22 Phase 3: single shimmer block.
  // v25 F1: added `rows` + `variant` so the ~20 routes that used to hand-roll
  //   their own loading markup (or a bare "Loading..." string) share ONE shape.
  //   The point is not the shimmer — it is that the placeholder reserves the
  //   same space the real content will occupy, so the page does not jump when
  //   data lands. `variant` picks that reserved shape:
  //     text — a paragraph of ragged lines (settings panels, detail views)
  //     row  — a list of one-line rows        (tables, small-item lists)
  //     card — a list of tall cards            (feeds, grids, dashboards)
  //   Colour comes from the `.skeleton` class in app.css, which is token-driven,
  //   so a placeholder reads in both themes with no per-theme branch.
  let {
    width = "100%",
    height = "1rem",
    rounded = "md",
    rows = 1,
    variant = "text",
    gap = "0.5rem",
  }: {
    width?: string;
    height?: string;
    rounded?: "sm" | "md" | "lg" | "full";
    /** How many stacked placeholders to render. */
    rows?: number;
    /** Reserved shape of each placeholder. */
    variant?: "text" | "row" | "card";
    gap?: string;
  } = $props();

  const r = { sm: "rounded-sm", md: "rounded-md", lg: "rounded-lg", full: "rounded-full" };

  // Per-variant line composition: height plus the width each line takes, so a
  // multi-row block looks like ragged text rather than a stack of identical bars.
  const shapes: Record<string, { h: string; widths: string[] }> = {
    text: { h: "0.875rem", widths: ["100%", "92%", "78%"] },
    row: { h: "2.25rem", widths: ["100%"] },
    card: { h: "7rem", widths: ["100%"] },
  };
  const shape = $derived(shapes[variant] ?? shapes.text);
</script>

{#if rows > 1}
  <div class="flex flex-col" style="gap: {gap}" aria-hidden="true">
    {#each Array(rows) as _, i (i)}
      <div class="skeleton {r[rounded]}" style="width: {shape.widths[i % shape.widths.length]}; height: {shape.h};"></div>
    {/each}
  </div>
{:else}
  <div class="skeleton {r[rounded]}" style="width: {width}; height: {height};" aria-hidden="true"></div>
{/if}
