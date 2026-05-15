<script lang="ts">
  let { items, align = "left", children }: {
    items: Array<{ label: string; onclick: () => void; variant?: "default" | "danger" }>;
    align?: "left" | "right";
    children?: import("svelte").Snippet;
  } = $props();

  let open = $state(false);
  let menuEl: HTMLDivElement;

  function toggle() { open = !open; }
  function handleClick(fn: () => void) {
    fn();
    open = false;
  }

  $effect(() => {
    if (!open) return;
    function onDocClick(e: MouseEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) open = false;
    }
    document.addEventListener("click", onDocClick);
    return () => document.removeEventListener("click", onDocClick);
  });
</script>

<div class="relative inline-block" bind:this={menuEl}>
  <button onclick={toggle} class="dropdown-trigger">
    {#if children}{@render children()}{/if}
  </button>
  {#if open}
    <div class="dropdown-menu {align === 'right' ? 'right-0' : 'left-0'}" role="menu">
      {#each items as item}
        <button
          role="menuitem"
          onclick={() => handleClick(item.onclick)}
          class="dropdown-item {item.variant === 'danger' ? 'text-red-400' : ''}"
        >
          {item.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .dropdown-trigger { background: none; border: none; cursor: pointer; padding: 0; color: inherit; }
  .dropdown-menu {
    position: absolute; top: 100%; margin-top: 0.25rem; z-index: 50;
    min-width: 10rem; background: #1a1f2e; border: 1px solid #2a3045;
    border-radius: 0.5rem; box-shadow: 0 10px 25px rgba(0,0,0,0.3); padding: 0.25rem;
  }
  .dropdown-item {
    display: block; width: 100%; text-align: left; padding: 0.5rem 0.75rem;
    font-size: 0.875rem; color: #d1d5db; background: none; border: none;
    border-radius: 0.375rem; cursor: pointer;
  }
  .dropdown-item:hover { background: #2a3045; }
</style>
