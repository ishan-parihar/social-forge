<script lang="ts">
  import { mediaApi, type MediaItem } from "$lib/api/media";
  import MediaGrid from "$lib/media/MediaGrid.svelte";
  import { toast } from "$lib/stores/toast";

  let items = $state<MediaItem[]>([]);
  let loading = $state(true);
  let uploading = $state(false);
  let error = $state<string | null>(null);
  let search = $state("");
  let debouncedSearch = $state("");
  let offset = $state(0);
  let hasMore = $state(false);
  const limit = 50;
  const apiLimit = limit + 1;

  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      debouncedSearch = search;
      offset = 0;
    }, 300);
    return () => { if (timer) clearTimeout(timer); };
  });

  $effect(() => {
    fetchMedia();
  });

  async function fetchMedia() {
    loading = true;
    error = null;
    const r = await mediaApi.list({
      limit: apiLimit,
      offset,
      search: debouncedSearch || undefined,
    });
    if (r.data) {
      let fetched = r.data;
      hasMore = fetched.length > limit;
      if (hasMore) fetched.pop();
      items = fetched;
    } else {
      error = r.error || "Failed to load media";
      hasMore = false;
    }
    loading = false;
  }

  async function handleUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    uploading = true;
    let uploaded = 0;
    for (const file of input.files) {
      const r = await mediaApi.upload(file);
      if (r.data) {
        items = [r.data, ...items];
        uploaded++;
      }
    }
    uploading = false;
    input.value = "";
    if (uploaded > 0) toast(`Uploaded ${uploaded} file(s)`, "success");
  }

  async function handleDelete(id: string) {
    const r = await mediaApi.delete(id);
    if (r.data?.deleted) {
      items = items.filter((m) => m.id !== id);
      toast("Media deleted", "success");
    } else {
      toast(r.error || "Delete failed", "error");
    }
  }

  function nextPage() {
    offset += limit;
  }

  function prevPage() {
    offset = Math.max(0, offset - limit);
  }


</script>

<div class="space-y-5">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold text-[#e8edf5]">Media Library</h2>
    <label class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm cursor-pointer transition-colors disabled:opacity-50" class:opacity-50={uploading}>
      {uploading ? "Uploading..." : "+ Upload"}
      <input type="file" multiple accept="image/*,video/*,audio/*,.pdf" onchange={handleUpload} class="hidden" disabled={uploading} />
    </label>
  </div>

  <div class="relative">
    <input
      type="text"
      placeholder="Search media by name..."
      bind:value={search}
      class="w-full px-3 py-2 pl-9 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db] placeholder-[#6b7280] focus:border-indigo-500 outline-none transition-colors"
    />
    <span class="absolute left-3 top-1/2 -translate-y-1/2 text-[#6b7280] text-sm">🔍</span>
  </div>

  {#if error && !loading}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-4 text-center">
      <p class="mb-2">{error}</p>
      <button onclick={fetchMedia} class="text-indigo-400 hover:text-indigo-300 underline">Retry</button>
    </div>
  {:else}
    <MediaGrid {items} {loading} onDelete={handleDelete} />

    {#if !loading && items.length > 0}
      <div class="flex items-center justify-center gap-3 pt-2">
        <button
          onclick={prevPage}
          disabled={offset === 0}
          class="px-3 py-1.5 text-sm border border-[#1e2435] rounded-lg disabled:opacity-30 disabled:cursor-not-allowed hover:bg-[#1a1f2e] transition-colors text-[#d1d5db]"
        >
          Previous
        </button>
        <span class="text-xs text-[#6b7280]">Page {Math.floor(offset / limit) + 1}</span>
        <button
          onclick={nextPage}
          disabled={!hasMore}
          class="px-3 py-1.5 text-sm border border-[#1e2435] rounded-lg disabled:opacity-30 disabled:cursor-not-allowed hover:bg-[#1a1f2e] transition-colors text-[#d1d5db]"
        >
          Next
        </button>
      </div>
    {/if}
  {/if}
</div>
