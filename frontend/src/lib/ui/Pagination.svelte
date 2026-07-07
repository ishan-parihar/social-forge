<script lang="ts">
  // v22 Phase 3: Pagination primitive — page numbers + prev/next.
  // Replaces ad-hoc pagination in posts list, feed, search.
  let {
    page = $bindable(),
    totalPages,
    maxVisible = 7,
  }: {
    page: number;
    totalPages: number;
    maxVisible?: number;
  } = $props();

  // Build a windowed page list: show first, last, and a window around current.
  let pages = $derived.by(() => {
    if (totalPages <= maxVisible) {
      return Array.from({ length: totalPages }, (_, i) => i + 1);
    }
    const half = Math.floor(maxVisible / 2);
    let start = Math.max(1, page - half);
    let end = Math.min(totalPages, start + maxVisible - 1);
    start = Math.max(1, end - maxVisible + 1);
    const arr: (number | "...")[] = [];
    if (start > 1) {
      arr.push(1);
      if (start > 2) arr.push("...");
    }
    for (let i = start; i <= end; i++) arr.push(i);
    if (end < totalPages) {
      if (end < totalPages - 1) arr.push("...");
      arr.push(totalPages);
    }
    return arr;
  });
</script>

{#if totalPages > 1}
  <nav class="flex items-center gap-1 text-sm" aria-label="Pagination">
    <button
      onclick={() => (page = Math.max(1, page - 1))}
      disabled={page <= 1}
      class="px-2 py-1 rounded text-muted hover:text-content hover:bg-surface-hover disabled:opacity-40 disabled:hover:bg-transparent"
      aria-label="Previous page"
    >
      ‹
    </button>
    {#each pages as p (p)}
      {#if p === "..."}
        <span class="px-2 text-muted">…</span>
      {:else}
        <button
          onclick={() => (page = p)}
          class="min-w-[28px] px-2 py-1 rounded transition-colors {p === page ? 'bg-brand-500 text-white' : 'text-muted hover:text-content hover:bg-surface-hover'}"
          aria-current={p === page ? "page" : undefined}
        >
          {p}
        </button>
      {/if}
    {/each}
    <button
      onclick={() => (page = Math.min(totalPages, page + 1))}
      disabled={page >= totalPages}
      class="px-2 py-1 rounded text-muted hover:text-content hover:bg-surface-hover disabled:opacity-40 disabled:hover:bg-transparent"
      aria-label="Next page"
    >
      ›
    </button>
  </nav>
{/if}
