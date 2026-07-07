<script lang="ts">
  // v22 Phase 3: Tooltip primitive — hover/focus tooltip using a
  // title-attribute fallback + a styled span for capable browsers.
  // Lightweight (no floating-ui dep); positions above by default.
  let {
    text,
    position = "top",
    children,
  }: {
    text: string;
    position?: "top" | "bottom" | "left" | "right";
    children?: import("svelte").Snippet;
  } = $props();

  const posClass = {
    top: "bottom-full left-1/2 -translate-x-1/2 mb-1",
    bottom: "top-full left-1/2 -translate-x-1/2 mt-1",
    left: "right-full top-1/2 -translate-y-1/2 mr-1",
    right: "left-full top-1/2 -translate-y-1/2 ml-1",
  };
</script>

<span class="relative inline-flex group" tabindex={0}>
  {#if children}{@render children()}{/if}
  <span
    class="pointer-events-none absolute {posClass[position]} px-2 py-1 text-xs bg-background-input border border-line text-content rounded whitespace-nowrap opacity-0 group-hover:opacity-100 group-focus:opacity-100 transition-opacity z-50"
    role="tooltip"
  >
    {text}
  </span>
</span>
