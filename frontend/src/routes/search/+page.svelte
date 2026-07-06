<script lang="ts">
  import Icon from "$lib/ui/Icon.svelte";
  import { onMount, onDestroy } from "svelte";
  import { feedApi, proxyMediaUrl, type FeedPost, type FeedAccount } from "$lib/api/feed";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";
  import { timezone } from "$lib/stores/timezone.svelte";
  import EngagementCard from "$lib/components/EngagementCard.svelte";
  import MediaCarousel from "$lib/media/MediaCarousel.svelte";

  let posts = $state<FeedPost[]>([]);
  let accounts = $state<FeedAccount[]>([]);
  let connectedIntegrations = $state<Integration[]>([]);
  let loading = $state(true);
  let importing = $state(false);
  let searchQuery = $state("");
  let selectedProvider = $state("all");
  let hasMore = $state(false);
  let nextCursor = $state<string | null>(null);
  // Debounce timer for server-side ?q= search.
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  // Tracks the most recent query we sent to the backend so we can ignore
  // stale responses that arrive after the user has typed more.
  let lastSentQuery = $state<string>("");

  // Build platform list dynamically from connected integrations
  let platforms = $derived.by(() => {
    const connected = connectedIntegrations
      .filter(i => !i.disabled)
      .map(i => i.provider_identifier);
    const unique = [...new Set(connected)];
    const icons: Record<string, string> = {
      x: "X", reddit: "R", facebook: "f", instagram: "IG", youtube: "YT",
      linkedin: "in", bluesky: "BS", mastodon: "MA", pinterest: "PIN",
      tiktok: "TT", threads: "TH", discord: "DC", slack: "SL",
      "telegram-bot": "TG", whatsapp: "WA", "instagram-standalone": "IG",
      "linkedin-page": "in", wordpress: "WP", medium: "MD", devto: "DT",
      hashnode: "HN", github: "GH", vk: "VK", kick: "KI", skool: "SK",
    };
    return [
      { value: "all", label: "All", icon: "ALL" },
      ...unique.map(p => ({ value: p, label: p.charAt(0).toUpperCase() + p.slice(1), icon: icons[p] || p.slice(0, 2).toUpperCase() })),
    ];
  });

  // Filtered results: when the user types a search query, the backend
  // already filters via ?q= (server-side ILIKE across text/author fields),
  // so we only need client-side provider filtering on the response set.
  // When no query is typed, we still apply provider filtering client-side
  // for the "All" tab to avoid an extra round-trip.
  let filteredPosts = $derived(
    posts.filter(p => {
      const matchesProvider = selectedProvider === "all" || p.provider === selectedProvider;
      return matchesProvider;
    })
  );

  // Saved searches (localStorage)
  let savedSearches = $state<string[]>([]);
  let showSaveDialog = $state(false);
  let newSearchName = $state("");

  function loadSavedSearches() {
    try {
      const stored = localStorage.getItem("sf_saved_searches");
      if (stored) savedSearches = JSON.parse(stored);
    } catch { /* ignore */ }
  }

  function saveSearch() {
    if (!searchQuery.trim()) return;
    const entry = searchQuery.trim();
    if (!savedSearches.includes(entry)) {
      savedSearches = [...savedSearches, entry];
      localStorage.setItem("sf_saved_searches", JSON.stringify(savedSearches));
      toast("Search saved", "success");
    }
    showSaveDialog = false;
  }

  function removeSearch(term: string) {
    savedSearches = savedSearches.filter(s => s !== term);
    localStorage.setItem("sf_saved_searches", JSON.stringify(savedSearches));
  }

  async function load() {
    loading = true;
    const q = searchQuery.trim() || undefined;
    // Track which query this request corresponds to so we can ignore
    // stale responses (user typed more before the first response arrived).
    lastSentQuery = q || "";
    const r = await feedApi.list(undefined, undefined, undefined, 100, q);
    if (r.data) {
      // Ignore stale responses — only apply if the user hasn't typed more.
      if (lastSentQuery !== (q || "")) {
        loading = false;
        return;
      }
      posts = r.data.posts;
      hasMore = r.data.has_more;
      nextCursor = r.data.next_cursor;
    } else if (r.error) {
      toast(`Failed to load feed: ${r.error}`, "error");
    }
    loading = false;
  }

  // Debounced search trigger — fires 350ms after the user stops typing.
  // Coalesces rapid keystrokes so we don't spam the backend on every char.
  function scheduleSearch() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      searchTimer = null;
      nextCursor = null;
      load();
    }, 350);
  }

  function onSearchInput() {
    scheduleSearch();
  }

  async function loadAccounts() {
    const r = await feedApi.accounts();
    if (r.data) {
      accounts = r.data;
    }
  }

  async function importFeed() {
    importing = true;
    const r = await feedApi.import();
    if (r.data) {
      toast(`Imported ${r.data.imported} posts`, "success");
      await load();
    } else if (r.error) {
      toast(`Import failed: ${r.error}`, "error");
    }
    importing = false;
  }

  async function loadMore() {
    if (!nextCursor) return;
    // Pass the same search query through to the next page so pagination
    // stays within the result set of the current search.
    const q = searchQuery.trim() || undefined;
    const r = await feedApi.list(nextCursor, undefined, undefined, 50, q);
    if (r.data) {
      posts = [...posts, ...r.data.posts];
      hasMore = r.data.has_more;
      nextCursor = r.data.next_cursor;
    }
  }

  function providerIcon(p: string): string {
    const found = platforms.find(pl => pl.value === p);
    return found?.icon || "•";
  }

  function providerColor(p: string): string {
    const colors: Record<string, string> = {
      x: 'text-gray-300', reddit: 'text-orange-400', linkedin: 'text-blue-400',
      facebook: 'text-blue-500', instagram: 'text-pink-400', youtube: 'text-red-400',
      bluesky: 'text-sky-400', mastodon: 'text-purple-400', pinterest: 'text-red-500',
      tiktok: 'text-white', threads: 'text-gray-400',
    };
    return colors[p] || 'text-gray-400';
  }

  function formatTime(iso: string): string {
    // Render in the user's selected timezone (F-7) instead of the
    // browser's local timezone. Relative-time strings ("just now",
    // "5h ago") are timezone-independent so they stay as-is.
    const d = new Date(iso);
    const now = new Date();
    const diffH = Math.floor((now.getTime() - d.getTime()) / 3600000);
    if (diffH < 1) return 'just now';
    if (diffH < 24) return `${diffH}h ago`;
    const diffD = Math.floor(diffH / 24);
    if (diffD < 7) return `${diffD}d ago`;
    return timezone.formatDate(iso);
  }

  let unsubscribers: (() => void)[] = [];

  onMount(async () => {
    loadSavedSearches();
    loadAccounts();
    load();
    const integRes = await integrationsApi.list();
    if (integRes.data) connectedIntegrations = integRes.data.integrations;
    // Realtime: when the feed refresher pulls new posts in the
    // background, refresh the search results so they appear without
    // a manual reload. Also refresh on integration changes (new
    // account connected → new feed source available).
    unsubscribers.push(realtime.on('integration_connected', () => {
      loadAccounts();
      load();
    }));
    unsubscribers.push(realtime.on('integration_disconnected', () => {
      loadAccounts();
      load();
    }));
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
    if (searchTimer) clearTimeout(searchTimer);
  });
