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
  .btn { font-weight: 500; border-radius: 0.5rem; transition: all 0.15s; cursor: pointer; border: none; }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn-primary { background: #6366f1; color: white; }
  .btn-primary:hover:not(:disabled) { background: #5558e6; }
  .btn-secondary { background: #1e2435; color: #d1d5db; border: 1px solid #2a3045; }
  .btn-secondary:hover:not(:disabled) { background: #2a3045; }
  .btn-ghost { background: transparent; color: #6b7280; }
  .btn-ghost:hover:not(:disabled) { background: #1a1f2e; color: #d1d5db; }
  .btn-danger { background: #dc2626; color: white; }
  .btn-danger:hover:not(:disabled) { background: #b91c1c; }
  .btn-sm { padding: 0.375rem 0.75rem; font-size: 0.75rem; }
  .btn-md { padding: 0.5rem 1rem; font-size: 0.875rem; }
  .btn-lg { padding: 0.75rem 1.5rem; font-size: 1rem; }
</style>
