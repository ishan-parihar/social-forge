<script lang="ts">
  import type { MediaItem } from "$lib/api/media";

  let {
    items = [],
    loading = false,
    selectable = false,
    selectedIds = [],
    onSelect,
    onDelete,
  }: {
    items?: MediaItem[];
    loading?: boolean;
    selectable?: boolean;
    selectedIds?: string[];
    onSelect?: (item: MediaItem) => void;
    onDelete?: (id: string) => void;
  } = $props();

  let deleting = $state<string | null>(null);

  $effect(() => {
    if (deleting !== null) {
      const handler = () => { deleting = null; };
      document.addEventListener("click", handler);
      return () => document.removeEventListener("click", handler);
    }
  });

  function confirmDelete(id: string) {
    if (deleting === id) {
      onDelete?.(id);
      deleting = null;
    } else {
      deleting = id;
    }
  }

  function isSelected(id: string) {
    return selectedIds.includes(id);
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1048576) return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / 1048576).toFixed(1) + " MB";
  }

  function formatDate(iso: string | undefined): string {
    if (!iso) return "";
    const d = new Date(iso);
    const now = new Date();
    const diff = now.getTime() - d.getTime();
    const days = Math.floor(diff / 86400000);
    if (days === 0) return "Today";
    if (days === 1) return "Yesterday";
    if (days < 7) return `${days} days ago`;
    return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  function fileIcon(mime: string): string {
    if (mime.startsWith("image/")) return "";
    if (mime.startsWith("video/")) return "🎬";
    if (mime.startsWith("audio/")) return "🎵";
    if (mime.includes("pdf")) return "📄";
    return "📁";
  }
</script>

{#if loading}
  <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
    {#each Array(10) as _, i (i)}
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden animate-pulse">
        <div class="w-full aspect-square bg-[#1a1f2e]"></div>
        <div class="p-2 space-y-1.5">
          <div class="h-3 bg-[#1a1f2e] rounded w-3/4"></div>
          <div class="h-2.5 bg-[#1a1f2e] rounded w-1/2"></div>
        </div>
      </div>
    {/each}
  </div>
{:else if items.length === 0}
  <div class="text-center py-16 text-sm text-[#6b7280]">
    <div class="text-3xl mb-3">📁</div>
    <p>No media uploaded yet</p>
  </div>
{:else}
  <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 gap-3">
    {#each items as item (item.id)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        role={selectable ? "button" : undefined}
        tabindex={selectable ? 0 : undefined}
        onclick={selectable ? () => onSelect?.(item) : undefined}
        onkeydown={selectable ? (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onSelect?.(item); } } : undefined}
        class="relative group bg-[#131720] border rounded-xl overflow-hidden transition-all duration-150"
        class:cursor-pointer={selectable}
        class:border-indigo-400={selectable && isSelected(item.id)}
        class:border-[#1e2435]={!selectable || !isSelected(item.id)}
        class:ring-1={selectable && isSelected(item.id)}
        class:ring-indigo-400={selectable && isSelected(item.id)}
      >
        {#if item.mime_type.startsWith("image/")}
          <img
            src={item.url}
            alt={item.original_name}
            class="w-full aspect-square object-cover"
            loading="lazy"
          />
        {:else}
          <div class="w-full aspect-square bg-[#1a1f2e] flex items-center justify-center text-3xl">
            {fileIcon(item.mime_type)}
          </div>
        {/if}

        <div class="p-2">
          <p class="text-xs text-[#d1d5db] truncate" title={item.original_name}>{item.original_name}</p>
          <p class="text-[10px] text-[#6b7280]">{formatSize(item.file_size)} &middot; {formatDate(item.created_at)}</p>
        </div>

        {#if onDelete}
          <button
            aria-label="Delete media"
            onclick={(e) => { e.stopPropagation(); confirmDelete(item.id); }}
            class="absolute top-1.5 right-1.5 w-6 h-6 rounded-full flex items-center justify-center text-xs transition-all duration-150"
            class:bg-red-500={deleting !== item.id}
            class:bg-red-600={deleting === item.id}
            class:opacity-0={deleting !== item.id}
            class:group-hover:opacity-100={deleting !== item.id}
            class:opacity-100={deleting === item.id}
          >
            {deleting === item.id ? "✓" : "×"}
          </button>
        {/if}

        {#if selectable && isSelected(item.id)}
          <div class="absolute top-1.5 left-1.5 w-5 h-5 bg-indigo-500 rounded-full flex items-center justify-center">
            <span class="text-white text-[10px]">✓</span>
          </div>
        {/if}
      </div>
    {/each}
  </div>
{/if}
