<script lang="ts" module>
  // v22 Phase 3: Badge now supports both state-based and generic-variant
  // styling. Use `state="published"` for post-state badges, or
  // `variant="success"` for generic status badges. The state-based map
  // uses semantic tokens (bg-success/20 text-success) so badges retheme
  // correctly in light mode - previously they used Tailwind's hardcoded
  // `green-500`/`red-500` which don't retheme.
  export type BadgeState = "draft" | "queued" | "published" | "error" | "idea";
  export type BadgeVariant = "default" | "success" | "warning" | "error" | "info";
</script>

<script lang="ts">
  let {
    state,
    variant,
    children,
  }: {
    state?: BadgeState;
    variant?: BadgeVariant;
    children?: import("svelte").Snippet;
  } = $props();

  // State → variant mapping (post lifecycle states). The text is the content
  // colour: the status hue on its 20% tint measured 3.9 to 3.95:1 at 12 px, and
  // no tint of the token reaches 4.5:1. The hue stays on a ring instead.
  const stateClass: Record<BadgeState, string> = {
    draft: "bg-info/20 ring-1 ring-inset ring-info/40 text-content",
    queued: "bg-warning/20 ring-1 ring-inset ring-warning/40 text-content",
    published: "bg-success/20 ring-1 ring-inset ring-success/40 text-content",
    error: "bg-error/20 ring-1 ring-inset ring-error/40 text-content",
    idea: "bg-hue-violet/20 ring-1 ring-inset ring-hue-violet/40 text-content",
  };

  // Generic variant mapping (for non-post-state badges like "Active",
  // "Paused", "Verified", etc.).
  const variantClass: Record<BadgeVariant, string> = {
    default: "bg-muted/20 text-muted",
    success: "bg-success/20 ring-1 ring-inset ring-success/40 text-content",
    warning: "bg-warning/20 ring-1 ring-inset ring-warning/40 text-content",
    error: "bg-error/20 ring-1 ring-inset ring-error/40 text-content",
    info: "bg-info/20 ring-1 ring-inset ring-info/40 text-content",
  };

  let cls = $derived(
    state ? stateClass[state] : variantClass[variant ?? "default"]
  );
</script>

<span class="px-2 py-0.5 rounded text-xs font-medium {cls}">
  {#if children}{@render children()}{:else if state}{state}{/if}
</span>