</script>

<div class="page-enter space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Search & Discovery</h2>
      <p class="text-sm text-muted mt-1">Search across your connected social media feeds</p>
    </div>
    <button
      onclick={importFeed}
      disabled={importing}
      class="px-3 py-1.5 text-sm bg-surface-hover hover:bg-line border border-line rounded-lg transition-colors disabled:opacity-50"
    >
      {importing ? "Importing..." : "Import Feed"}
    </button>
  </div>

  <!-- Search Bar -->
  <div class="flex gap-3">
    <div class="relative flex-1">
      <input
        type="text"
        bind:value={searchQuery}
        oninput={onSearchInput}
        placeholder="Search posts, authors, hashtags..."
        class="w-full px-4 py-2.5 pl-10 bg-background-input border border-line rounded-lg text-sm text-content-secondary placeholder-muted focus:border-indigo-500 outline-none transition-colors"
      />
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-muted"><Icon name="search" class="w-4 h-4" /></span>
    </div>
    <button
      onclick={() => showSaveDialog = true}
      disabled={!searchQuery.trim()}
      class="px-3 py-2 text-sm bg-surface-hover hover:bg-line border border-line rounded-lg transition-colors disabled:opacity-30"
      title="Save this search"
    >
      <Icon name="bookmark" class="w-3.5 h-3.5 inline" /> Save
    </button>
  </div>

  <!-- Saved Searches -->
  {#if savedSearches.length > 0}
    <div class="flex gap-2 flex-wrap">
      {#each savedSearches as term}
        <div class="flex items-center gap-1 px-3 py-1 bg-surface border border-line rounded-full text-xs">
          <button onclick={() => { searchQuery = term; onSearchInput(); }} class="text-muted hover:text-indigo-400 transition-colors">
            {term}
          </button>
          <button onclick={() => removeSearch(term)} class="text-muted-dark hover:text-red-400 transition-colors ml-1">
            ✕
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Platform Tabs -->
  <div class="flex gap-1 bg-surface border border-line rounded-lg p-1 overflow-x-auto">
    {#each platforms as p}
      <button
        onclick={() => selectedProvider = p.value}
        class="px-3 py-1.5 text-xs rounded-md transition-colors whitespace-nowrap flex items-center gap-1.5
          {selectedProvider === p.value ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover'}"
      >
        <span>{p.icon}</span>
        {p.label}
      </button>
    {/each}
  </div>

  <!-- Results count -->
  <div class="flex items-center justify-between">
    <span class="text-xs text-muted">
      {filteredPosts.length} result{filteredPosts.length !== 1 ? 's' : ''}
      {searchQuery.trim() ? ` for "${searchQuery}"` : ''}
    </span>
    {#if accounts.length > 0}
      <span class="text-xs text-muted-dark">{accounts.length} accounts tracked</span>
    {/if}
  </div>

  <!-- Results -->
  {#if loading}
    <div class="space-y-3">
      {#each [1, 2, 3] as _}
        <div class="skeleton h-32 rounded-xl"></div>
      {/each}
    </div>
  {:else if filteredPosts.length === 0}
    <div class="text-center py-16">
      <Icon name="search" class="w-8 h-8 text-muted" />
      <p class="text-content-secondary mt-3 mb-1">
        {#if posts.length === 0}
          No feed data yet. Click "Refresh Feed" to import posts from your connected accounts.
        {:else}
          No results found{searchQuery.trim() ? ` for "${searchQuery}"` : ''}.
        {/if}
      </p>
      {#if posts.length === 0}
        <button onclick={importFeed} disabled={importing} class="mt-3 px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors disabled:opacity-50">
          {importing ? "Importing..." : "Import Feed"}
        </button>
      {/if}
    </div>
  {:else}
    <div class="space-y-3">
      {#each filteredPosts as post (post.id)}
        <div class="bg-surface border border-line rounded-xl p-4 hover:border-line transition-colors">
          <div class="flex items-start gap-3">
            <!-- Avatar -->
            {#if post.author_avatar}
              <img src={proxyMediaUrl(post.author_avatar)} alt="" class="w-10 h-10 rounded-full flex-shrink-0 object-cover" />
            {:else}
              <div class="w-10 h-10 rounded-full bg-line flex items-center justify-center flex-shrink-0">
                <span class="text-sm {providerColor(post.provider)}">{providerIcon(post.provider)}</span>
              </div>
            {/if}

            <div class="flex-1 min-w-0">
              <!-- Author + time -->
              <div class="flex items-center gap-2 mb-1">
                <span class="text-sm font-medium text-content">{post.author_name || 'Unknown'}</span>
                {#if post.author_handle}
                  <span class="text-xs text-muted">@{post.author_handle}</span>
                {/if}
                <span class="text-xs {providerColor(post.provider)}">{providerIcon(post.provider)}</span>
                <span class="text-xs text-muted-dark ml-auto">{formatTime(post.created_at)}</span>
              </div>

              <!-- Text -->
              <p class="text-sm text-content-secondary mb-2 whitespace-pre-wrap break-words">{post.text}</p>

              <!-- Media -->
              {#if post.media.length > 0}
                <div class="mb-2">
                  <MediaCarousel items={post.media} />
                </div>
              {/if}

              <!-- Engagement + link -->
              <div class="flex items-center gap-4">
                {#if post.engagement}
                  <EngagementCard engagement={post.engagement} provider={post.provider} compact={true} />
                {/if}
                {#if post.url}
                  <a href={post.url} target="_blank" rel="noopener" class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors ml-auto">
                    View original →
                  </a>
                {/if}
              </div>
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Load more -->
    {#if hasMore}
      <div class="text-center py-4">
        <button onclick={loadMore} class="px-4 py-2 text-sm bg-surface-hover hover:bg-line border border-line rounded-lg transition-colors">
          Load More
        </button>
      </div>
    {/if}
  {/if}
</div>

<!-- Save Search Dialog -->
{#if showSaveDialog}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-sm">
      <h3 class="text-lg font-semibold mb-2">Save Search</h3>
      <p class="text-sm text-muted mb-4">Save "{searchQuery}" for quick access later</p>
      <div class="flex gap-3 justify-end">
        <button onclick={() => showSaveDialog = false} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={saveSearch} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded-lg">Save</button>
      </div>
    </div>
  </div>
{/if}
