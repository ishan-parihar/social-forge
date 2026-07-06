<script lang="ts">
  import { toast } from "$lib/stores/toast";
  import { onMount, onDestroy } from "svelte";
  import { postsApi, type PostSummary } from "$lib/api/posts";
  import { realtime } from "$lib/stores/realtime";
  import { timezone } from "$lib/stores/timezone.svelte";
  import Badge from "$lib/ui/Badge.svelte";
  import Icon from "$lib/ui/Icon.svelte";
  import { goto } from "$app/navigation";
  import { page as pageStore } from "$app/stores";

  let posts = $state<PostSummary[]>([]);
  let filter = $state("all");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let currentPage = $state(1);
  let totalPages = $state(1);
  let totalItems = $state(0);
  let groupByCampaign = $state(false);
  const limit = 20;

  // Bulk-selection state (R-15 / U-6): a Set of post IDs the user has
  // checked. Empty by default; cleared on filter change or page change.
  let selectedIds = $state<Set<string>>(new Set());
  let bulkActionLoading = $state(false);
  // Per-post duplication in-flight flag (R-14 / U-5): prevents double-clicks
  // from spawning two duplicates of the same post.
  let duplicatingId = $state<string | null>(null);

  // Derived: are any posts selected? Used to toggle the bulk-action bar.
  let hasSelection = $derived(selectedIds.size > 0);

  function toggleSelect(id: string, e: Event) {
    e.stopPropagation();
    e.preventDefault();
    const next = new Set(selectedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selectedIds = next;
  }

  function toggleSelectAll() {
    if (selectedIds.size === posts.length) {
      selectedIds = new Set();
    } else {
      selectedIds = new Set(posts.map(p => p.id));
    }
  }

  function clearSelection() {
    selectedIds = new Set();
  }

  async function load() {
    loading = true;
    error = null;
    const params: Record<string, string | number> = { limit, offset: (currentPage - 1) * limit };
    if (filter !== "all") params.state = filter;
    const r = await postsApi.list(params);
    if (r.data) {
      posts = r.data.posts;
      totalItems = r.data.total;
      totalPages = Math.ceil(r.data.total / limit);
      // Clear selection whenever the page/filter reloads — the IDs in
      // the selection set may no longer be on the visible page.
      selectedIds = new Set();
    } else {
      toast(`Failed: ${r.error}`, "error");
    }
    loading = false;
  }

  function toggleFilter(f: string) {
    filter = f;
    currentPage = 1;
    load();
  }

  function handlePageChange(p: number) {
    currentPage = p;
    load();
  }

  // Duplicate a single post (R-14 / U-5): fetches full detail, finds the
  // next available slot for that integration, and creates a new draft
  // with all the original's content/media/tags/first_comment preserved.
  async function handleDuplicate(postId: string) {
    if (duplicatingId) return;
    duplicatingId = postId;
    try {
      const detail = await postsApi.get(postId);
      if (detail.error || !detail.data) {
        toast(`Failed to fetch post: ${detail.error || 'unknown'}`, "error");
        return;
      }
      const post = detail.data;

      const slot = await postsApi.findSlot(post.integration_id);
      if (slot.error || !slot.data?.date) {
        toast(`Failed to find slot: ${slot.error || 'unknown'}`, "error");
        return;
      }

      await postsApi.create({
        integration_ids: [post.integration_id],
        content: post.content,
        title: post.title,
        scheduled_at: slot.data.date,
        tag_ids: post.tags?.map(t => t.id) || [],
        first_comment: post.first_comment || undefined,
        media: (post.media || []).map(m => ({
          id: crypto.randomUUID(),
          url: m.url,
          mime_type: m.mime_type,
          alt: m.alt,
        })),
      });
      toast("Post duplicated", "success");
      load();
    } catch (e) {
      toast(`Failed to duplicate: ${e instanceof Error ? e.message : 'unknown'}`, "error");
    } finally {
      duplicatingId = null;
    }
  }

  // Bulk delete (R-15 / U-6): confirm, then delete each selected post.
  // Failures are collected and surfaced as a single toast.
  async function handleBulkDelete() {
    if (selectedIds.size === 0) return;
    if (!confirm(`Delete ${selectedIds.size} post${selectedIds.size > 1 ? 's' : ''}? This cannot be undone.`)) return;
    bulkActionLoading = true;
    let failures = 0;
    const ids = Array.from(selectedIds);
    for (const id of ids) {
      const r = await postsApi.delete(id);
      if (r.error) failures++;
    }
    bulkActionLoading = false;
    if (failures === 0) {
      toast(`Deleted ${ids.length} post${ids.length > 1 ? 's' : ''}`, "success");
    } else {
      toast(`Deleted ${ids.length - failures}, ${failures} failed`, "error");
    }
    clearSelection();
    load();
  }

  // Campaign grouping: cluster posts by group_id
  let campaignGroups = $derived.by(() => {
    if (!groupByCampaign) return null;
    const groups = new Map<string, PostSummary[]>();
    for (const p of posts) {
      const gid = p.group_id || 'single';
      if (!groups.has(gid)) groups.set(gid, []);
      groups.get(gid)!.push(p);
    }
    return Array.from(groups.entries()).sort((a, b) => b[1].length - a[1].length);
  });

  let postsUnsubscribers: (() => void)[] = [];
  const filters = ["all", "draft", "queued", "published", "error"];

  onMount(() => {
    // Read state filter from URL params (e.g. /posts?state=error)
    const stateParam = $pageStore.url.searchParams.get('state');
    if (stateParam && filters.includes(stateParam)) {
      filter = stateParam;
    }
    load();
    const events = ['post_created', 'post_scheduled', 'post_published', 'post_failed', 'post_deleted'];
    for (const evt of events) {
      postsUnsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    postsUnsubscribers.forEach(fn => fn());
  });
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Posts</h2>
    <div class="flex gap-2 items-center">
      <button
        onclick={() => groupByCampaign = !groupByCampaign}
        class="flex items-center gap-1.5 px-3 py-1.5 text-sm border border-line rounded-lg transition-colors {groupByCampaign ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover'}"
      >
        <Icon name="analytics" class="w-3.5 h-3.5" />
        {groupByCampaign ? 'Grouped' : 'Group by Campaign'}
      </button>
      <button onclick={() => goto("/posts/new")} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Post</button>
    </div>
  </div>

  <!-- Filter tabs -->
  <div class="flex gap-1 bg-surface border border-line rounded-lg p-1 overflow-x-auto">
    {#each filters as f}
      <button
        onclick={() => toggleFilter(f)}
        class="px-3 py-1.5 text-xs capitalize rounded-md transition-colors {filter === f ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover'}"
      >{f}</button>
    {/each}
  </div>

  <!-- Bulk action bar (visible when one or more posts are checked) -->
  {#if hasSelection}
    <div class="flex items-center justify-between bg-indigo-500/10 border border-indigo-500/30 rounded-lg px-4 py-2">
      <div class="flex items-center gap-3">
        <span class="text-sm text-indigo-300">{selectedIds.size} selected</span>
        <button onclick={clearSelection} class="text-xs text-muted hover:text-white">Clear</button>
      </div>
      <div class="flex items-center gap-2">
        <button
          onclick={handleBulkDelete}
          disabled={bulkActionLoading}
          class="px-3 py-1.5 text-xs bg-red-600 hover:bg-red-500 text-white rounded-lg disabled:opacity-50 transition-colors"
        >
          {bulkActionLoading ? 'Deleting...' : 'Delete Selected'}
        </button>
      </div>
    </div>
  {/if}

  <!-- Post list -->
  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if posts.length === 0}
    <div class="text-center py-12">
      <p class="text-sm text-muted mb-3">No posts found</p>
      <button onclick={() => goto("/posts/new")} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">
        Create your first post
      </button>
    </div>
  {:else if groupByCampaign && campaignGroups}
    <!-- Campaign grouped view -->
    <div class="space-y-4">
      {#each campaignGroups as [gid, groupPosts] (gid)}
        <div class="bg-surface border border-line rounded-xl overflow-hidden">
          <div class="px-4 py-2.5 bg-surface-hover border-b border-line flex items-center gap-2">
            <Icon name="analytics" class="w-3.5 h-3.5 text-indigo-400" />
            <span class="text-sm font-medium">
              {gid === 'single' ? 'Individual Posts' : `Campaign ${gid.slice(0, 8)}`}
            </span>
            <span class="text-xs text-muted ml-auto">{groupPosts.length} post{groupPosts.length > 1 ? 's' : ''}</span>
          </div>
          {#each groupPosts as post (post.id)}
            <button
              onclick={() => goto(`/posts/${post.id}`)}
              class="w-full flex items-center gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors text-left"
            >
              <div class="flex-1 min-w-0">
                <div class="text-sm truncate">{post.content || '(no content)'}</div>
                <div class="text-xs text-muted mt-0.5">{post.integration_name}</div>
              </div>
              <div class="text-xs text-muted shrink-0">
                {post.scheduled_at ? timezone.formatDate(post.scheduled_at) : ""}
              </div>
              <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
            </button>
          {/each}
        </div>
      {/each}
    </div>
  {:else}
    <!-- Flat list view -->
    <div class="bg-surface border border-line rounded-xl overflow-hidden">
      <!-- Select-all header row -->
      <div class="flex items-center gap-3 px-4 py-2 bg-surface-hover border-b border-line text-xs text-muted">
        <input
          type="checkbox"
          checked={posts.length > 0 && selectedIds.size === posts.length}
          onchange={toggleSelectAll}
          class="rounded"
          aria-label="Select all"
        />
        <span>{selectedIds.size > 0 ? `${selectedIds.size} of ${posts.length} selected` : `${totalItems} total`}</span>
      </div>
      {#each posts as post (post.id)}
        <div class="flex items-center gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors">
          <input
            type="checkbox"
            checked={selectedIds.has(post.id)}
            onclick={(e) => toggleSelect(post.id, e)}
            class="rounded shrink-0"
            aria-label="Select post"
          />
          <button
            onclick={() => goto(`/posts/${post.id}`)}
            class="flex-1 flex items-center gap-4 text-left min-w-0"
          >
            <div class="flex-1 min-w-0">
              <div class="text-sm truncate">{post.content || '(no content)'}</div>
              <div class="text-xs text-muted mt-0.5 flex items-center gap-2">
                {post.integration_name}
                {#if post.group_id}
                  <span class="text-indigo-400">Campaign {post.group_id.slice(0, 8)}</span>
                {/if}
              </div>
            </div>
            <div class="text-xs text-muted shrink-0">
              {post.scheduled_at ? timezone.formatDate(post.scheduled_at) : ""}
            </div>
            <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
            {#if post.error_message}
              <span class="text-xs text-red-400" title={post.error_message}>!</span>
            {/if}
          </button>
          <!-- Per-row duplicate button (R-14 / U-5) -->
          <button
            onclick={() => handleDuplicate(post.id)}
            disabled={duplicatingId === post.id}
            class="text-xs text-muted hover:text-indigo-400 disabled:opacity-50 transition-colors px-2 py-1 rounded"
            title="Duplicate post"
            aria-label="Duplicate post"
          >
            {#if duplicatingId === post.id}
              <Icon name="calendar" class="w-3.5 h-3.5 animate-pulse" />
            {:else}
              <Icon name="post" class="w-3.5 h-3.5" />
            {/if}
          </button>
        </div>
      {/each}
    </div>

    {#if totalPages > 1}
      <div class="flex items-center justify-between px-4 py-3 bg-surface border border-line rounded-xl">
        <span class="text-sm text-muted">
          Showing {(currentPage - 1) * limit + 1}–{Math.min(currentPage * limit, totalItems)} of {totalItems}
        </span>
        <div class="flex gap-2">
          <button
            onclick={() => handlePageChange(currentPage - 1)}
            disabled={currentPage <= 1}
            class="px-3 py-1 text-sm rounded bg-surface-hover text-content-secondary disabled:opacity-50 hover:bg-line transition-colors"
          >Previous</button>
          <button
            onclick={() => handlePageChange(currentPage + 1)}
            disabled={currentPage >= totalPages}
            class="px-3 py-1 text-sm rounded bg-surface-hover text-content-secondary disabled:opacity-50 hover:bg-line transition-colors"
          >Next</button>
        </div>
      </div>
    {/if}
  {/if}
</div>
