<script lang="ts">
  import { focusTrap } from './focus-trap';

  let { open = false, title = "", onclose, children }: {
    open?: boolean; title?: string; onclose?: () => void;
    children?: import("svelte").Snippet;
  } = $props();

  // F5: stable id so the dialog can name itself. Without it the accessible
  // name was whatever a screen reader inferred from the contents, which for a
  // form modal is nothing useful.
  const uid = $props.id();
  const titleId = `modal-title-${uid}`;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && onclose) onclose();
  }

  // Body scroll lock. Kept from the pre-F5 version: focus trapping does not
  // stop the wheel from scrolling the page behind the scrim.
  $effect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => { document.body.style.overflow = prev; };
  });
</script>

{#if open}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center"
    role="dialog"
    aria-modal="true"
    aria-labelledby={title ? titleId : undefined}
    aria-label={title ? undefined : "Dialog"}
    tabindex="-1"
    use:focusTrap
    onkeydown={handleKeydown}
  >
    <div class="absolute inset-0 bg-overlay" aria-hidden="true" onclick={onclose}></div>
    <div class="relative bg-surface border border-line rounded-xl p-6 max-w-lg w-full mx-4 shadow-lg">
      {#if title}
        <div class="flex items-center justify-between mb-4">
          <h3 id={titleId} class="text-lg font-semibold">{title}</h3>
          <button onclick={onclose} aria-label="Close dialog" class="text-muted hover:text-content">&times;</button>
        </div>
      {/if}
      {#if children}{@render children()}{/if}
    </div>
  </div>
{/if}
