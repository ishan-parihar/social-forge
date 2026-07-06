<script lang="ts">
  import Icon from "$lib/ui/Icon.svelte";
  import { analyticsApi, type AnalyticsSummary } from '$lib/api/analytics';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { feedApi } from '$lib/api/feed';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import DateRangePicker from '$lib/analytics/DateRangePicker.svelte';
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";
  import { onMount, onDestroy } from "svelte";

  let days = $state(30);
  let data = $state<AnalyticsSummary | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedProvider = $state<string>("all");
  let topPosts = $state<PostSummary[]>([]);
  let providerAnalytics = $state<import("$lib/api/analytics").ProviderAnalytics | null>(null);
  let feedEngagement = $state<{ total_posts: number; total_likes: number; total_comments: number; total_shares: number; total_impressions: number } | null>(null);
  let connectedIntegrations = $state<Integration[]>([]);

  // Build provider list dynamically from connected integrations
  let providers = $derived.by(() => {
    const connected = connectedIntegrations
      .filter(i => !i.disabled)
      .map(i => i.provider_identifier);
    const unique = [...new Set(connected)];
    const labels: Record<string, string> = {
      x: "X (Twitter)", facebook: "Facebook", instagram: "Instagram",
      linkedin: "LinkedIn", youtube: "YouTube", reddit: "Reddit",
      bluesky: "Bluesky", mastodon: "Mastodon", pinterest: "Pinterest",
      tiktok: "TikTok", threads: "Threads", "instagram-standalone": "Instagram (Standalone)",
      "linkedin-page": "LinkedIn Page", discord: "Discord", slack: "Slack",
      "telegram-bot": "Telegram Bot", whatsapp: "WhatsApp", wordpress: "WordPress",
      medium: "Medium", devto: "Dev.to", hashnode: "Hashnode", github: "GitHub",
      vk: "VK", kick: "Kick", skool: "Skool", lemmy: "Lemmy", farcaster: "Farcaster",
    };
    return [
      { value: "all", label: "All Platforms" },
      ...unique.map(p => ({ value: p, label: labels[p] || p })),
    ];
  });

  async function fetchData(signal?: AbortSignal) {
    loading = true;
    error = null;
    const requests: Promise<unknown>[] = [
      analyticsApi.getSummary(days, signal),
      postsApi.list({ state: 'published', limit: 50 }),
      feedApi.analytics(days),
    ];
    if (selectedProvider !== "all") {
      requests.push(analyticsApi.getProvider(selectedProvider, days, signal));
    }
    const [summaryRes, postsRes, feedRes, provRes] = await Promise.all(requests) as [
      typeof summaryRes, typeof postsRes, typeof feedRes, typeof provRes
    ];
    if (signal?.aborted) return;
    if (summaryRes.error) {
      error = summaryRes.error;
      data = null;
    } else if (summaryRes.data) {
      data = summaryRes.data;
    }
    if (feedRes.data) {
      feedEngagement = feedRes.data;
    }
    if (postsRes.data) {
      let posts = postsRes.data.posts;
      if (selectedProvider !== "all") {
        posts = posts.filter(p =>
          p.integration_name?.toLowerCase().includes(selectedProvider)
        );
      }
      topPosts = posts
        .map(p => ({
          ...p,
          _engagement: (p.likes || 0) + (p.comments || 0) + (p.shares || 0),
        }))
        .sort((a, b) => (b._engagement || 0) - (a._engagement || 0))
        .slice(0, 10) as PostSummary[];
    }
    if (provRes) {
      providerAnalytics = provRes.error ? null : provRes.data || null;
    } else {
      providerAnalytics = null;
    }
    loading = false;
  }

  $effect(() => {
    const controller = new AbortController();
    fetchData(controller.signal);
    return () => controller.abort();
  });

  // ── Realtime: refresh when posts are published/failed/deleted ──
  // Analytics are derived from post state, so any post state change
  // means the charts and top-posts list are stale. We re-fetch on
  // these events. The fetchData() call above is idempotent and
  // abort-safe, so duplicate triggers (e.g. rapid publishes) just
  // collapse into the latest fetch.
  let unsubscribers: (() => void)[] = [];

  onMount(() => {
    // U-7: mark analytics as visited so the Getting Started checklist
    // can check off "View your analytics".
    try { localStorage.setItem('social-forge-analytics-visited', 'true'); } catch { /* ignore */ }
    const events = ['post_published', 'post_failed', 'post_deleted', 'post_created'];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => fetchData()));
    }
  });

  onMount(async () => {
    const integRes = await integrationsApi.list();
    if (integRes.data) {
      connectedIntegrations = integRes.data.integrations;
    }
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
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
    <h1 class="text-xl font-bold text-content">Analytics</h1>
    <div class="flex gap-3 items-center">
      <select
        bind:value={selectedProvider}
        class="px-3 py-1.5 text-sm bg-surface border border-line rounded-lg text-content"
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
  {:else if data && data.total_posts === 0 && (!feedEngagement || feedEngagement.total_posts === 0)}
    <div class="text-center py-16">
      <p class="text-content-secondary mb-4">No analytics data yet. Import your feed or start posting!</p>
      <div class="flex gap-2 justify-center">
        <a href="/feed" class="inline-flex items-center gap-2 px-4 py-2 bg-surface-hover text-content rounded-lg hover:bg-line-hover transition-colors text-sm">
          Import Feed
        </a>
        <a href="/posts/new" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-500 transition-colors text-sm">
          Create Post
        </a>
      </div>
    </div>
  {:else if data}
    <!-- Feed Engagement Summary (from imported posts) -->
    {#if feedEngagement && feedEngagement.total_posts > 0}
      <div class="bg-surface border border-line rounded-xl p-4">
        <h3 class="text-sm font-semibold mb-3">Imported Post Engagement ({days}d)</h3>
        <div class="grid grid-cols-2 lg:grid-cols-5 gap-3">
          <div>
            <div class="text-xl font-bold text-indigo-400">{feedEngagement.total_posts}</div>
            <div class="text-xs text-muted">Imported Posts</div>
          </div>
          <div>
            <div class="text-xl font-bold text-green-400">{feedEngagement.total_likes?.toLocaleString() ?? 0}</div>
            <div class="text-xs text-muted">Total Likes</div>
          </div>
          <div>
            <div class="text-xl font-bold text-blue-400">{feedEngagement.total_comments?.toLocaleString() ?? 0}</div>
            <div class="text-xs text-muted">Total Comments</div>
          </div>
          <div>
            <div class="text-xl font-bold text-orange-400">{feedEngagement.total_shares?.toLocaleString() ?? 0}</div>
            <div class="text-xs text-muted">Total Shares</div>
          </div>
          <div>
            <div class="text-xl font-bold text-purple-400">{feedEngagement.total_impressions?.toLocaleString() ?? 0}</div>
            <div class="text-xs text-muted">Total Impressions</div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Summary Cards -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <div class="stat-card bg-surface border border-line rounded-xl p-4">
        <div class="text-2xl font-bold text-indigo-400">{data.total_posts}</div>
        <div class="text-xs text-muted mt-1 uppercase tracking-wider">Scheduled Posts</div>
      </div>
      <div class="stat-card bg-surface border border-line rounded-xl p-4">
        <div class="text-2xl font-bold text-green-400">{data.published}</div>
        <div class="text-xs text-muted mt-1 uppercase tracking-wider">Published</div>
      </div>
      <div class="stat-card bg-surface border border-line rounded-xl p-4">
        <div class="text-2xl font-bold text-red-400">{data.failed}</div>
        <div class="text-xs text-muted mt-1 uppercase tracking-wider">Failed</div>
      </div>
      <div class="stat-card bg-surface border border-line rounded-xl p-4">
        <div class="text-2xl font-bold text-amber-400">{data.best_provider?.count ?? 0}</div>
        <div class="text-xs text-muted mt-1 uppercase tracking-wider">Best: {data.best_provider?.provider ?? '—'}</div>
      </div>
    </div>

    <!-- Engagement Stats (if available) -->
    {#if totalEngagement.likes > 0 || totalEngagement.comments > 0 || totalEngagement.shares > 0}
      <div class="grid grid-cols-3 gap-4">
        <div class="stat-card bg-surface border border-line rounded-xl p-4 bg-pink-500/5">
          <div class="flex items-center gap-2">
            <Icon name="heart" class="w-4 h-4 text-pink-400" />
            <div>
              <div class="text-xl font-bold text-pink-400">{totalEngagement.likes.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Total Likes</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-surface border border-line rounded-xl p-4 bg-blue-500/5">
          <div class="flex items-center gap-2">
            <Icon name="comment-bubble" class="w-4 h-4 text-blue-400" />
            <div>
              <div class="text-xl font-bold text-blue-400">{totalEngagement.comments.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Total Comments</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-surface border border-line rounded-xl p-4 bg-green-500/5">
          <div class="flex items-center gap-2">
            <Icon name="share" class="w-4 h-4 text-green-400" />
            <div>
              <div class="text-xl font-bold text-green-400">{totalEngagement.shares.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Total Shares</div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Posts Over Time Chart -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <h3 class="text-sm font-medium text-content mb-4">Posts Over Time</h3>
      {#if data.posts_by_day.length === 0}
        <p class="text-content-secondary text-sm py-8 text-center">No data for this period</p>
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
              <span class="text-[10px] text-muted">{day.date.slice(5)}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Per-Provider Engagement Analytics (when specific provider selected) -->
    {#if providerAnalytics && providerAnalytics.data}
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">{providers.find(p => p.value === selectedProvider)?.label || selectedProvider} Engagement Metrics</h3>
        <div class="space-y-3">
          {#each providerAnalytics.data as metric}
            <div>
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs text-muted capitalize">{metric.label}</span>
                {#if metric.percentage_change !== 0}
                  <span class="text-xs {metric.percentage_change > 0 ? 'text-green-400' : 'text-red-400'}">
                    {metric.percentage_change > 0 ? '+' : ''}{metric.percentage_change.toFixed(1)}%
                  </span>
                {/if}
              </div>
              <div class="flex items-end gap-1 h-20">
                {#each metric.data.slice(-14) as point}
                  <div class="flex-1 flex flex-col items-center justify-end h-full">
                    <div
                      class="w-full bg-indigo-500/60 rounded-t hover:bg-indigo-400 transition-colors min-h-[2px]"
                      style="height: {Math.max((parseFloat(point.total) || 0) / Math.max(...metric.data.map(d => parseFloat(d.total) || 0), 1)) * 100}%"
                      title="{point.date}: {point.total}"
                    ></div>
                  </div>
                {/each}
              </div>
              <div class="text-xs text-muted-dark mt-1">
                Latest: {metric.data[metric.data.length - 1]?.total || '0'}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Two-column: Provider Performance + Top Posts -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- Provider Performance -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">Posts by Channel</h3>
        {#if filteredProviders.length === 0}
          <p class="text-sm text-muted py-4 text-center">No data</p>
        {:else}
          <div class="space-y-2">
            {#each filteredProviders as prov}
              <div class="flex items-center gap-3">
                <span class="text-xs text-muted w-24 truncate">{prov.provider}</span>
                <div class="flex-1 bg-background-input rounded-full h-6 overflow-hidden">
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
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">Top Posts by Engagement</h3>
        {#if topPosts.length === 0}
          <p class="text-sm text-muted py-4 text-center">No published posts with engagement data yet</p>
        {:else}
          <div class="space-y-2">
            {#each topPosts as post, i (post.id)}
              <div class="flex items-start gap-3 py-2 border-b border-line last:border-0">
                <span class="text-xs text-muted-dark w-4 mt-0.5">{i + 1}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-sm text-content-secondary truncate">{post.content || post.title || '(no content)'}</p>
                  <div class="flex gap-3 mt-1 text-[10px] text-muted">
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
