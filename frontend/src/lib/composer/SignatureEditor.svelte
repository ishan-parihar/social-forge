<script lang="ts">
  import { onMount } from 'svelte';
  import { signaturesApi, type Signature } from '$lib/api/signatures';
  import Spinner from '$lib/ui/Spinner.svelte';

  let { onInsert }: {
    onInsert?: (content: string) => void;
  } = $props();

  let open = $state(false);
  let signatures = $state<Signature[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  // v25 F5: every option's aria-selected was hardcoded "false", so the listbox
  // reported "nothing selected" unconditionally. It is the applied signature,
  // so it has to be remembered across the close.
  let selectedId = $state<string | null>(null);
  let triggerEl: HTMLButtonElement | undefined = $state();

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      const r = await signaturesApi.list();
      if (r.error) {
        error = r.error;
      } else if (r.data) {
        signatures = r.data || [];
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to load signatures';
    } finally {
      loading = false;
    }
  }

  function handleSelect(sig: Signature) {
    selectedId = sig.id;
    onInsert?.(sig.content);
    open = false;
    triggerEl?.focus();
  }

  function toggle() {
    open = !open;
    if (open) load();
  }

  // Group signatures by provider
  let grouped = $derived.by(() => {
    const global: Signature[] = [];
    const byProvider = new Map<string, Signature[]>();
    for (const s of signatures) {
      if (!s.provider) {
        global.push(s);
      } else {
        const arr = byProvider.get(s.provider) ?? [];
        arr.push(s);
        byProvider.set(s.provider, arr);
      }
    }
    return { global, byProvider };
  });
</script>

<div class="relative">
  <!-- v25 F5: the trigger never said a listbox was behind it, and the listbox
       itself had no keydown handler — no Escape, no arrows, and focus never
       left the trigger, so the panel was only reachable by blind Tabbing. -->
  <button
    onclick={toggle}
    bind:this={triggerEl}
    aria-label="Insert signature"
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls="sig-listbox"
    class="toolbar-btn"
    class:active={open}
  >
    <span aria-hidden="true">📝</span>
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-40"
      role="none"
      onclick={() => (open = false)}
    ></div>
    <div
      class="absolute top-full left-0 mt-1 w-[min(18rem,calc(100vw-2rem))] bg-surface border border-line rounded-lg shadow-lg z-50 max-h-80 overflow-y-auto"
      role="listbox"
      id="sig-listbox"
      aria-label="Select a signature"
      tabindex="-1"
      onkeydown={(e) => {
        if (e.key === 'Escape') { e.preventDefault(); open = false; triggerEl?.focus(); }
      }}
    >
      {#if loading}
        <div class="flex justify-center py-6">
          <Spinner size="sm" />
        </div>
      {:else if error}
        <div class="text-sm text-error p-3">{error}</div>
      {:else if signatures.length === 0}
        <div class="text-sm text-muted p-4 text-center">
          No signatures yet — <a href="/settings/signatures" class="text-accent hover:underline" onclick={() => (open = false)}>create one</a> in Settings
        </div>
      {:else}
        <!-- Global signatures -->
        {#if grouped.global.length > 0}
          <div class="px-3 pt-2 pb-1 text-xs text-muted font-semibold uppercase tracking-wider">Global</div>
          {#each grouped.global as sig (sig.id)}
<button
            onclick={() => handleSelect(sig)}
            class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors"
            role="option"
            aria-selected={selectedId === sig.id}
            aria-label={sig.name}
          >
            <div class="text-sm text-content-secondary truncate">{sig.name}</div>
            <div class="text-xs text-muted truncate mt-0.5">{sig.content.slice(0, 60)}{sig.content.length > 60 ? '...' : ''}</div>
          </button>
          {/each}
        {/if}
        <!-- Provider-specific signatures -->
        {#each [...grouped.byProvider.entries()] as [provider, sigs] (provider)}
          <div class="px-3 pt-2 pb-1 text-xs text-muted font-semibold uppercase tracking-wider" role="presentation">{provider}</div>
          {#each sigs as sig (sig.id)}
            <button
              onclick={() => handleSelect(sig)}
              class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors"
              role="option"
              aria-selected={selectedId === sig.id}
              aria-label={sig.name}
            >
              <div class="text-sm text-content-secondary truncate">{sig.name}</div>
              <div class="text-xs text-muted truncate mt-0.5">{sig.content.slice(0, 60)}{sig.content.length > 60 ? '...' : ''}</div>
            </button>
          {/each}
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .toolbar-btn {
    padding: 0.25rem 0.5rem; font-size: 0.8rem; background: transparent;
    border: 1px solid transparent; border-radius: 0.25rem; cursor: pointer;
    color: var(--text-muted); font-weight: 500;
  }
  .toolbar-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .toolbar-btn.active { background: var(--brand); color: white; border-color: var(--brand); }
</style>
