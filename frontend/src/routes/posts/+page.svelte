<script lang="ts">
  import { toast } from "$lib/stores/toast";
  import { onMount, onDestroy } from "svelte";
  import { postsApi, type PostSummary } from "$lib/api/posts";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { realtime } from "$lib/stores/realtime";
  import { timezone } from "$lib/stores/timezone.svelte";
  import { composer } from "$lib/stores/composer.svelte";
  import { confirmModal } from "$lib/stores/modals.svelte";
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

  // Phase 5: search + sort state.
  let searchQuery = $state("");
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let sortBy = $state("scheduled_date");
  let allIntegrations = $state<Integration[]>([]);
  let filterIntegrationIds = $state<string[]>([]);

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
    const r = await postsApi.list({
      limit,
      offset: (currentPage - 1) * limit,
      ...(filter !== "all" && { state: filter }),
      ...(searchQuery.trim() && { q: searchQuery.trim() }),
      ...(filterIntegrationIds.length > 0 && { integration_ids: filterIntegrationIds }),
      sort: sortBy,
    });
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

  // Phase 5: debounced search trigger.
  function onSearchInput() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      searchTimer = null;
      currentPage = 1;
      load();
    }, 350);
  }

  function toggleFilter(f: string) {
    filter = f;
    currentPage = 1;
    load();
  }

  function toggleIntegrationFilter(intId: string) {
    if (filterIntegrationIds.includes(intId)) {
      filterIntegrationIds = filterIntegrationIds.filter(id => id !== intId);
    } else {
      filterIntegrationIds = [...filterIntegrationIds, intId];
    }
    currentPage = 1;
    load();
  }

  function handleSortChange(e: Event) {
    sortBy = (e.currentTarget as HTMLSelectElement).value;
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

      // Phase 2: open the composer modal with prefilled content so the
      // user can review/edit before scheduling. Previously this created
      // the duplicate directly without review.
      composer.openCreate(
        undefined,                                // no preset date
        [post.integration_id],                    // same channel
        post.content,                             // prefilled content
      );
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

  // Phase 5: bulk reschedule with offset.
  // Reschedules all selected posts to a base date, spread by N minutes.
  async function handleBulkReschedule() {
    if (selectedIds.size === 0) return;
    const baseDate = prompt(`Enter base date+time for ${selectedIds.size} posts (YYYY-MM-DD HH:MM):`);
    if (!baseDate) return;
    const spreadMin = parseInt(prompt('Spread posts by how many minutes? (0 = same time)', '30') || '0', 10);
    const baseIso = new Date(baseDate.replace(' ', 'T') + ':00.000Z').toISOString();
    if (isNaN(new Date(baseIso).getTime())) {
      toast('Invalid date format', 'error');
      return;
    }
    bulkActionLoading = true;
    let failures = 0;
    let successes = 0;
    const ids = Array.from(selectedIds);
    for (let i = 0; i < ids.length; i++) {
      const offsetMs = i * spreadMin * 60 * 1000;
      const schedAt = new Date(new Date(baseIso).getTime() + offsetMs).toISOString();
      const r = await postsApi.reschedule(ids[i], schedAt, false);
      if (r.error) failures++;
      else successes++;
    }
    bulkActionLoading = false;
    if (failures === 0) {
      toast(`Rescheduled ${successes} posts`, 'success');
    } else {
      toast(`Rescheduled ${successes}, ${failures} failed`, 'error');
    }
    clearSelection();
    load();
  }

  // Phase 5: bulk duplicate — opens the composer with the first post's
  // content prefilled. (One at a time; bulk duplicate via composer is
  // more useful than blind batch creation.)
  async function handleBulkDuplicate() {
    if (selectedIds.size === 0) return;
    const firstId = Array.from(selectedIds)[0];
    const detail = await postsApi.get(firstId);
    if (detail.data) {
      composer.openCreate(undefined, [detail.data.integration_id], detail.data.content);
    }
  }

  onMount(async () => {
    // Read state filter from URL params (e.g. /posts?state=error)
    const stateParam = $pageStore.url.searchParams.get('state');
    if (stateParam && filters.includes(stateParam)) {
      filter = stateParam;
    }
    // Phase 5: load integrations for the channel filter.
    const integRes = await integrationsApi.list();
    if (integRes.data) allIntegrations = integRes.data.integrations.filter(i => !i.disabled);
    load();
    const events = ['post_created', 'post_scheduled', 'post_published', 'post_failed', 'post_deleted'];
    for (const evt of events) {
      postsUnsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    postsUnsubscribers.forEach(fn => fn());
    if (searchTimer) clearTimeout(searchTimer);
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
      <button onclick={() => composer.openCreate()} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Post</button>
    </div>
  </div>

  <!-- Search + sort + channel filter row (Phase 5) -->
  <div class="flex gap-2 flex-wrap items-center">
    <div class="relative flex-1 min-w-[200px]">
      <input
        type="text"
        bind:value={searchQuery}
        oninput={onSearchInput}
        placeholder="Search posts by content or title..."
        class="w-full px-3 py-2 pl-9 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none"
      />
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-muted">
        <Icon name="search" class="w-4 h-4" />
      </span>
    </div>
    <select
      value={sortBy}
      onchange={handleSortChange}
      class="px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none"
      title="Sort by"
    >
      <option value="scheduled_date">Sort: Scheduled date</option>
      <option value="created_date">Sort: Created date</option>
      <option value="engagement">Sort: Engagement</option>
    </select>
  </div>

  <!-- Channel filter (Phase 5) — only show if integrations exist -->
  {#if allIntegrations.length > 0}
    <div class="flex gap-1 flex-wrap items-center">
      <span class="text-xs text-muted mr-1">Channels:</span>
      <button
        onclick={() => { filterIntegrationIds = []; currentPage = 1; load(); }}
        class="px-2 py-1 text-[10px] rounded-md transition-colors {filterIntegrationIds.length === 0 ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover border border-line'}"
      >All</button>
      {#each allIntegrations as int (int.id)}
        <button
          onclick={() => toggleIntegrationFilter(int.id)}
          class="px-2 py-1 text-[10px] rounded-md transition-colors {filterIntegrationIds.includes(int.id) ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover border border-line'}"
        >{int.provider_name}</button>
      {/each}
    </div>
  {/if}

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
          onclick={handleBulkReschedule}
          disabled={bulkActionLoading}
          class="px-3 py-1.5 text-xs bg-surface-hover hover:bg-line border border-line text-content rounded-lg disabled:opacity-50 transition-colors"
        >
          {bulkActionLoading ? '...' : 'Reschedule'}
        </button>
        <button
          onclick={handleBulkDuplicate}
          disabled={bulkActionLoading}
          class="px-3 py-1.5 text-xs bg-surface-hover hover:bg-line border border-line text-content rounded-lg disabled:opacity-50 transition-colors"
        >
          Duplicate
        </button>
        <button
          onclick={handleBulkDelete}
          disabled={bulkActionLoading}
          class="px-3 py-1.5 text-xs bg-red-600 hover:bg-red-500 text-white rounded-lg disabled:opacity-50 transition-colors"
        >
          {bulkActionLoading ? 'Deleting...' : 'Delete'}
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
              onclick={() => composer.openEdit(post.id)}
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
            onclick={() => composer.openEdit(post.id)}
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
