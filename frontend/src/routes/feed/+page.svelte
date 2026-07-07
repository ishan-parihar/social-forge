<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { feedApi, proxyMediaUrl, type FeedPost, type FeedAccount } from "$lib/api/feed";
  import { integrationsApi } from "$lib/api/integrations";
  import EngagementCard from "$lib/components/EngagementCard.svelte";
  import CommentsThread from "$lib/components/CommentsThread.svelte";
  import MediaCarousel from "$lib/media/MediaCarousel.svelte";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";
  import { composer } from "$lib/stores/composer.svelte";
  import { modals } from "$lib/stores/modals.svelte";
  import { providerMeta as centralProviderMeta, platformPostUrl } from "$lib/providers";

  let posts = $state<FeedPost[]>([]);
  let loading = $state(false);
  let loadingMore = $state(false);
  let importing = $state(false);
  let fetchError = $state<string | null>(null);
  let nextCursor = $state<string | null>(null);
  let hasMore = $state(false);
  let initialLoad = $state(true);
  let attemptedImport = $state(false);
  let commentsOpenFor = $state<string | null>(null);

  // Phase v21: Repurpose + Edit modal state.
  // The Repurpose modal asks the user to pick a target integration (channel)
  // before calling POST /api/feed/{id}/repurpose. The Edit modal is a simple
  // textarea + JSON-editor for fixing import errors on the cached copy.
  let repurposeModalOpen = $state(false);
  let repurposePost = $state<FeedPost | null>(null);
  let repurposeTargetIntegration = $state<string>("");
  let repurposeSubmitting = $state(false);
  let editModalOpen = $state(false);
  let editPost = $state<FeedPost | null>(null);
  let editText = $state("");
  let editSubmitting = $state(false);

  // We need integrations list for the Repurpose target picker.
  let allIntegrations = $state<Array<{ id: string; provider_name: string; provider_identifier: string; disabled: boolean }>>([]);

  // Filter state — accounts (channels) grouped by provider
  let showFilter = $state(false);
  let allAccounts = $state<FeedAccount[]>([]);
  let selectedAccountHandle = $state<string | null>(null);
  let selectedProvider = $state<string | null>(null);
  let filterRef = $state<HTMLDivElement | null>(null);

  // Sentinel for detecting bottom
  let sentinel = $state<HTMLDivElement | null>(null);
  let nearBottom = $state(false);

  // Derived: grouped accounts for the filter dropdown
  let providerGroups = $derived.by(() => {
    const groups = new Map<string, FeedAccount[]>();
    for (const a of allAccounts) {
      const list = groups.get(a.provider) ?? [];
      list.push(a);
      groups.set(a.provider, list);
    }
    return groups;
  });

  // Derived: filtered posts — server-side filtered when author_handle is active, else client-side
  let filteredPosts = $derived.by(() => {
    if (selectedAccountHandle) {
      // Server-side filtered by author_handle — show all returned posts
      return posts;
    }
    if (selectedProvider) {
      return posts.filter(p => p.provider === selectedProvider);
    }
    return posts;
  });

  let activeFilterLabel = $derived.by(() => {
    if (selectedAccountHandle) {
      const acct = allAccounts.find(a => a.author_handle === selectedAccountHandle);
      return acct?.author_name || acct?.author_handle || selectedAccountHandle;
    }
    if (selectedProvider) {
      return providerMeta(selectedProvider).label;
    }
    return null;
  });

  async function load() {
    loading = true;
    fetchError = null;
    const r = await feedApi.list(undefined, selectedProvider ?? undefined, selectedAccountHandle ?? undefined);
    if (r.data) {
      posts = r.data.posts;
      nextCursor = r.data.next_cursor;
      hasMore = r.data.has_more;
    } else {
      fetchError = r.error || "Failed to load feed";
    }
    loading = false;
    initialLoad = false;
  }

  async function loadAccounts() {
    const r = await feedApi.accounts();
    if (r.data) {
      allAccounts = r.data;
    }
  }

  async function triggerImport() {
    importing = true;
    try {
      await feedApi.import();
    } catch {
      // API call failed silently — load() will show any feed errors
    }
    importing = false;
    attemptedImport = true;
    await load();
  }

  async function loadMore() {
    if (loadingMore || !nextCursor || !hasMore) return;
    loadingMore = true;
    const r = await feedApi.list(nextCursor, selectedProvider ?? undefined, selectedAccountHandle ?? undefined);
    if (r.data) {
      posts = [...posts, ...r.data.posts];
      nextCursor = r.data.next_cursor;
      hasMore = r.data.has_more;
    }
    loadingMore = false;
  }

  function selectAccount(handle: string | null, provider?: string) {
    selectedAccountHandle = handle;
    selectedProvider = handle ? null : (provider ?? null);
    showFilter = false;
    nextCursor = null;
    hasMore = false;
    posts = [];
    load();
  }

  function clearFilter() {
    selectedAccountHandle = null;
    selectedProvider = null;
    showFilter = false;
    nextCursor = null;
    hasMore = false;
    posts = [];
    load();
  }

  function formatTime(iso: string): string {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    if (diffDay < 7) return `${diffDay}d ago`;
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
  }

  function isVideo(mimeType: string | undefined, url: string): boolean {
    if (mimeType && mimeType.startsWith('video/')) return true;
    if (url.match(/\.(mp4|webm|mov|avi|mkv)(\?|$)/i)) return true;
    return false;
  }

  function isEmbed(mimeType: string | undefined): boolean {
    return mimeType === 'text/html';
  }

  function embedDomain(url: string): string {
    try {
      const u = new URL(url);
      if (u.hostname.includes('youtube.com') || u.hostname.includes('youtu.be')) return 'YouTube';
      if (u.hostname.includes('tiktok.com')) return 'TikTok';
      return 'External';
    } catch {
      return 'External';
    }
  }

  function posterUrl(post: FeedPost): string | null {
    if (post.metadata && typeof post.metadata === 'object') {
      const m = post.metadata as Record<string, unknown>;
      if (typeof m.poster_url === 'string') return m.poster_url;
    }
    return null;
  }

  // Provider metadata now comes from the central $lib/providers module
  // (R-8) — no more 18-entry local map. The local providerMeta() wrapper
  // adapts the central shape ({label, color, icon, charLimit}) to the
  // {label, color, dot} shape this file's template expects (dot == color).
  function providerMeta(provider: string): { label: string; color: string; dot: string } {
    const m = centralProviderMeta(provider);
    return { label: m.label, color: m.color, dot: m.color };
  }

  // Click outside to close filter
  $effect(() => {
    if (!showFilter || !filterRef) return;
    const handler = (e: MouseEvent) => {
      if (filterRef && !filterRef.contains(e.target as Node)) {
        showFilter = false;
      }
    };
    requestAnimationFrame(() => document.addEventListener('click', handler));
    return () => document.removeEventListener('click', handler);
  });

  // IntersectionObserver to detect when user scrolls near bottom
  $effect(() => {
    if (!sentinel) return;
    const obs = new IntersectionObserver(
      (entries) => {
        nearBottom = entries[0].isIntersecting;
      },
      { threshold: 0.1, rootMargin: '200px' }
    );
    obs.observe(sentinel);
    return () => obs.disconnect();
  });

  let feedUnsubscribers: (() => void)[] = [];

  // ── Phase v21: Repurpose + Edit modal functions ──────────────────────

  async function openRepurposeModal(post: FeedPost) {
    repurposePost = post;
    repurposeTargetIntegration = "";
    repurposeSubmitting = false;
    // Load integrations if not already loaded
    if (allIntegrations.length === 0) {
      const r = await integrationsApi.list();
      if (r.data) {
        allIntegrations = r.data.integrations.filter(i => !i.disabled);
      }
    }
    if (allIntegrations.length === 0) {
      toast("Connect a channel first to repurpose posts", "error");
      return;
    }
    // Pre-select the first integration if there's only one
    if (allIntegrations.length === 1) {
      repurposeTargetIntegration = allIntegrations[0].id;
    }
    repurposeModalOpen = true;
  }

  async function submitRepurpose() {
    if (!repurposePost || !repurposeTargetIntegration) return;
    repurposeSubmitting = true;
    try {
      const r = await feedApi.repurpose(repurposePost.id, {
        integration_id: repurposeTargetIntegration,
      });
      if (r.error) {
        toast(`Repurpose failed: ${r.error}`, "error");
        return;
      }
      toast("Post created as draft — open the composer to schedule it", "success");
      repurposeModalOpen = false;
      repurposePost = null;
      // Optionally open the composer to edit the new draft
      if (r.data?.post?.id) {
        composer.openEdit(r.data.post.id);
      }
    } finally {
      repurposeSubmitting = false;
    }
  }

  function openEditModal(post: FeedPost) {
    editPost = post;
    editText = post.text;
    editSubmitting = false;
    editModalOpen = true;
  }

  async function submitEdit() {
    if (!editPost) return;
    if (!editText.trim()) {
      toast("Text cannot be empty", "error");
      return;
    }
    editSubmitting = true;
    try {
      const r = await feedApi.update(editPost.id, { text: editText });
      if (r.error) {
        toast(`Edit failed: ${r.error}`, "error");
        return;
      }
      // Update the local post in-place so the UI reflects the change immediately
      const idx = posts.findIndex(p => p.id === editPost!.id);
      if (idx >= 0) {
        posts[idx] = { ...posts[idx], text: editText };
        posts = posts; // trigger Svelte 5 reactivity
      }
      toast("Post updated", "success");
      editModalOpen = false;
      editPost = null;
    } finally {
      editSubmitting = false;
    }
  }

  async function hidePost(post: FeedPost) {
    // Phase v21: replace inline anonymous handler with named function +
    // use modals.areYouSure for confirmation (consistent with calendar/posts).
    const ok = await modals.areYouSure({
      title: 'Hide this post from feed?',
      message: 'The post will be hidden from your feed but remains on the platform. You can re-import it later by clicking Refresh.',
      confirmLabel: 'Hide',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    const r = await feedApi.delete(post.id);
    if (r.error) {
      toast("Failed to hide: " + r.error, "error");
    } else {
      posts = posts.filter(p => p.id !== post.id);
      toast("Post hidden from feed", "success");
    }
  }

  onMount(async () => {
    await loadAccounts();
    await load();
    // If feed is empty, auto-trigger an import from connected providers
    if (posts.length === 0) {
      importing = true;
      try {
        await feedApi.import();
      } catch {
        // API call failed silently — load() will show any feed errors
      }
      importing = false;
      attemptedImport = true;
      await load();
    }
    // Auto-refresh on realtime events
    const events = ['post_published', 'post_created', 'lagged'];
    for (const evt of events) {
      feedUnsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    feedUnsubscribers.forEach(fn => fn());
  });
</script>

<div class="page-enter page-enter max-w-3xl mx-auto space-y-5">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-3">
      <h2 class="text-xl font-bold text-content tracking-tight">Feed</h2>
      {#if !loading && posts.length > 0}
        <span class="text-xs text-muted font-mono bg-surface-hover px-2 py-0.5 rounded-full border border-line">
          {filteredPosts.length}{#if filteredPosts.length !== posts.length} / {posts.length}{/if}
        </span>
      {/if}
    </div>

    <div class="flex items-center gap-2">
      <!-- Refresh button -->
      <button
        onclick={triggerImport}
        disabled={importing}
        class="flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-lg transition-all duration-200
          bg-surface-hover text-muted border border-line
          hover:border-line hover:text-content
          disabled:opacity-50 disabled:cursor-not-allowed"
      >
        <svg
          class="w-3.5 h-3.5 transition-transform duration-500 {importing ? 'animate-spin' : ''}"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        >
          <path d="M2 8a6 6 0 0111.3-3M14 8a6 6 0 01-11.3 3" stroke-linecap="round" />
          <path d="M13 2v3h-3M3 14v-3h3" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        {importing ? 'Refreshing…' : 'Refresh'}
      </button>

      <!-- Active filter chip -->
      {#if activeFilterLabel}
        {@const chipMeta = selectedProvider ? providerMeta(selectedProvider) : null}
        <span class="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium rounded-lg
          bg-brand-500/15 text-brand-300 border border-brand-500/25">
          {#if chipMeta}
            <span class="w-1.5 h-1.5 rounded-full" style="background: {chipMeta.dot}" />
          {/if}
          {activeFilterLabel}
          <button onclick={clearFilter} class="ml-0.5 hover:text-brand-200 transition-colors">
            <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M3 3l6 6M9 3l-6 6" stroke-linecap="round" />
            </svg>
          </button>
        </span>
      {/if}

      <!-- Filter button -->
      {#if allAccounts.length > 0}
        <div bind:this={filterRef} class="relative">
          <button
            onclick={() => showFilter = !showFilter}
            class="flex items-center gap-2 px-3 py-1.5 text-xs font-medium rounded-lg transition-all duration-200
              {showFilter || activeFilterLabel
                ? 'bg-brand-500/20 text-brand-300 border border-brand-500/30 shadow-[0_0_12px_rgb(var(--brand-rgb)/0.1)]'
                : 'bg-surface-hover text-muted border border-line hover:border-line hover:text-content'}"
          >
            <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M2 4h12M4 8h8M6 12h4" stroke-linecap="round" />
            </svg>
            {activeFilterLabel ? activeFilterLabel : 'Channels'}
          </button>

          <!-- Filter dropdown -->
          {#if showFilter}
            <div class="absolute right-0 top-full mt-2 w-72 z-50
              bg-surface border border-line rounded-xl shadow-2xl shadow-black/40
              overflow-hidden motion-safe:animate-in duration-200">
              <!-- Header -->
              <div class="px-4 py-3 border-b border-line">
                <p class="text-xs font-semibold text-content">Filter by channel</p>
                <p class="text-[10px] text-muted mt-0.5">
                  {allAccounts.length} account{allAccounts.length !== 1 ? 's' : ''} connected
                </p>
              </div>

              <!-- "All channels" option -->
              <button
                onclick={() => clearFilter()}
                class="w-full flex items-center gap-3 px-4 py-2.5 text-left transition-colors duration-150 border-b border-line
                  {!selectedAccountHandle && !selectedProvider
                    ? 'bg-brand-500/10 text-content'
                    : 'text-muted hover:text-muted hover:bg-background-input'}"
              >
                <svg class="w-4 h-4 flex-shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M2 4h12M4 8h8M6 12h4" stroke-linecap="round" />
                </svg>
                <span class="text-sm font-medium">All channels</span>
                {#if !selectedAccountHandle && !selectedProvider}
                  <span class="ml-auto w-1.5 h-1.5 rounded-full bg-brand-400" />
                {/if}
              </button>

              <!-- Accounts grouped by provider -->
              <div class="max-h-80 overflow-y-auto py-1">
                {#each [...providerGroups.entries()] as [provider, accounts]}
                  {@const pmeta = providerMeta(provider)}
                  <!-- Provider header -->
                  <div class="px-4 py-1.5 flex items-center gap-2">
                    <span class="w-1.5 h-1.5 rounded-full flex-shrink-0" style="background: {pmeta.dot}" />
                    <span class="text-[10px] font-semibold uppercase tracking-wider text-muted">{pmeta.label}</span>
                  </div>

                  <!-- Accounts in this provider -->
                  {#each accounts as acct}
                    <button
                      onclick={() => selectAccount(acct.author_handle, acct.provider)}
                      class="w-full flex items-center gap-3 px-4 py-2 text-left transition-colors duration-150
                        {selectedAccountHandle === acct.author_handle
                          ? 'bg-brand-500/10 text-content'
                          : 'text-muted hover:text-content hover:bg-background-input'}"
                    >
                      {#if acct.author_avatar}
                        <img
                          src={acct.author_avatar}
                          alt=""
                          class="w-6 h-6 rounded-full flex-shrink-0 object-cover ring-1 ring-line"
                        />
                      {:else}
                        <span class="w-6 h-6 rounded-full flex-shrink-0 bg-line flex items-center justify-center">
                          <svg class="w-3 h-3 text-muted" viewBox="0 0 16 16" fill="currentColor">
                            <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6a5 5 0 0110 0H3z"/>
                          </svg>
                        </span>
                      {/if}
                      <div class="flex-1 min-w-0">
                        <div class="text-sm font-medium truncate">{acct.author_name || acct.author_handle || 'Unknown'}</div>
                        {#if acct.author_handle}
                          <div class="text-[10px] text-muted truncate">@{acct.author_handle}</div>
                        {/if}
                      </div>
                      {#if selectedAccountHandle === acct.author_handle}
                        <span class="ml-auto w-1.5 h-1.5 rounded-full bg-brand-400 flex-shrink-0" />
                      {/if}
                    </button>
                  {/each}
                {/each}
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>

  <!-- Error state -->
  {#if fetchError}
    <div class="text-center py-12">
      <div class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-error/15 border border-error/30 text-sm text-error">
        <svg class="w-4 h-4 flex-shrink-0" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1a7 7 0 110 14A7 7 0 018 1zm0 9.5a.75.75 0 100 1.5.75.75 0 000-1.5zM8.75 4.5a.75.75 0 00-1.5 0v4a.75.75 0 001.5 0v-4z"/>
        </svg>
        {fetchError}
      </div>
      <button
        onclick={load}
        class="mt-4 px-4 py-2 text-xs font-medium rounded-lg bg-surface-hover text-muted hover:text-content hover:bg-line border border-line transition-colors">
        Try again
      </button>
    </div>

  <!-- Loading skeleton -->
  {:else if loading}
    <div class="page-enter space-y-3    motion-safe:animate-in duration-300">
      {#each Array(5) as _, i (i)}
        <div class="bg-surface rounded-xl p-5 border border-line space-y-3">
          <div class="flex items-center gap-2.5">
            <div class="w-6 h-6 rounded-full bg-line" />
            <div class="h-3 bg-line rounded w-20" />
            <div class="h-2.5 bg-line rounded w-16" />
            <div class="ml-auto h-2.5 bg-line rounded w-12" />
          </div>
          <div class="page-enter space-y-2">
            <div class="h-3 bg-line rounded w-full" />
            <div class="h-3 bg-line rounded w-5/6" />
            <div class="h-3 bg-line rounded w-2/3" />
          </div>
        </div>
      {/each}
    </div>

  <!-- Empty state: importing for the first time -->
  {:else if importing}
    <div class="text-center py-20">
      <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-surface border border-line flex items-center justify-center">
        <svg class="w-7 h-7 text-brand-400 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round" />
        </svg>
      </div>
      <p class="text-sm font-medium text-content">Importing your posts…</p>
      <p class="text-xs text-muted mt-1">Fetching recent posts from all your connected providers</p>
      <div class="mt-6 w-48 h-1 mx-auto bg-line rounded-full overflow-hidden">
        <div class="h-full bg-brand-500/50 rounded-full animate-pulse" style="width: 60%" />
      </div>
    </div>

  <!-- Empty state (already attempted) -->
  {:else if !initialLoad && posts.length === 0 && attemptedImport}
    <div class="text-center py-20">
      <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-surface border border-line flex items-center justify-center">
        <svg class="w-7 h-7 text-muted" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <p class="text-sm font-medium text-muted mb-1">No posts found</p>
      <p class="text-xs text-muted mb-6">Connect a social media account to see your feed here</p>
      <button
        onclick={triggerImport}
        class="inline-flex items-center gap-2 px-4 py-2 text-xs font-medium rounded-lg
          bg-surface-hover text-muted hover:text-content hover:bg-line
          border border-line transition-colors"
      >
        <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M2 8a6 6 0 0111.3-3M14 8a6 6 0 01-11.3 3" stroke-linecap="round" />
          <path d="M13 2v3h-3M3 14v-3h3" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        Try again
      </button>
    </div>

  <!-- Filtered-empty state -->
  {:else if filteredPosts.length === 0 && posts.length > 0}
    <div class="text-center py-20">
      <div class="w-14 h-14 mx-auto mb-4 rounded-2xl bg-surface border border-line flex items-center justify-center">
        <svg class="w-6 h-6 text-muted" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M2 4h12M4 8h8M6 12h4" stroke-linecap="round" />
        </svg>
      </div>
      <p class="text-sm font-medium text-muted">No posts match your filters</p>
      <button onclick={clearFilter} class="mt-3 text-xs text-brand-400 hover:text-brand-300 transition-colors">
        Clear filters
      </button>
    </div>

  <!-- Post list -->
  {:else}
    <div class="page-enter space-y-2.5">
      {#each filteredPosts as post (post.id)}
        {@const meta = providerMeta(post.provider)}
        <article
          class="group relative bg-surface rounded-xl border border-line transition-all duration-200
            hover:border-line-hover hover:bg-surface-hover"
        >
          <!-- Subtle top accent line -->
          <div class="absolute top-0 left-6 right-6 h-px opacity-0 group-hover:opacity-100 transition-opacity duration-300"
            style="background: linear-gradient(90deg, transparent, {meta.color}22, transparent)" />

          <div class="p-5">
            <!-- Header: provider badge + author + time -->
            <div class="flex items-center gap-2.5 mb-3">
              <!-- Provider dot -->
              <span class="w-2 h-2 rounded-full flex-shrink-0" style="background: {meta.dot}" />

              <!-- Provider badge -->
              <span class="text-[11px] font-semibold uppercase tracking-wider"
                style="color: {meta.color}">
                {meta.label}
              </span>

              <!-- Avatar -->
              {#if post.author_avatar}
                <img
                  src={post.author_avatar}
                  alt=""
                  class="w-5 h-5 rounded-full flex-shrink-0 object-cover ring-1 ring-line"
                  loading="lazy"
                />
              {/if}

              <!-- Author -->
              {#if post.author_name}
                <span class="text-sm font-medium text-content">{post.author_name}</span>
              {/if}
              {#if post.author_handle}
                <span class="text-xs text-muted">@{post.author_handle}</span>
              {/if}

              <!-- Time -->
              <span class="ml-auto text-[11px] text-muted font-mono">{formatTime(post.created_at)}</span>
            </div>

            <!-- Content -->
            <p class="text-sm text-content-secondary leading-relaxed whitespace-pre-wrap break-words
              selection:bg-brand-500/20 selection:text-brand-200">
              {post.text}
            </p>

            <!-- Media attachments -->
            {#if post.media && post.media.length > 0}
              {#if post.media.length === 1}
                {@const item = post.media[0]}
                <div class="mt-3">
                  <div class="rounded-xl overflow-hidden bg-background-input ring-1 ring-line group/media">
                    {#if isEmbed(item.mime_type)}
                      <!-- Embedded content (YouTube, TikTok, etc.) -->
                      <div class="relative w-full" style="aspect-ratio: 16 / 9; max-height: 65vh;">
                        <div
                          class="absolute inset-0 bg-cover bg-center bg-no-repeat transition-opacity duration-500"
                          style="background-image: {posterUrl(post) ? `url(${posterUrl(post)})` : 'none'}"
                        ></div>
                        <iframe
                          src={item.url}
                          title={item.alt ?? 'Embedded video'}
                          class="w-full h-full bg-transparent relative z-10"
                          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
                          allowfullscreen
                          loading="lazy"
                          referrerpolicy="strict-origin-when-cross-origin"
                        ></iframe>
                        <!-- Fallback link for non-JS or blocked iframes -->
                        <a
                          href={post.url || item.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          class="absolute inset-0 flex items-center justify-center opacity-0 hover:opacity-100 bg-black/40 transition-opacity z-20"
                        >
                          <span class="text-xs font-medium text-white bg-black/60 px-3 py-1.5 rounded-lg">
                            Open on {embedDomain(item.url)}
                          </span>
                        </a>
                      </div>
                    {:else if isVideo(item.mime_type, item.url)}
                      <div class="relative group/video">
                        <video
                          src={proxyMediaUrl(item.url)}
                          controls
                          preload="metadata"
                          playsinline
                          class="w-full max-h-[65vh] bg-black"
                          poster={item.poster_url ? proxyMediaUrl(item.poster_url) : ''}
                        >
                          <a href={item.url} target="_blank" rel="noopener noreferrer"
                            class="text-xs text-brand-400 hover:text-brand-300 underline p-2 block">
                            Download video
                          </a>
                        </video>
                        <!-- Play button overlay when no poster -->
                        {#if !item.poster_url}
                          <div class="absolute inset-0 flex items-center justify-center pointer-events-none
                            bg-black/30 group-hover/video:bg-black/10 transition-colors duration-200">
                            <div class="w-16 h-16 rounded-full bg-white/90 flex items-center justify-center
                              shadow-lg shadow-black/30 group-hover/video:scale-110 transition-transform duration-200">
                              <svg class="w-7 h-7 text-gray-900 ml-1" viewBox="0 0 24 24" fill="currentColor">
                                <path d="M8 5v14l11-7z"/>
                              </svg>
                            </div>
                          </div>
                        {/if}
                      </div>
                    {:else}
                      <a
                        href={item.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        class="block"
                      >
                        <img
                          src={proxyMediaUrl(item.url)}
                          alt={item.alt ?? ''}
                          class="w-full h-auto object-contain max-h-[65vh] transition-transform duration-300 hover:scale-[1.02]"
                          loading="lazy"
                        />
                      </a>
                    {/if}
                  </div>
                </div>
              {:else}
                <div class="mt-3">
                  <MediaCarousel items={post.media} />
                </div>
              {/if}
            {/if}

            <!-- Engagement metrics -->
            {#if post.engagement}
              <EngagementCard engagement={post.engagement} provider={post.provider} />
            {/if}

            <!-- Comments toggle & thread -->
            <div class="mt-2">
              <button
                onclick={() => commentsOpenFor = commentsOpenFor === post.id ? null : post.id}
                class="inline-flex items-center gap-1.5 text-xs font-medium transition-colors duration-150 px-2.5 py-1 rounded-lg
                  {commentsOpenFor === post.id
                    ? 'bg-brand-500/15 text-brand-300 border border-brand-500/25'
                    : 'text-muted hover:text-muted hover:bg-background-input border border-transparent'}"
              >
                <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M14 8a6 6 0 01-9.3 5L2 14l1-2.7A6 6 0 1114 8z" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                {commentsOpenFor === post.id ? 'Hide comments' : 'Comments'}
              </button>

              {#if commentsOpenFor === post.id}
                <CommentsThread {post} onclose={() => commentsOpenFor = null} />
              {/if}
            </div>

            <!-- Footer: link + repurpose + hide -->
            <div class="mt-3 pt-3 border-t border-line flex items-center justify-between">
              <div class="flex items-center gap-4">
                {#if post.url}
                  <a
                    href={post.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center gap-1.5 text-xs font-medium transition-colors duration-150"
                    style="color: {meta.color}"
                  >
                    View original
                    <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                      <path d="M6 3l5 5-5 5" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                  </a>
                {/if}
                <!-- v23-6: "Manage on platform" link — lets the user
                     edit/delete the post on the platform's own UI.
                     Constructed from provider + platform_post_id. -->
                {#if platformPostUrl(post.provider, post.platform_post_id) && platformPostUrl(post.provider, post.platform_post_id) !== post.url}
                  <a
                    href={platformPostUrl(post.provider, post.platform_post_id)!}
                    target="_blank"
                    rel="noopener noreferrer"
                    class="inline-flex items-center gap-1.5 text-xs font-medium text-muted hover:text-content transition-colors duration-150"
                    title="Open this post on {meta.label} to edit or delete it"
                  >
                    Manage on {meta.label}
                    <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                      <path d="M10 2h4v4M14 2l-7 7M8 4H4a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h8a1 1 0 0 0 1-1V9" stroke-linecap="round" stroke-linejoin="round" />
                    </svg>
                  </a>
                {/if}
              </div>
              <div class="flex items-center gap-3">
                <!-- Phase v21: Repurpose — now calls the real backend
                     endpoint POST /api/feed/{id}/repurpose which creates
                     a Social Forge post row with source_external_post_id
                     set for provenance. Previously this was a frontend-only
                     no-op that just opened the composer with prefilled text
                     and made zero backend calls. -->
                <button
                  onclick={() => openRepurposeModal(post)}
                  class="text-xs text-brand-400 hover:text-brand-300 transition-colors"
                  title="Create a new Social Forge post from this content"
                >
                  ✏️ Repurpose
                </button>
                <!-- Phase v21: Edit — calls PUT /api/feed/{id} to update the
                     cached text/media/metadata. Useful for fixing import
                     errors without re-importing. -->
                <button
                  onclick={() => openEditModal(post)}
                  class="text-xs text-muted hover:text-emerald-400 transition-colors"
                  title="Edit the cached copy of this post"
                >
                  Edit
                </button>
                <!-- Phase 3: Save/bookmark -->
                <button
                  onclick={async () => {
                    const r = await feedApi.save(post.id);
                    if (r.error) {
                      toast("Failed to save: " + r.error, "error");
                    } else {
                      toast("Post saved", "success");
                    }
                  }}
                  class="text-xs text-muted hover:text-warning transition-colors"
                  title="Save for later"
                >
                  🔖 Save
                </button>
                <button
                  onclick={() => hidePost(post)}
                  class="text-xs text-muted hover:text-error transition-colors"
                  title="Hide from feed (does not delete on platform)"
                >
                  Hide
                </button>
              </div>
            </div>
          </div>
        </article>
      {/each}
    </div>

    <!-- Bottom section: Load More or end -->
    <div bind:this={sentinel} class="py-6">
      {#if hasMore}
        {#if loadingMore}
          <div class="flex items-center justify-center gap-2.5 py-4">
            <div class="w-5 h-5 rounded-full border-2 border-brand-400/30 border-t-brand-400 animate-spin" />
            <span class="text-xs text-muted font-mono">Loading more…</span>
          </div>
        {:else if nearBottom}
          <div class="flex justify-center            motion-safe:animate-in duration-300">
            <button
              onclick={loadMore}
              class="group flex items-center gap-2.5 px-6 py-3 text-sm font-medium rounded-xl
                bg-surface-hover border border-line-hover text-content-secondary
                hover:bg-line-hover hover:border-brand-500/30 hover:text-content
                transition-all duration-200 shadow-sm hover:shadow-[0_0_20px_rgb(var(--brand-rgb)/0.08)]"
            >
              <svg class="w-4 h-4 transition-transform group-hover:rotate-180 duration-300" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M8 3v10M4 9l4 4 4-4" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
              Load more posts
              {#if nextCursor}
                <span class="text-[10px] text-muted font-mono">· next page</span>
              {/if}
            </button>
          </div>
        {:else}
          <div class="flex justify-center">
            <span class="text-[10px] text-muted-dark font-mono tracking-wider uppercase">Scroll down to load more</span>
          </div>
        {/if}
      {:else if posts.length > 0}
        <div class="flex flex-col items-center gap-2 py-8">
          <div class="w-12 h-px bg-gradient-to-r from-transparent via-line to-transparent" />
          <span class="text-[10px] text-muted-dark font-mono tracking-wider">You're all caught up</span>
          <div class="w-12 h-px bg-gradient-to-r from-transparent via-line to-transparent" />
        </div>
      {/if}
    </div>
  {/if}
</div>

<!-- Phase v21: Repurpose modal — pick a target channel + call the backend -->
{#if repurposeModalOpen && repurposePost}
  <div
    class="fixed inset-0 z-[200] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
    onclick={() => !repurposeSubmitting && (repurposeModalOpen = false)}
    role="dialog"
    aria-modal="true"
    aria-labelledby="repurpose-title"
  >
    <div
      class="bg-surface border border-line rounded-xl shadow-2xl w-full max-w-md p-5"
      onclick={(e) => e.stopPropagation()}
    >
      <h3 id="repurpose-title" class="text-lg font-semibold mb-1">Repurpose post</h3>
      <p class="text-xs text-muted mb-4">
        Create a new Social Forge draft from this imported post. You can edit and schedule it after.
      </p>
      <div class="bg-background-input border border-line rounded-lg p-3 mb-4 max-h-32 overflow-y-auto">
        <p class="text-xs text-content-secondary whitespace-pre-wrap line-clamp-4">{repurposePost.text}</p>
      </div>
      <label class="text-sm text-muted block mb-1.5">Post to channel</label>
      <select
        bind:value={repurposeTargetIntegration}
        disabled={repurposeSubmitting}
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-brand-500 outline-none mb-4"
      >
        <option value="">Select a channel…</option>
        {#each allIntegrations as int (int.id)}
          <option value={int.id}>{int.provider_name}</option>
        {/each}
      </select>
      <div class="flex items-center justify-end gap-2">
        <button
          onclick={() => (repurposeModalOpen = false)}
          disabled={repurposeSubmitting}
          class="px-3 py-1.5 text-sm text-muted hover:text-content border border-line rounded-lg disabled:opacity-50 transition-colors"
        >Cancel</button>
        <button
          onclick={submitRepurpose}
          disabled={repurposeSubmitting || !repurposeTargetIntegration}
          class="px-3 py-1.5 text-sm bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-lg transition-colors flex items-center gap-2"
        >
          {#if repurposeSubmitting}
            <div class="w-3 h-3 rounded-full border-2 border-white/30 border-t-white animate-spin"></div>
            Creating…
          {:else}
            Create draft
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Phase v21: Edit modal — update the cached feed post's text -->
{#if editModalOpen && editPost}
  <div
    class="fixed inset-0 z-[200] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
    onclick={() => !editSubmitting && (editModalOpen = false)}
    role="dialog"
    aria-modal="true"
    aria-labelledby="edit-title"
  >
    <div
      class="bg-surface border border-line rounded-xl shadow-2xl w-full max-w-lg p-5"
      onclick={(e) => e.stopPropagation()}
    >
      <h3 id="edit-title" class="text-lg font-semibold mb-1">Edit cached post</h3>
      <p class="text-xs text-muted mb-4">
        Update the cached copy of this imported post. This does NOT change the original on the platform — only what you see in this feed.
      </p>
      <label class="text-sm text-muted block mb-1.5">Text</label>
      <textarea
        bind:value={editText}
        disabled={editSubmitting}
        rows="6"
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-brand-500 outline-none mb-4 resize-y"
      ></textarea>
      <div class="flex items-center justify-end gap-2">
        <button
          onclick={() => (editModalOpen = false)}
          disabled={editSubmitting}
          class="px-3 py-1.5 text-sm text-muted hover:text-content border border-line rounded-lg disabled:opacity-50 transition-colors"
        >Cancel</button>
        <button
          onclick={submitEdit}
          disabled={editSubmitting || !editText.trim()}
          class="px-3 py-1.5 text-sm bg-emerald-600 hover:bg-emerald-500 disabled:opacity-50 text-white rounded-lg transition-colors flex items-center gap-2"
        >
          {#if editSubmitting}
            <div class="w-3 h-3 rounded-full border-2 border-white/30 border-t-white animate-spin"></div>
            Saving…
          {:else}
            Save changes
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  @keyframes fade-in {
    from { opacity: 0; transform: translateY(4px); }
    to { opacity: 1; transform: translateY(0); }
  }
  :global(.animate-in) {
    animation: fade-in 0.3s ease-out both;
  }
  @media (prefers-reduced-motion: reduce) {
    :global(.animate-in) {
      animation: none;
    }
  }
</style>