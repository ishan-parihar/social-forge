<script lang="ts">
  import { tick } from "svelte";
  import { mediaApi, type MediaItem } from "$lib/api/media";
  import MediaGrid from "./MediaGrid.svelte";

  let {
    open = false,
    onClose,
    onSelect,
  }: {
    open?: boolean;
    onClose?: () => void;
    onSelect?: (url: string) => void;
  } = $props();

  let items = $state<MediaItem[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let dialogEl = $state<HTMLDivElement | null>(null);
  let closeBtn = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    if (open) {
      fetchMedia();
      tick().then(() => {
        closeBtn?.focus();
      });
    }
  });

  async function fetchMedia() {
    loading = true;
    error = null;
    const r = await mediaApi.list({ limit: 100 });
    if (r.data) {
      items = r.data;
    } else {
      error = r.error || "Failed to load media";
    }
    loading = false;
  }

  function handleSelect(item: MediaItem) {
    onSelect?.(item.url);
    onClose?.();
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onClose?.();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose?.();
      return;
    }
    if (e.key === "Tab") {
      const focusable = dialogEl?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      );
      if (!focusable || focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    role="dialog"
    aria-modal="true"
    aria-label="Choose from Library"
    tabindex="-1"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
  >
    <div
      bind:this={dialogEl}
      tabindex="-1"
      class="bg-surface border border-line rounded-xl w-full max-w-3xl max-h-[80vh] flex flex-col shadow-2xl"
    >
      <div class="flex items-center justify-between px-5 py-4 border-b border-line">
        <h3 class="text-base font-semibold text-content">Choose from Library</h3>
        <button
          bind:this={closeBtn}
          aria-label="Close"
          onclick={onClose}
          class="w-7 h-7 rounded-lg flex items-center justify-center text-muted hover:text-white hover:bg-surface-hover transition-colors text-sm"
        >
          &times;
        </button>
      </div>

      <div class="flex-1 overflow-y-auto p-5">
        {#if error}
          <div class="text-center py-12">
            <p class="text-sm text-red-400 mb-3">{error}</p>
            <button onclick={fetchMedia} class="text-sm text-indigo-400 hover:text-indigo-300">Retry</button>
          </div>
        {:else}
          <MediaGrid {items} {loading} selectable={true} onSelect={handleSelect} />
        {/if}
      </div>
    </div>
  </div>
{/if}
