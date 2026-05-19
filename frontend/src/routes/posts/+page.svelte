<script lang="ts">
  import { onMount } from "svelte";
  import { postsApi, type PostSummary } from "$lib/api/posts";
  import Badge from "$lib/ui/Badge.svelte";
  import { goto } from "$app/navigation";

  let posts = $state<PostSummary[]>([]);
  let filter = $state("all");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let page = $state(1);
  let totalPages = $state(1);
  let totalItems = $state(0);
  const limit = 20;

  // Bulk selection
  let selected = $state<Set<string>>(new Set());
  let bulkScheduleDate = $state("");
  let bulkScheduleTime = $state("09:00");
  let showBulkSchedule = $state(false);
  let bulkProcessing = $state(false);

  function toggleSelect(id: string, e: Event) {
    e.stopPropagation();
    const s = new Set(selected);
    if (s.has(id)) s.delete(id); else s.add(id);
    selected = s;
  }

  function toggleAll() {
    if (selected.size === posts.length) selected = new Set();
    else selected = new Set(posts.map(p => p.id));
  }

  async function bulkDelete() {
    if (!confirm(`Delete ${selected.size} post(s)?`)) return;
    bulkProcessing = true;
    for (const id of selected) { await postsApi.delete(id); }
    selected = new Set();
    bulkProcessing = false;
    load();
  }

  async function bulkReschedule() {
    if (!bulkScheduleDate) return;
    bulkProcessing = true;
    const iso = `${bulkScheduleDate}T${bulkScheduleTime}:00.000Z`;
    for (const id of selected) { await postsApi.schedule(id, iso); }
    selected = new Set();
    showBulkSchedule = false;
    bulkProcessing = false;
    load();
  }

  async function load() {
    loading = true;
    error = null;
    const params: Record<string, string | number> = { limit, offset: (page - 1) * limit };
    if (filter !== "all") params.state = filter;
    const r = await postsApi.list(params);
    if (r.data) {
      posts = r.data.posts;
      totalItems = r.data.total;
      totalPages = Math.ceil(r.data.total / limit);
    } else {
      error = r.error || "Failed to load posts";
    }
    loading = false;
  }

  function toggleFilter(f: string) {
    filter = f;
    page = 1;
    selected = new Set();
    load();
  }

  function handlePageChange(p: number) {
    page = p;
    selected = new Set();
    load();
  }

  onMount(load);

  const filters = ["all", "draft", "queued", "published", "error"];
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Posts</h2>
    <button onclick={() => goto("/posts/new")} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Post</button>
  </div>

  <!-- Filter tabs -->
  <div class="flex gap-1 bg-[#131720] border border-[#1e2435] rounded-lg p-1 overflow-x-auto">
    {#each filters as f}
      <button
        onclick={() => toggleFilter(f)}
        class="px-3 py-1.5 text-xs capitalize rounded-md transition-colors {filter === f ? 'bg-indigo-600 text-white' : 'text-[#6b7280] hover:bg-[#1a1f2e]'}"
      >{f}</button>
    {/each}
  </div>

  <!-- Post list -->
  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-[#6b7280]">Loading...</div>
  {:else if posts.length === 0}
    <div class="text-center py-12 text-sm text-[#6b7280]">No posts found</div>
  {:else}
    <!-- Bulk action bar -->
    {#if selected.size > 0}
      <div class="flex items-center gap-3 bg-indigo-600/10 border border-indigo-500/30 rounded-lg px-4 py-2">
        <span class="text-sm text-indigo-300">{selected.size} selected</span>
        <button onclick={() => showBulkSchedule = !showBulkSchedule} disabled={bulkProcessing} class="px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">Reschedule</button>
        <button onclick={bulkDelete} disabled={bulkProcessing} class="px-3 py-1 text-xs bg-red-600 hover:bg-red-500 rounded disabled:opacity-50">Delete</button>
        <button onclick={() => selected = new Set()} class="ml-auto text-xs text-[#6b7280] hover:text-white">Clear</button>
      </div>
      {#if showBulkSchedule}
        <div class="flex items-center gap-2 bg-[#0d1117] border border-[#2a3045] rounded-lg p-3">
          <input type="date" bind:value={bulkScheduleDate} class="px-2 py-1 bg-[#131720] border border-[#1e2435] rounded text-sm text-[#d1d5db]" />
          <input type="time" bind:value={bulkScheduleTime} class="px-2 py-1 bg-[#131720] border border-[#1e2435] rounded text-sm text-[#d1d5db]" />
          <button onclick={bulkReschedule} disabled={bulkProcessing || !bulkScheduleDate} class="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 rounded text-xs disabled:opacity-50">Apply</button>
        </div>
      {/if}
    {/if}

    <div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
      <!-- Select all header -->
      <div class="flex items-center gap-3 px-4 py-2 border-b border-[#1e2435] bg-[#0d1117]">
        <input type="checkbox" checked={selected.size === posts.length && posts.length > 0} onchange={toggleAll} class="rounded" />
        <span class="text-xs text-[#6b7280]">Select all</span>
      </div>
      {#each posts as post (post.id)}
        <div class="flex items-center gap-3 px-4 py-3 border-b border-[#1e2435] last:border-0 hover:bg-[#1a1f2e] transition-colors">
          <input type="checkbox" checked={selected.has(post.id)} onchange={(e) => toggleSelect(post.id, e)} class="rounded shrink-0" />
          <button
            onclick={() => goto(`/posts/${post.id}`)}
            class="flex-1 flex items-center gap-4 text-left min-w-0"
          >
            <div class="flex-1 min-w-0">
              <div class="text-sm truncate">{post.content}</div>
              <div class="text-xs text-[#6b7280] mt-0.5">{post.integration_name}</div>
            </div>
            <div class="text-xs text-[#6b7280] shrink-0">
              {post.scheduled_at ? new Date(post.scheduled_at).toLocaleDateString() : ""}
            </div>
            <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
            {#if post.error_message}
              <span role="img" aria-label={post.error_message} class="text-xs text-red-400" title={post.error_message}>⚠</span>
            {/if}
          </button>
        </div>
      {/each}
    </div>

    {#if totalPages > 1}
      <div class="flex items-center justify-between px-4 py-3 bg-[#131720] border border-[#1e2435] rounded-xl">
        <span class="text-sm text-[#6b7280]">
          Showing {(page - 1) * limit + 1}–{Math.min(page * limit, totalItems)} of {totalItems}
        </span>
        <div class="flex gap-2">
          <button
            onclick={() => handlePageChange(page - 1)}
            disabled={page <= 1}
            class="px-3 py-1 text-sm rounded bg-[#1e2435] text-[#d1d5db] disabled:opacity-50 hover:bg-[#2a3045] transition-colors"
          >← Previous</button>
          <button
            onclick={() => handlePageChange(page + 1)}
            disabled={page >= totalPages}
            class="px-3 py-1 text-sm rounded bg-[#1e2435] text-[#d1d5db] disabled:opacity-50 hover:bg-[#2a3045] transition-colors"
          >Next →</button>
        </div>
      </div>
    {/if}
  {/if}
</div>
