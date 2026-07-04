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
    onInsert?.(sig.content);
    open = false;
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
  <button
    onclick={toggle}
    aria-label="Insert signature"
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
      class="absolute top-full left-0 mt-1 w-72 bg-surface border border-line rounded-lg shadow-xl z-50 max-h-80 overflow-y-auto"
      role="listbox"
      aria-label="Select a signature"
    >
      {#if loading}
        <div class="flex justify-center py-6">
          <Spinner size="sm" />
        </div>
      {:else if error}
        <div class="text-sm text-red-400 p-3">{error}</div>
      {:else if signatures.length === 0}
        <div class="text-sm text-muted p-4 text-center">
          No signatures yet — <a href="/settings/signatures" class="text-indigo-400 hover:underline" onclick={() => (open = false)}>create one</a> in Settings
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
              aria-selected="false"
              aria-label={sig.name}
            >
              <div class="text-sm text-content-secondary truncate">{sig.name}</div>
              <div class="text-xs text-muted truncate mt-0.5">{sig.content.slice(0, 60)}{sig.content.length > 60 ? '...' : ''}</div>
            </button>
          {/each}
        {/if}
        <!-- Provider-specific signatures -->
        {#each [...grouped.byProvider.entries()] as [provider, sigs] (provider)}
          <div class="px-3 pt-2 pb-1 text-xs text-muted font-semibold uppercase tracking-wider">{provider}</div>
          {#each sigs as sig (sig.id)}
            <button
              onclick={() => handleSelect(sig)}
              class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors"
              role="option"
              aria-selected="false"
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
    color: #9ca3af; font-weight: 500;
  }
  .toolbar-btn:hover { background: #1a1f2e; color: #e5e7eb; }
  .toolbar-btn.active { background: #6366f1; color: white; border-color: #6366f1; }
</style>
