<script lang="ts">
  import { tick } from "svelte";

  let { items, align = "left", children }: {
    items: Array<{ label: string; onclick: () => void; variant?: "default" | "danger"; disabled?: boolean }>;
    align?: "left" | "right";
    children?: import("svelte").Snippet;
  } = $props();

  let open = $state(false);
  let menuEl: HTMLDivElement;
  let triggerEl: HTMLButtonElement | undefined = $state();
  // Roving cursor over the items. `role="menu"` promises that arrow keys walk
  // the list and that Tab leaves it; before F5 the items were ordinary tab
  // stops, so both promises were false.
  let cursor = $state(0);

  const uid = $props.id();
  const menuId = `dd-menu-${uid}`;

  function toggle() { open = !open; }

  function handleClick(fn: () => void) {
    fn();
    open = false;
    triggerEl?.focus();
  }

  function close(returnFocus = true) {
    open = false;
    if (returnFocus) triggerEl?.focus();
  }

  function move(delta: number) {
    const enabled = items.map((it, i) => (it.disabled ? -1 : i)).filter((i) => i >= 0);
    if (enabled.length === 0) return;
    const at = enabled.indexOf(cursor);
    const next = enabled[(at + delta + enabled.length) % enabled.length];
    cursor = next;
    tick().then(() => {
      menuEl?.querySelectorAll<HTMLElement>('[role="menuitem"]')[next]?.focus();
    });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") { e.preventDefault(); move(1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); move(-1); }
    else if (e.key === "Home") { e.preventDefault(); move(-items.length); }
    else if (e.key === "End") { e.preventDefault(); move(items.length); }
    else if (e.key === "Escape" && open) { e.preventDefault(); close(); }
    else if (e.key === "Tab" && open) {
      // Tabbing out of a menu dismisses it; without this the menu stays open
      // behind whatever the user moved on to.
      close(false);
    }
  }

  $effect(() => {
    if (!open) return;
    cursor = 0;
    function onDocClick(e: MouseEvent) {
      if (menuEl && !menuEl.contains(e.target as Node)) close(false);
    }
    document.addEventListener("click", onDocClick);
    return () => document.removeEventListener("click", onDocClick);
  });

  // Opening from the keyboard should land on the first item, which is what
  // every menu on the platform does. Mouse opening leaves focus on the trigger
  // so the two input modes behave differently on purpose.
  $effect(() => {
    if (!open) return;
    tick().then(() => {
      if (!open) return;
      menuEl?.querySelector<HTMLElement>('[role="menuitem"]:not([disabled])')?.focus();
    });
  });
</script>

<!-- role="presentation": the wrapper is not itself interactive — it only
     delegates the keydown so the trigger and the menu share one handler. -->
<div class="relative inline-block" bind:this={menuEl} role="presentation" onkeydown={onKeydown}>
  <!-- v25 F5: the trigger declared `role="menu"` was never paired with
       aria-haspopup/aria-expanded, so assistive tech had no way to know a menu
       was behind this button. The component also did not guarantee an
       accessible name — the one call site supplied one in its snippet. -->
  <button
    bind:this={triggerEl}
    onclick={toggle}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={open ? menuId : undefined}
    class="dropdown-trigger"
  >
    {#if children}{@render children()}{/if}
  </button>
  {#if open}
    <div id={menuId} class="dropdown-menu {align === 'right' ? 'right-0' : 'left-0'}" role="menu">
      {#each items as item, i (item.label)}
        <button
          role="menuitem"
          tabindex={i === cursor ? 0 : -1}
          onfocus={() => (cursor = i)}
          onclick={() => handleClick(item.onclick)}
          disabled={item.disabled}
          class="dropdown-item {item.variant === 'danger' ? 'text-error' : ''} {item.disabled ? 'opacity-50 cursor-not-allowed' : ''}"
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
    min-width: 10rem; max-width: calc(100vw - 2rem); background: var(--bg-card); border: 1px solid var(--border);
    border-radius: 0.5rem; box-shadow: var(--shadow-lg); padding: 0.25rem;
  }
  .dropdown-item {
    display: block; width: 100%; text-align: left; padding: 0.5rem 0.75rem;
    font-size: 0.875rem; color: var(--text-secondary); background: none; border: none;
    border-radius: 0.375rem; cursor: pointer;
  }
  .dropdown-item:hover { background: var(--bg-hover); }
</style>
