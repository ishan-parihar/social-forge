<script lang="ts">
  import { onMount } from "svelte";
  import { feedApi, type FeedPost, type FeedAccount } from "$lib/api/feed";
  import EngagementCard from "$lib/components/EngagementCard.svelte";
  import CommentsThread from "$lib/components/CommentsThread.svelte";
  import MediaCarousel from "$lib/media/MediaCarousel.svelte";

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

  function providerMeta(provider: string): { label: string; color: string; bg: string; dot: string } {
    const meta: Record<string, { label: string; color: string; bg: string; dot: string }> = {
      x:          { label: 'X',      color: '#9ca3af', bg: '#1f222e', dot: '#9ca3af' },
      reddit:     { label: 'Reddit', color: '#fb923c', bg: '#2a1e1a', dot: '#fb923c' },
      bluesky:    { label: 'Bluesky',color: '#38bdf8', bg: '#18222e', dot: '#38bdf8' },
      github:     { label: 'GitHub', color: '#d1d5db', bg: '#1c212e', dot: '#d1d5db' },
      devto:      { label: 'Dev.to', color: '#9ca3af', bg: '#1c212e', dot: '#9ca3af' },
      mastodon:   { label: 'Mastodon',color:'#38bdf8',bg: '#18222e', dot: '#38bdf8' },
      lemmy:      { label: 'Lemmy',  color: '#f97316', bg: '#2a1e1a', dot: '#f97316' },
      medium:     { label: 'Medium', color: '#22c55e', bg: '#18221a', dot: '#22c55e' },
      wordpress:  { label: 'WordPress',color:'#60a5fa',bg: '#1a222e', dot: '#60a5fa' },
      linkedin:   { label: 'LinkedIn',color:'#3b82f6',bg: '#1a2230', dot: '#3b82f6' },
      facebook:   { label: 'Facebook',color:'#2563eb',bg: '#161e2e', dot: '#2563eb' },
      instagram:  { label: 'Instagram',color:'#f472b6',bg: '#2a1a24', dot: '#f472b6' },
      threads:    { label: 'Threads',color:'#a78bfa',bg: '#1e1a2e', dot: '#a78bfa' },
      youtube:    { label: 'YouTube',color:'#ef4444',bg: '#2a1a1a', dot: '#ef4444' },
      pinterest:  { label: 'Pinterest',color:'#f87171',bg: '#2a1a1a', dot: '#f87171' },
      tiktok:     { label: 'TikTok', color:'#67e8f9',bg: '#16222e', dot: '#67e8f9' },
      hashnode:   { label: 'Hashnode',color:'#60a5fa',bg: '#1a222e', dot: '#60a5fa' },
      vk:         { label: 'VK',     color:'#60a5fa',bg: '#1a222e', dot: '#60a5fa' },
    };
    return meta[provider] || { label: provider.replace(/_/g, ' '), color: '#818cf8', bg: '#1a1a2e', dot: '#818cf8' };
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
  });
</script>

