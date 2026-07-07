<script lang="ts" module>
  // v22 Phase 3: Badge now supports both state-based and generic-variant
  // styling. Use `state="published"` for post-state badges, or
  // `variant="success"` for generic status badges. The state-based map
  // uses semantic tokens (bg-success/20 text-success) so badges retheme
  // correctly in light mode — previously they used Tailwind's hardcoded
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

  // State → variant mapping (post lifecycle states).
  const stateClass: Record<BadgeState, string> = {
    draft: "bg-info/20 text-info",
    queued: "bg-warning/20 text-warning",
    published: "bg-success/20 text-success",
    error: "bg-error/20 text-error",
    idea: "bg-purple-500/20 text-purple-300",
  };

  // Generic variant mapping (for non-post-state badges like "Active",
  // "Paused", "Verified", etc.).
  const variantClass: Record<BadgeVariant, string> = {
    default: "bg-muted/20 text-muted",
    success: "bg-success/20 text-success",
    warning: "bg-warning/20 text-warning",
    error: "bg-error/20 text-error",
    info: "bg-info/20 text-info",
  };

  let cls = $derived(
    state ? stateClass[state] : variantClass[variant ?? "default"]
  );
</script>

<span class="px-2 py-0.5 rounded text-xs font-medium {cls}">
  {#if children}{@render children()}{:else if state}{state}{/if}
</span>
