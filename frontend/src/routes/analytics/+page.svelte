<script lang="ts">
  import Icon from "$lib/ui/Icon.svelte";
  import { analyticsApi, type AnalyticsSummary } from '$lib/api/analytics';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import DateRangePicker from '$lib/analytics/DateRangePicker.svelte';
  import { toast } from "$lib/stores/toast";

  let days = $state(30);
  let data = $state<AnalyticsSummary | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedProvider = $state<string>("all");
  let topPosts = $state<PostSummary[]>([]);

  const providers = [
    { value: "all", label: "All Platforms" },
    { value: "x", label: "X (Twitter)" },
    { value: "facebook", label: "Facebook" },
    { value: "instagram", label: "Instagram" },
    { value: "linkedin", label: "LinkedIn" },
    { value: "youtube", label: "YouTube" },
    { value: "reddit", label: "Reddit" },
    { value: "bluesky", label: "Bluesky" },
    { value: "mastodon", label: "Mastodon" },
    { value: "pinterest", label: "Pinterest" },
    { value: "tiktok", label: "TikTok" },
    { value: "threads", label: "Threads" },
  ];

  async function fetchData(signal?: AbortSignal) {
    loading = true;
    error = null;
    const [summaryRes, postsRes] = await Promise.all([
      analyticsApi.getSummary(days, signal),
      postsApi.list({ state: 'published', limit: 50 }),
    ]);
    if (signal?.aborted) return;
    if (summaryRes.error) {
      error = summaryRes.error;
      data = null;
    } else if (summaryRes.data) {
      data = summaryRes.data;
    }
    if (postsRes.data) {
      // Sort by engagement (likes + comments + shares) descending
      topPosts = postsRes.data.posts
        .map(p => ({
          ...p,
          _engagement: (p.likes || 0) + (p.comments || 0) + (p.shares || 0),
        }))
        .sort((a, b) => (b._engagement || 0) - (a._engagement || 0))
        .slice(0, 10) as PostSummary[];
    }
    loading = false;
  }

  $effect(() => {
    const controller = new AbortController();
    fetchData(controller.signal);
    return () => controller.abort();
  });

  function handleDaysChange(newDays: number) {
    days = newDays;
  }

  // Calculate max for chart scaling
  let maxCount = $derived(
    data ? Math.max(...data.posts_by_day.map(d => d.count), 1) : 1
  );
  let maxProviderCount = $derived(
    data ? Math.max(...data.posts_by_provider.map(p => p.count), 1) : 1
  );

  // Filtered provider data
  let filteredProviders = $derived(
    data
      ? (selectedProvider === "all"
        ? data.posts_by_provider
        : data.posts_by_provider.filter(p => p.provider === selectedProvider))
      : []
  );

  // Total engagement from top posts
  let totalEngagement = $derived(
    topPosts.reduce(
      (acc, p) => ({
        likes: acc.likes + (p.likes || 0),
        comments: acc.comments + (p.comments || 0),
        shares: acc.shares + (p.shares || 0),
      }),
      { likes: 0, comments: 0, shares: 0 }
    )
  );
</script>