<div class="max-w-3xl mx-auto space-y-5">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-3">
      <h2 class="text-xl font-bold text-[#e8edf5] tracking-tight">Feed</h2>
      {#if !loading && posts.length > 0}
        <span class="text-xs text-[#6b7280] font-mono bg-[#161b28] px-2 py-0.5 rounded-full border border-[#1e2435]">
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
          bg-[#161b28] text-[#9ca3af] border border-[#1e2435]
          hover:border-[#2a3045] hover:text-[#e8edf5]
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
          bg-indigo-500/15 text-indigo-300 border border-indigo-500/25">
          {#if chipMeta}
            <span class="w-1.5 h-1.5 rounded-full" style="background: {chipMeta.dot}" />
          {/if}
          {activeFilterLabel}
          <button onclick={clearFilter} class="ml-0.5 hover:text-indigo-200 transition-colors">
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
                ? 'bg-indigo-500/20 text-indigo-300 border border-indigo-500/30 shadow-[0_0_12px_rgba(99,102,241,0.1)]'
                : 'bg-[#161b28] text-[#9ca3af] border border-[#1e2435] hover:border-[#2a3045] hover:text-[#e8edf5]'}"
          >
            <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M2 4h12M4 8h8M6 12h4" stroke-linecap="round" />
            </svg>
            {activeFilterLabel ? activeFilterLabel : 'Channels'}
          </button>

          <!-- Filter dropdown -->
          {#if showFilter}
            <div class="absolute right-0 top-full mt-2 w-72 z-50
              bg-[#111622] border border-[#1e2435] rounded-xl shadow-2xl shadow-black/40
              overflow-hidden motion-safe:animate-in duration-200">
              <!-- Header -->
              <div class="px-4 py-3 border-b border-[#1e2435]">
                <p class="text-xs font-semibold text-[#e8edf5]">Filter by channel</p>
                <p class="text-[10px] text-[#6b7280] mt-0.5">
                  {allAccounts.length} account{allAccounts.length !== 1 ? 's' : ''} connected
                </p>
              </div>

              <!-- "All channels" option -->
              <button
                onclick={() => clearFilter()}
                class="w-full flex items-center gap-3 px-4 py-2.5 text-left transition-colors duration-150 border-b border-[#1e2435]
                  {!selectedAccountHandle && !selectedProvider
                    ? 'bg-indigo-500/10 text-[#e8edf5]'
                    : 'text-[#6b7280] hover:text-[#9ca3af] hover:bg-[#0d121e]'}"
              >
                <svg class="w-4 h-4 flex-shrink-0" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                  <path d="M2 4h12M4 8h8M6 12h4" stroke-linecap="round" />
                </svg>
                <span class="text-sm font-medium">All channels</span>
                {#if !selectedAccountHandle && !selectedProvider}
                  <span class="ml-auto w-1.5 h-1.5 rounded-full bg-indigo-400" />
                {/if}
              </button>

              <!-- Accounts grouped by provider -->
              <div class="max-h-80 overflow-y-auto py-1">
                {#each [...providerGroups.entries()] as [provider, accounts]}
                  {@const pmeta = providerMeta(provider)}
                  <!-- Provider header -->
                  <div class="px-4 py-1.5 flex items-center gap-2">
                    <span class="w-1.5 h-1.5 rounded-full flex-shrink-0" style="background: {pmeta.dot}" />
                    <span class="text-[10px] font-semibold uppercase tracking-wider text-[#5a6070]">{pmeta.label}</span>
                  </div>

                  <!-- Accounts in this provider -->
                  {#each accounts as acct}
                    <button
                      onclick={() => selectAccount(acct.author_handle, acct.provider)}
                      class="w-full flex items-center gap-3 px-4 py-2 text-left transition-colors duration-150
                        {selectedAccountHandle === acct.author_handle
                          ? 'bg-indigo-500/10 text-[#e8edf5]'
                          : 'text-[#9ca3af] hover:text-[#e8edf5] hover:bg-[#0d121e]'}"
                    >
                      {#if acct.author_avatar}
                        <img
                          src={acct.author_avatar}
                          alt=""
                          class="w-6 h-6 rounded-full flex-shrink-0 object-cover ring-1 ring-[#1e2435]"
                        />
                      {:else}
                        <span class="w-6 h-6 rounded-full flex-shrink-0 bg-[#1e2435] flex items-center justify-center">
                          <svg class="w-3 h-3 text-[#5a6070]" viewBox="0 0 16 16" fill="currentColor">
                            <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6a5 5 0 0110 0H3z"/>
                          </svg>
                        </span>
                      {/if}
                      <div class="flex-1 min-w-0">
                        <div class="text-sm font-medium truncate">{acct.author_name || acct.author_handle || 'Unknown'}</div>
                        {#if acct.author_handle}
                          <div class="text-[10px] text-[#5a6070] truncate">@{acct.author_handle}</div>
                        {/if}
                      </div>
                      {#if selectedAccountHandle === acct.author_handle}
                        <span class="ml-auto w-1.5 h-1.5 rounded-full bg-indigo-400 flex-shrink-0" />
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
      <div class="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-red-900/15 border border-red-800/30 text-sm text-red-400">
        <svg class="w-4 h-4 flex-shrink-0" viewBox="0 0 16 16" fill="currentColor">
          <path d="M8 1a7 7 0 110 14A7 7 0 018 1zm0 9.5a.75.75 0 100 1.5.75.75 0 000-1.5zM8.75 4.5a.75.75 0 00-1.5 0v4a.75.75 0 001.5 0v-4z"/>
        </svg>
        {fetchError}
      </div>
      <button
        onclick={load}
        class="mt-4 px-4 py-2 text-xs font-medium rounded-lg bg-[#1a1f2e] text-[#9ca3af] hover:text-[#e8edf5] hover:bg-[#1e2435] border border-[#1e2435] transition-colors">
        Try again
      </button>
    </div>

  <!-- Loading skeleton -->
  {:else if loading}
    <div class="space-y-3    motion-safe:animate-in duration-300">
      {#each Array(5) as _, i (i)}
        <div class="bg-[#131825] rounded-xl p-5 border border-[#1a2035] space-y-3">
          <div class="flex items-center gap-2.5">
            <div class="w-6 h-6 rounded-full bg-[#1e2435]" />
            <div class="h-3 bg-[#1e2435] rounded w-20" />
            <div class="h-2.5 bg-[#1e2435] rounded w-16" />
            <div class="ml-auto h-2.5 bg-[#1e2435] rounded w-12" />
          </div>
          <div class="space-y-2">
            <div class="h-3 bg-[#1e2435] rounded w-full" />
            <div class="h-3 bg-[#1e2435] rounded w-5/6" />
            <div class="h-3 bg-[#1e2435] rounded w-2/3" />
          </div>
        </div>
      {/each}
    </div>

  <!-- Empty state: importing for the first time -->
  {:else if importing}
    <div class="text-center py-20">
      <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-[#131825] border border-[#1e2435] flex items-center justify-center">
        <svg class="w-7 h-7 text-indigo-400 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M12 2v4M12 18v4M4.93 4.93l2.83 2.83M16.24 16.24l2.83 2.83M2 12h4M18 12h4M4.93 19.07l2.83-2.83M16.24 7.76l2.83-2.83" stroke-linecap="round" />
        </svg>
      </div>
      <p class="text-sm font-medium text-[#e8edf5]">Importing your posts…</p>
      <p class="text-xs text-[#6b7280] mt-1">Fetching recent posts from all your connected providers</p>
      <div class="mt-6 w-48 h-1 mx-auto bg-[#1e2435] rounded-full overflow-hidden">
        <div class="h-full bg-indigo-500/50 rounded-full animate-pulse" style="width: 60%" />
      </div>
    </div>

  <!-- Empty state (already attempted) -->
  {:else if !initialLoad && posts.length === 0 && attemptedImport}
    <div class="text-center py-20">
      <div class="w-16 h-16 mx-auto mb-4 rounded-2xl bg-[#131825] border border-[#1e2435] flex items-center justify-center">
        <svg class="w-7 h-7 text-[#6b7280]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M19 20H5a2 2 0 01-2-2V6a2 2 0 012-2h10a2 2 0 012 2v1m2 13a2 2 0 01-2-2V7m2 13a2 2 0 002-2V9a2 2 0 00-2-2h-2m-4-3H9M7 16h6M7 8h6v4H7V8z" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </div>
      <p class="text-sm font-medium text-[#9ca3af] mb-1">No posts found</p>
      <p class="text-xs text-[#6b7280] mb-6">Connect a social media account to see your feed here</p>
      <button
        onclick={triggerImport}
        class="inline-flex items-center gap-2 px-4 py-2 text-xs font-medium rounded-lg
          bg-[#1a1f2e] text-[#9ca3af] hover:text-[#e8edf5] hover:bg-[#1e2435]
          border border-[#1e2435] transition-colors"
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
      <div class="w-14 h-14 mx-auto mb-4 rounded-2xl bg-[#131825] border border-[#1e2435] flex items-center justify-center">
        <svg class="w-6 h-6 text-[#6b7280]" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M2 4h12M4 8h8M6 12h4" stroke-linecap="round" />
        </svg>
      </div>
      <p class="text-sm font-medium text-[#9ca3af]">No posts match your filters</p>
      <button onclick={clearFilter} class="mt-3 text-xs text-indigo-400 hover:text-indigo-300 transition-colors">
        Clear filters
      </button>
    </div>

  <!-- Post list -->
  {:else}
    <div class="space-y-2.5">
      {#each filteredPosts as post (post.id)}
        {@const meta = providerMeta(post.provider)}
        <article
          class="group relative bg-[#131825] rounded-xl border border-[#1a2035] transition-all duration-200
            hover:border-[#222a45] hover:bg-[#151b2a]"
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
                  class="w-5 h-5 rounded-full flex-shrink-0 object-cover ring-1 ring-[#1e2435]"
                  loading="lazy"
                />
              {/if}

              <!-- Author -->
              {#if post.author_name}
                <span class="text-sm font-medium text-[#e8edf5]">{post.author_name}</span>
              {/if}
              {#if post.author_handle}
                <span class="text-xs text-[#5a6070]">@{post.author_handle}</span>
              {/if}

              <!-- Time -->
              <span class="ml-auto text-[11px] text-[#5a6070] font-mono">{formatTime(post.created_at)}</span>
            </div>

            <!-- Content -->
            <p class="text-sm text-[#cdd2dc] leading-relaxed whitespace-pre-wrap break-words
              selection:bg-indigo-500/20 selection:text-indigo-200">
              {post.text}
            </p>

            <!-- Media attachments -->
            {#if post.media && post.media.length > 0}
              {#if post.media.length === 1}
                {@const item = post.media[0]}
                <div class="mt-3">
                  <div class="rounded-xl overflow-hidden bg-[#0d121e] ring-1 ring-[#1e2435] group/media">
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
                          src={item.url}
                          controls
                          preload="metadata"
                          playsinline
                          class="w-full max-h-[65vh] bg-black"
                          poster={item.poster_url ?? ''}
                        >
                          <a href={item.url} target="_blank" rel="noopener noreferrer"
                            class="text-xs text-indigo-400 hover:text-indigo-300 underline p-2 block">
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
                          src={item.url}
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
                    ? 'bg-indigo-500/15 text-indigo-300 border border-indigo-500/25'
                    : 'text-[#5a6070] hover:text-[#9ca3af] hover:bg-[#0d121e] border border-transparent'}"
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

            <!-- Footer: link -->
            {#if post.url}
              <div class="mt-3 pt-3 border-t border-[#1a2035]">
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
              </div>
            {/if}
          </div>
        </article>
      {/each}
    </div>

    <!-- Bottom section: Load More or end -->
    <div bind:this={sentinel} class="py-6">
      {#if hasMore}
        {#if loadingMore}
          <div class="flex items-center justify-center gap-2.5 py-4">
            <div class="w-5 h-5 rounded-full border-2 border-indigo-400/30 border-t-indigo-400 animate-spin" />
            <span class="text-xs text-[#6b7280] font-mono">Loading more…</span>
          </div>
        {:else if nearBottom}
          <div class="flex justify-center            motion-safe:animate-in duration-300">
            <button
              onclick={loadMore}
              class="group flex items-center gap-2.5 px-6 py-3 text-sm font-medium rounded-xl
                bg-[#1a2035] border border-[#222a45] text-[#cdd2dc]
                hover:bg-[#1e2440] hover:border-indigo-500/30 hover:text-[#e8edf5]
                transition-all duration-200 shadow-sm hover:shadow-[0_0_20px_rgba(99,102,241,0.08)]"
            >
              <svg class="w-4 h-4 transition-transform group-hover:rotate-180 duration-300" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M8 3v10M4 9l4 4 4-4" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
              Load more posts
              {#if nextCursor}
                <span class="text-[10px] text-[#5a6070] font-mono">· next page</span>
              {/if}
            </button>
          </div>
        {:else}
          <div class="flex justify-center">
            <span class="text-[10px] text-[#4a5060] font-mono tracking-wider uppercase">Scroll down to load more</span>
          </div>
        {/if}
      {:else if posts.length > 0}
        <div class="flex flex-col items-center gap-2 py-8">
          <div class="w-12 h-px bg-gradient-to-r from-transparent via-[#1e2435] to-transparent" />
          <span class="text-[10px] text-[#4a5060] font-mono tracking-wider">You're all caught up</span>
          <div class="w-12 h-px bg-gradient-to-r from-transparent via-[#1e2435] to-transparent" />
        </div>
      {/if}
    </div>
  {/if}
</div>

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