<script lang="ts">
  import Icon from "$lib/ui/Icon.svelte";
  import { onMount } from "svelte";
  import { feedApi, proxyMediaUrl, type FeedPost, type FeedAccount } from "$lib/api/feed";
  import { toast } from "$lib/stores/toast";
  import EngagementCard from "$lib/components/EngagementCard.svelte";
  import MediaCarousel from "$lib/media/MediaCarousel.svelte";

  let posts = $state<FeedPost[]>([]);
  let accounts = $state<FeedAccount[]>([]);
  let loading = $state(true);
  let importing = $state(false);
  let searchQuery = $state("");
  let selectedProvider = $state("all");
  let hasMore = $state(false);
  let nextCursor = $state<string | null>(null);

  const platforms = [
    { value: "all", label: "All", icon: "🌐" },
    { value: "x", label: "X", icon: "𝕏" },
    { value: "reddit", label: "Reddit", icon: "𝗥" },
    { value: "facebook", label: "Facebook", icon: "f" },
    { value: "instagram", label: "Instagram", icon: "📷" },
    { value: "youtube", label: "YouTube", icon: "▶" },
    { value: "linkedin", label: "LinkedIn", icon: "in" },
    { value: "bluesky", label: "Bluesky", icon: "☁" },
    { value: "mastodon", label: "Mastodon", icon: "🐘" },
    { value: "pinterest", label: "Pinterest", icon: "📌" },
    { value: "tiktok", label: "TikTok", icon: "🎵" },
    { value: "threads", label: "Threads", icon: "🧵" },
  ];

  // Filtered results (client-side text search on feed data)
  let filteredPosts = $derived(
    posts.filter(p => {
      const matchesProvider = selectedProvider === "all" || p.provider === selectedProvider;
      const matchesQuery = !searchQuery.trim() ||
        p.text.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (p.author_name || "").toLowerCase().includes(searchQuery.toLowerCase()) ||
        (p.author_handle || "").toLowerCase().includes(searchQuery.toLowerCase());
      return matchesProvider && matchesQuery;
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
    const r = await feedApi.list(undefined, undefined, undefined, 100);
    if (r.data) {
      posts = r.data.posts;
      hasMore = r.data.has_more;
      nextCursor = r.data.next_cursor;
    } else if (r.error) {
      toast(`Failed to load feed: ${r.error}`, "error");
    }
    loading = false;
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
    const r = await feedApi.list(nextCursor, undefined, undefined, 50);
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
    const d = new Date(iso);
    const now = new Date();
    const diffH = Math.floor((now.getTime() - d.getTime()) / 3600000);
    if (diffH < 1) return 'just now';
    if (diffH < 24) return `${diffH}h ago`;
    const diffD = Math.floor(diffH / 24);
    if (diffD < 7) return `${diffD}d ago`;
    return d.toLocaleDateString();
  }

  onMount(() => {
    loadSavedSearches();
    loadAccounts();
    load();
  });
</script>

<div class="page-enter space-y-5">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Search & Discovery</h2>
      <p class="text-sm text-[#6b7280] mt-1">Search across your connected social media feeds</p>
    </div>
    <button
      onclick={importFeed}
      disabled={importing}
      class="px-3 py-1.5 text-sm bg-[#1a1f2e] hover:bg-[#1e2435] border border-[#1e2435] rounded-lg transition-colors disabled:opacity-50"
    >
      {importing ? "Importing..." : ""}
    </button>
  </div>

  <!-- Search Bar -->
  <div class="flex gap-3">
    <div class="relative flex-1">
      <input
        type="text"
        bind:value={searchQuery}
        placeholder="Search posts, authors, hashtags..."
        class="w-full px-4 py-2.5 pl-10 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db] placeholder-[#6b7280] focus:border-indigo-500 outline-none transition-colors"
      />
      <span class="absolute left-3 top-1/2 -translate-y-1/2 text-[#6b7280]"><Icon name="search" class="w-4 h-4" /></span>
    </div>
    <button
      onclick={() => showSaveDialog = true}
      disabled={!searchQuery.trim()}
      class="px-3 py-2 text-sm bg-[#1a1f2e] hover:bg-[#1e2435] border border-[#1e2435] rounded-lg transition-colors disabled:opacity-30"
      title="Save this search"
    >
      <Icon name="bookmark" class="w-3.5 h-3.5 inline" /> Save
    </button>
  </div>

  <!-- Saved Searches -->
  {#if savedSearches.length > 0}
    <div class="flex gap-2 flex-wrap">
      {#each savedSearches as term}
        <div class="flex items-center gap-1 px-3 py-1 bg-[#131720] border border-[#1e2435] rounded-full text-xs">
          <button onclick={() => searchQuery = term} class="text-[#6b7280] hover:text-indigo-400 transition-colors">
            {term}
          </button>
          <button onclick={() => removeSearch(term)} class="text-[#4b5563] hover:text-red-400 transition-colors ml-1">
            ✕
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- Platform Tabs -->
  <div class="flex gap-1 bg-[#131720] border border-[#1e2435] rounded-lg p-1 overflow-x-auto">
    {#each platforms as p}
      <button
        onclick={() => selectedProvider = p.value}
        class="px-3 py-1.5 text-xs rounded-md transition-colors whitespace-nowrap flex items-center gap-1.5
          {selectedProvider === p.value ? 'bg-indigo-600 text-white' : 'text-[#6b7280] hover:bg-[#1a1f2e]'}"
      >
        <span>{p.icon}</span>
        {p.label}
      </button>
    {/each}
  </div>

  <!-- Results count -->
  <div class="flex items-center justify-between">
    <span class="text-xs text-[#6b7280]">
      {filteredPosts.length} result{filteredPosts.length !== 1 ? 's' : ''}
      {searchQuery.trim() ? ` for "${searchQuery}"` : ''}
    </span>
    {#if accounts.length > 0}
      <span class="text-xs text-[#4b5563]">{accounts.length} accounts tracked</span>
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
      <Icon name="search" class="w-8 h-8 text-[#6b7280]" />
      <p class="text-[#d1d5db] mt-3 mb-1">
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
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4 hover:border-[#2a3045] transition-colors">
          <div class="flex items-start gap-3">
            <!-- Avatar -->
            {#if post.author_avatar}
              <img src={proxyMediaUrl(post.author_avatar)} alt="" class="w-10 h-10 rounded-full flex-shrink-0 object-cover" />
            {:else}
              <div class="w-10 h-10 rounded-full bg-[#1e2435] flex items-center justify-center flex-shrink-0">
                <span class="text-sm {providerColor(post.provider)}">{providerIcon(post.provider)}</span>
              </div>
            {/if}

            <div class="flex-1 min-w-0">
              <!-- Author + time -->
              <div class="flex items-center gap-2 mb-1">
                <span class="text-sm font-medium text-[#e8edf5]">{post.author_name || 'Unknown'}</span>
                {#if post.author_handle}
                  <span class="text-xs text-[#6b7280]">@{post.author_handle}</span>
                {/if}
                <span class="text-xs {providerColor(post.provider)}">{providerIcon(post.provider)}</span>
                <span class="text-xs text-[#4b5563] ml-auto">{formatTime(post.created_at)}</span>
              </div>

              <!-- Text -->
              <p class="text-sm text-[#d1d5db] mb-2 whitespace-pre-wrap break-words">{post.text}</p>

              <!-- Media -->
              {#if post.media.length > 0}
                <div class="mb-2">
                  <MediaCarousel media={post.media} />
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
        <button onclick={loadMore} class="px-4 py-2 text-sm bg-[#1a1f2e] hover:bg-[#1e2435] border border-[#1e2435] rounded-lg transition-colors">
          Load More
        </button>
      </div>
    {/if}
  {/if}
</div>

<!-- Save Search Dialog -->
{#if showSaveDialog}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-[#0d1117] border border-[#1e2435] rounded-xl p-6 w-full max-w-sm">
      <h3 class="text-lg font-semibold mb-2">Save Search</h3>
      <p class="text-sm text-[#6b7280] mb-4">Save "{searchQuery}" for quick access later</p>
      <div class="flex gap-3 justify-end">
        <button onclick={() => showSaveDialog = false} class="px-4 py-2 text-sm text-[#6b7280] hover:text-white">Cancel</button>
        <button onclick={saveSearch} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded-lg">Save</button>
      </div>
    </div>
  </div>
{/if}
