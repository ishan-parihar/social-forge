<script lang="ts">
  let { open = false, title = "", onclose, children }: {
    open?: boolean; title?: string; onclose?: () => void;
    children?: import("svelte").Snippet;
  } = $props();

  let dialogEl = $state<HTMLDivElement | null>(null);
  let prevFocus: HTMLElement | null = null;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && onclose) onclose();
  }

  $effect(() => {
    if (open) {
      prevFocus = document.activeElement as HTMLElement;
      requestAnimationFrame(() => dialogEl?.focus());
    } else if (prevFocus) {
      prevFocus.focus();
      prevFocus = null;
    }
  });

  $effect(() => {
    if (!open) return;
    const prev = document.body.style.overflow;
    document.body.style.overflow = "hidden";
    return () => { document.body.style.overflow = prev; };
  });
</script>

{#if open}
  <div
    bind:this={dialogEl}
    tabindex="-1"
    class="fixed inset-0 z-50 flex items-center justify-center"
    role="dialog"
    aria-modal="true"
    onkeydown={handleKeydown}
  >
    <div class="absolute inset-0 bg-black/60" aria-hidden="true" onclick={onclose}></div>
    <div class="relative bg-[#131720] border border-[#1e2435] rounded-xl p-6 max-w-lg w-full mx-4 shadow-2xl">
      {#if title}
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-lg font-semibold">{title}</h3>
          <button onclick={onclose} aria-label="Close dialog" class="text-[#6b7280] hover:text-white">&times;</button>
        </div>
      {/if}
      {#if children}{@render children()}{/if}
    </div>
  </div>
{/if}