<div class="page-enter space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <h1 class="text-xl font-bold text-[#e8edf5]">Analytics</h1>
    <div class="flex gap-3 items-center">
      <select
        bind:value={selectedProvider}
        class="px-3 py-1.5 text-sm bg-[#131720] border border-[#1e2435] rounded-lg text-[#e8edf5]"
      >
        {#each providers as p}
          <option value={p.value}>{p.label}</option>
        {/each}
      </select>
      <DateRangePicker selected={days} onChange={handleDaysChange} />
    </div>
  </div>

  {#if error}
    <div class="bg-red-900/20 border border-red-800/40 rounded-lg p-4">
      <p class="text-red-400 text-sm">{error}</p>
      <button onclick={() => fetchData()} class="mt-2 text-sm text-indigo-400 hover:text-indigo-300">Retry</button>
    </div>
  {:else if loading}
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      {#each [1, 2, 3, 4] as _}
        <div class="skeleton h-24 rounded-xl"></div>
      {/each}
    </div>
    <div class="skeleton h-48 rounded-xl"></div>
  {:else if data && data.total_posts === 0}
    <div class="text-center py-16">
      <p class="text-[#d1d5db] mb-4">No analytics data yet. Start posting!</p>
      <a href="/posts/new" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-500 transition-colors text-sm">
        Create Post
      </a>
    </div>
  {:else if data}
    <!-- Summary Cards -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4">
        <div class="text-2xl font-bold text-indigo-400">{data.total_posts}</div>
        <div class="text-xs text-[#6b7280] mt-1 uppercase tracking-wider">Total Posts</div>
      </div>
      <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4">
        <div class="text-2xl font-bold text-green-400">{data.published}</div>
        <div class="text-xs text-[#6b7280] mt-1 uppercase tracking-wider">Published</div>
      </div>
      <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4">
        <div class="text-2xl font-bold text-red-400">{data.failed}</div>
        <div class="text-xs text-[#6b7280] mt-1 uppercase tracking-wider">Failed</div>
      </div>
      <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4">
        <div class="text-2xl font-bold text-amber-400">{data.best_provider?.count ?? 0}</div>
        <div class="text-xs text-[#6b7280] mt-1 uppercase tracking-wider">Best: {data.best_provider?.provider ?? '—'}</div>
      </div>
    </div>

    <!-- Engagement Stats (if available) -->
    {#if totalEngagement.likes > 0 || totalEngagement.comments > 0 || totalEngagement.shares > 0}
      <div class="grid grid-cols-3 gap-4">
        <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4 bg-pink-500/5">
          <div class="flex items-center gap-2">
            <Icon name="heart" class="w-4 h-4 text-pink-400" />
            <div>
              <div class="text-xl font-bold text-pink-400">{totalEngagement.likes.toLocaleString()}</div>
              <div class="text-[10px] text-[#6b7280] uppercase tracking-wider">Total Likes</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4 bg-blue-500/5">
          <div class="flex items-center gap-2">
            <Icon name="comment-bubble" class="w-4 h-4 text-blue-400" />
            <div>
              <div class="text-xl font-bold text-blue-400">{totalEngagement.comments.toLocaleString()}</div>
              <div class="text-[10px] text-[#6b7280] uppercase tracking-wider">Total Comments</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-[#131720] border border-[#1e2435] rounded-xl p-4 bg-green-500/5">
          <div class="flex items-center gap-2">
            <Icon name="share" class="w-4 h-4 text-green-400" />
            <div>
              <div class="text-xl font-bold text-green-400">{totalEngagement.shares.toLocaleString()}</div>
              <div class="text-[10px] text-[#6b7280] uppercase tracking-wider">Total Shares</div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Posts Over Time Chart -->
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
      <h3 class="text-sm font-medium text-[#e8edf5] mb-4">Posts Over Time</h3>
      {#if data.posts_by_day.length === 0}
        <p class="text-[#d1d5db] text-sm py-8 text-center">No data for this period</p>
      {:else}
        <div class="flex items-end gap-1 h-40">
          {#each data.posts_by_day as day (day.date)}
            <div class="flex-1 flex flex-col items-center justify-end h-full">
              <div
                class="w-full bg-indigo-500/80 rounded-t hover:bg-indigo-400 transition-colors min-h-[4px]"
                style="height: {(day.count / maxCount) * 100}%"
                title="{day.date}: {day.count} posts"
              ></div>
            </div>
          {/each}
        </div>
        <div class="flex gap-1 mt-2">
          {#each data.posts_by_day as day (day.date)}
            <div class="flex-1 text-center">
              <span class="text-[10px] text-[#6b7280]">{day.date.slice(5)}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Two-column: Provider Performance + Top Posts -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- Provider Performance -->
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
        <h3 class="text-sm font-medium text-[#e8edf5] mb-4">Posts by Channel</h3>
        {#if filteredProviders.length === 0}
          <p class="text-sm text-[#6b7280] py-4 text-center">No data</p>
        {:else}
          <div class="space-y-2">
            {#each filteredProviders as prov}
              <div class="flex items-center gap-3">
                <span class="text-xs text-[#6b7280] w-24 truncate">{prov.provider}</span>
                <div class="flex-1 bg-[#0d1117] rounded-full h-6 overflow-hidden">
                  <div
                    class="h-full bg-indigo-500/60 rounded-full transition-all duration-500 flex items-center justify-end px-2"
                    style="width: {(prov.count / maxProviderCount) * 100}%"
                  >
                    <span class="text-[10px] font-medium text-white">{prov.count}</span>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Top Posts by Engagement -->
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
        <h3 class="text-sm font-medium text-[#e8edf5] mb-4">Top Posts by Engagement</h3>
        {#if topPosts.length === 0}
          <p class="text-sm text-[#6b7280] py-4 text-center">No published posts with engagement data yet</p>
        {:else}
          <div class="space-y-2">
            {#each topPosts as post, i (post.id)}
              <div class="flex items-start gap-3 py-2 border-b border-[#1e2435] last:border-0">
                <span class="text-xs text-[#4b5563] w-4 mt-0.5">{i + 1}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-sm text-[#d1d5db] truncate">{post.content || post.title || '(no content)'}</p>
                  <div class="flex gap-3 mt-1 text-[10px] text-[#6b7280]">
                    <span>{post.integration_name}</span>
                    {#if post.likes != null && post.likes > 0}
                      <span class="flex items-center gap-0.5 text-pink-400"><Icon name="heart" class="w-3 h-3" /> {post.likes}</span>
                    {/if}
                    {#if post.comments != null && post.comments > 0}
                      <span class="flex items-center gap-0.5 text-blue-400"><Icon name="comment-bubble" class="w-3 h-3" /> {post.comments}</span>
                    {/if}
                    {#if post.shares != null && post.shares > 0}
                      <span class="flex items-center gap-0.5 text-green-400"><Icon name="share" class="w-3 h-3" /> {post.shares}</span>
                    {/if}
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
