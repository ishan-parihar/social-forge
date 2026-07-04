<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { analyticsApi, type AnalyticsSummary } from '$lib/api/analytics';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { formatDateTime } from '$lib/calendar/utils';
  import { goto } from '$app/navigation';
  import { realtime } from '$lib/stores/realtime';
  import Icon from '$lib/ui/Icon.svelte';

  let upcoming = $state<PostSummary[]>([]);
  let recentPublished = $state<PostSummary[]>([]);
  let todayPosts = $state<PostSummary[]>([]);
  let stats = $state({ draft: 0, queued: 0, published: 0, error: 0 });
  let analyticsSummary = $state<AnalyticsSummary | null>(null);
  let integrations = $state<Integration[]>([]);
  let loading = $state(true);

  // Derived engagement totals from recent published posts
  let totalEngagement = $derived(
    recentPublished.reduce(
      (acc, p) => ({
        likes: acc.likes + (p.likes || 0),
        comments: acc.comments + (p.comments || 0),
        shares: acc.shares + (p.shares || 0),
      }),
      { likes: 0, comments: 0, shares: 0 }
    )
  );

  let alertCount = $derived(stats.error + upcoming.filter(p => p.state === 'error').length);

  async function load() {
    loading = true;
    const [postsRes, summaryRes, integRes] = await Promise.all([
      postsApi.list({ limit: 100 }),
      analyticsApi.getSummary(7),
      integrationsApi.list(),
    ]);

    if (postsRes.data) {
      const all = postsRes.data.posts;
      upcoming = all.filter(p => p.state === 'queued').slice(0, 5);
      recentPublished = all.filter(p => p.state === 'published').slice(0, 5);
      const t = new Date().toDateString();
      todayPosts = all.filter(p => p.scheduled_at && new Date(p.scheduled_at).toDateString() === t).slice(0, 5);
      stats = {
        draft: all.filter(p => p.state === 'draft').length,
        queued: all.filter(p => p.state === 'queued').length,
        published: all.filter(p => p.state === 'published').length,
        error: all.filter(p => p.state === 'error').length,
      };
    }

    if (summaryRes.data) {
      analyticsSummary = summaryRes.data;
    }

    if (integRes.data) {
      integrations = integRes.data.integrations.filter(i => !i.disabled);
    }

    loading = false;
  }

  let unsubscribers: (() => void)[] = [];

  onMount(() => {
    load();
    const events = ['post_created', 'post_scheduled', 'post_published', 'post_failed', 'post_deleted'];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  function providerIcon(p: string): string {
    const icons: Record<string, string> = {
      x: 'X', reddit: 'R', linkedin: 'in', facebook: 'f',
      instagram: 'IG', youtube: 'YT', bluesky: 'BS', mastodon: 'MA',
      pinterest: 'PIN', tiktok: 'TT', threads: 'TH', discord: 'DC',
      slack: 'SL', 'telegram-bot': 'TG', 'telegram-user': 'TG', whatsapp: 'WA',
    };
    return icons[p] || '•';
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

  // Max count for channel performance bars
  let maxProviderCount = $derived(
    analyticsSummary
      ? Math.max(...analyticsSummary.posts_by_provider.map(p => p.count), 1)
      : 1
  );
</script>

<div class="page-enter space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Dashboard</h2>
      <p class="text-sm text-muted mt-1">Your social media command center</p>
    </div>
    <div class="flex gap-2">
      <button onclick={() => goto('/posts/new')} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-medium transition-colors">
        + New Post
      </button>
      <button onclick={() => goto('/analytics')} class="px-4 py-2 bg-surface-hover hover:bg-line border border-line rounded-lg text-sm transition-colors">
        <Icon name="analytics" class="w-4 h-4 inline" /> Analytics
      </button>
    </div>
  </div>

  {#if loading}
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      {#each [1, 2, 3, 4] as _}
        <div class="skeleton h-24 rounded-xl"></div>
      {/each}
    </div>
  {:else}
    <!-- Post Stats Row -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      {#each [{ label: 'Drafts', value: stats.draft, color: 'text-blue-400', bg: 'bg-blue-500/10' }, { label: 'Queued', value: stats.queued, color: 'text-yellow-400', bg: 'bg-yellow-500/10' }, { label: 'Published', value: stats.published, color: 'text-green-400', bg: 'bg-green-500/10' }, { label: 'Errors', value: stats.error, color: 'text-red-400', bg: 'bg-red-500/10' }] as s}
        <div class="stat-card bg-surface border border-line rounded-xl p-5 {s.bg}">
          <div class="text-3xl font-bold {s.color}">{s.value}</div>
          <div class="text-xs text-muted mt-1 uppercase tracking-wider">{s.label}</div>
        </div>
      {/each}
    </div>

    <!-- Engagement Stats Row (only if there are published posts with engagement) -->
    {#if totalEngagement.likes > 0 || totalEngagement.comments > 0 || totalEngagement.shares > 0}
      <div class="grid grid-cols-3 gap-4">
        <div class="stat-card bg-surface border border-line rounded-xl p-4">
          <div class="flex items-center gap-2">
            <Icon name="heart" class="w-4 h-4 text-pink-400" />
            <div>
              <div class="text-xl font-bold text-pink-400">{totalEngagement.likes.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Likes (7d)</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-surface border border-line rounded-xl p-4">
          <div class="flex items-center gap-2">
            <Icon name="comment-bubble" class="w-4 h-4 text-blue-400" />
            <div>
              <div class="text-xl font-bold text-blue-400">{totalEngagement.comments.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Comments (7d)</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-surface border border-line rounded-xl p-4">
          <div class="flex items-center gap-2">
            <Icon name="share" class="w-4 h-4 text-green-400" />
            <div>
              <div class="text-xl font-bold text-green-400">{totalEngagement.shares.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Shares (7d)</div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Two-column: Channel Performance + Alerts -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- Channel Performance -->
      <div class="lg:col-span-2 bg-surface border border-line rounded-xl p-5">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-medium text-sm">Channel Performance (30d)</h3>
          <span class="text-xs text-muted">{integrations.length} connected</span>
        </div>
        {#if analyticsSummary && analyticsSummary.posts_by_provider.length > 0}
          <div class="space-y-2">
            {#each analyticsSummary.posts_by_provider.slice(0, 8) as prov}
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
        {:else}
          <p class="text-sm text-muted py-4 text-center">No posts published yet</p>
        {/if}
      </div>

      <!-- Alerts -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="font-medium text-sm mb-3">Alerts</h3>
        {#if alertCount === 0 && stats.draft === 0}
          <div class="text-center py-6">
            <span class="text-2xl">✅</span>
            <p class="text-xs text-muted mt-2">All clear — no issues detected</p>
          </div>
        {:else}
          <div class="space-y-2">
            {#if stats.error > 0}
              <a href="/posts" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-red-400"></span>
                <span class="text-xs text-red-400">{stats.error} failed post{stats.error > 1 ? 's' : ''}</span>
              </a>
            {/if}
            {#if stats.draft > 0}
              <a href="/posts" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-blue-400"></span>
                <span class="text-xs text-blue-400">{stats.draft} draft{stats.draft > 1 ? 's' : ''} waiting</span>
              </a>
            {/if}
            {#if upcoming.length > 0}
              <a href="/calendar" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-yellow-400"></span>
                <span class="text-xs text-yellow-400">{upcoming.length} scheduled post{upcoming.length > 1 ? 's' : ''} upcoming</span>
              </a>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Two-column: Today's Schedule + Recent Activity -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- Today's Schedule -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="font-medium text-sm mb-3">Today's Schedule</h3>
        {#if todayPosts.length === 0}
          <p class="text-sm text-muted py-4 text-center">No posts scheduled for today</p>
        {:else}
          <div class="space-y-1">
            {#each todayPosts as post}
              <div class="flex items-center gap-3 py-2 border-b border-line last:border-0">
                <span class="text-xs text-muted w-12 font-mono">{post.scheduled_at ? formatDateTime(post.scheduled_at).slice(-5) : ''}</span>
                <span class="flex-1 text-sm truncate text-content-secondary">{post.content || post.title || '(no content)'}</span>
                <span class="text-xs px-2 py-0.5 rounded badge-{post.state}">{post.integration_name}</span>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Recent Activity -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="font-medium text-sm mb-3">Recent Activity</h3>
        {#if recentPublished.length === 0}
          <p class="text-sm text-muted py-4 text-center">No posts published yet</p>
        {:else}
          <div class="space-y-1">
            {#each recentPublished as post}
              <div class="flex items-center gap-3 py-2 border-b border-line last:border-0">
                <span class="text-xs {providerColor(post.integration_name?.toLowerCase() || '')}">{providerIcon(post.integration_name?.toLowerCase() || '')}</span>
                <span class="flex-1 text-sm truncate text-content-secondary">{post.content || post.title || '(no content)'}</span>
                <div class="flex gap-2 text-[10px] text-muted">
                  {#if post.likes != null && post.likes > 0}
                    <span class="flex items-center gap-0.5"><Icon name="heart" class="w-3 h-3 text-pink-400" /> {post.likes}</span>
                  {/if}
                  {#if post.comments != null && post.comments > 0}
                    <span class="flex items-center gap-0.5"><Icon name="comment-bubble" class="w-3 h-3 text-blue-400" /> {post.comments}</span>
                  {/if}
                  {#if post.shares != null && post.shares > 0}
                    <span class="flex items-center gap-0.5"><Icon name="share" class="w-3 h-3 text-green-400" /> {post.shares}</span>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Quick Actions -->
    <div class="flex gap-3 flex-wrap">
      <button onclick={() => goto('/posts/new')} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-medium transition-colors">
        <Icon name="post" class="w-4 h-4 inline" /> Compose Post
      </button>
      <button onclick={() => goto('/feed')} class="px-4 py-2 bg-surface-hover hover:bg-line border border-line rounded-lg text-sm transition-colors">
        <Icon name="feed" class="w-4 h-4 inline" /> View Feed
      </button>
      <button onclick={() => goto('/channels')} class="px-4 py-2 bg-surface-hover hover:bg-line border border-line rounded-lg text-sm transition-colors">
        <Icon name="channel" class="w-4 h-4 inline" /> Manage Channels
      </button>
      <button onclick={() => goto('/analytics')} class="px-4 py-2 bg-surface-hover hover:bg-line border border-line rounded-lg text-sm transition-colors">
        <Icon name="analytics" class="w-4 h-4 inline" /> View Analytics
      </button>
    </div>
  {/if}
</div>
