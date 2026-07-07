<script lang="ts">
  let { variant = "primary", size = "md", disabled = false, onclick, children, type = "button", "aria-label": ariaLabel }: {
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    onclick?: (e: MouseEvent) => void;
    children?: import("svelte").Snippet;
    type?: "button" | "submit" | "reset";
    "aria-label"?: string;
  } = $props();
</script>

<button {type} {onclick} {disabled} aria-label={ariaLabel} class="btn btn-{variant} btn-{size}">
  {#if children}{@render children()}{/if}
</button>

<style>
  /* v22 Phase 3: replaced hardcoded hex with CSS variables so buttons
     retheme correctly in light mode. Previously the primary CTA used
     `#6366f1` (the dark-mode brand color) which made light-mode buttons
     look identical to dark-mode — defeating the theme toggle. */
  .btn { font-weight: 500; border-radius: var(--radius-md); transition: all 0.15s; cursor: pointer; border: none; }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-primary { background: var(--brand); color: white; }
  .btn-primary:hover:not(:disabled) { background: var(--brand-hover); }
  .btn-secondary { background: var(--bg-card); color: var(--text-secondary); border: 1px solid var(--border); }
  .btn-secondary:hover:not(:disabled) { background: var(--bg-hover); border-color: var(--border-hover); }
  .btn-ghost { background: transparent; color: var(--text-muted); }
  .btn-ghost:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
  .btn-danger { background: rgb(var(--error-rgb)); color: white; }
  .btn-danger:hover:not(:disabled) { background: rgb(var(--error-rgb) / 0.85); }
  .btn-sm { padding: 0.375rem 0.75rem; font-size: 0.75rem; }
  .btn-md { padding: 0.5rem 1rem; font-size: 0.875rem; }
  .btn-lg { padding: 0.75rem 1.5rem; font-size: 1rem; }
</style>
