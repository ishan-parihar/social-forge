<script lang="ts">
  // v22 Phase 3: Card primitive — surface bg, border, padding, optional
  // header/footer snippets. Replaces ad-hoc `<div class="bg-surface
  // border border-line rounded-xl p-5">` patterns scattered across routes.
  let {
    padding = "md",
    hover = false,
    children,
    header,
    footer,
  }: {
    padding?: "none" | "sm" | "md" | "lg";
    hover?: boolean;
    children?: import("svelte").Snippet;
    header?: import("svelte").Snippet;
    footer?: import("svelte").Snippet;
  } = $props();

  const padClass = {
    none: "",
    sm: "p-3",
    md: "p-5",
    lg: "p-7",
  };
</script>

<div
  class="bg-surface border border-line rounded-lg {padClass[padding]} {hover ? 'transition-colors hover:border-line-hover' : ''}"
>
  {#if header}
    <div class="mb-4 pb-3 border-b border-line">
      {@render header()}
    </div>
  {/if}
  {#if children}{@render children()}{/if}
  {#if footer}
    <div class="mt-4 pt-3 border-t border-line">
      {@render footer()}
    </div>
  {/if}
</div>
