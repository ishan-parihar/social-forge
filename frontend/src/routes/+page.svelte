<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { analyticsApi, type AnalyticsSummary } from '$lib/api/analytics';
  import { feedApi } from '$lib/api/feed';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { auth } from '$lib/api/auth';
  import { formatDateTime } from '$lib/calendar/utils';
  import { goto } from '$app/navigation';
  import { realtime } from '$lib/stores/realtime';
  import { timezone } from '$lib/stores/timezone.svelte';
  import { providerIcon, providerColor } from '$lib/providers';
  import Icon from '$lib/ui/Icon.svelte';
  import StatCard from '$lib/ui/StatCard.svelte';
  import GettingStarted from '$lib/onboarding/GettingStarted.svelte';
  import { composer } from '$lib/stores/composer.svelte';

  let upcoming = $state<PostSummary[]>([]);
  let recentPublished = $state<PostSummary[]>([]);
  let todayPosts = $state<PostSummary[]>([]);
  let allTodayPosts = $state<PostSummary[]>([]);
  // Drafts the user is composing — surfaced in the "Needs Attention"
  // inbox (U-2) so they don't get forgotten.
  let draftPosts = $state<PostSummary[]>([]);
  let stats = $state({ draft: 0, queued: 0, published: 0, error: 0 });
  let analyticsSummary = $state<AnalyticsSummary | null>(null);
  let feedEngagement = $state<{ total_likes: number; total_comments: number; total_shares: number } | null>(null);
  let integrations = $state<Integration[]>([]);
  let loading = $state(true);

  // Engagement totals from real feed analytics data (7d)
  let totalEngagement = $derived({
    likes: feedEngagement?.total_likes ?? 0,
    comments: feedEngagement?.total_comments ?? 0,
    shares: feedEngagement?.total_shares ?? 0,
  });

  let alertCount = $derived(stats.error);

  async function load() {
    loading = true;
    const [postsRes, summaryRes, integRes, feedRes] = await Promise.all([
      postsApi.list({ limit: 100 }),
      analyticsApi.getSummary(7),
      integrationsApi.list(),
      feedApi.analytics(7),
    ]);

    if (feedRes.data) {
      feedEngagement = feedRes.data;
    }

    if (postsRes.data) {
      const all = postsRes.data.posts;
      upcoming = all.filter(p => p.state === 'queued').slice(0, 5);
      recentPublished = all.filter(p => p.state === 'published').slice(0, 5);
      draftPosts = all.filter(p => p.state === 'draft');
      // v22 Phase 5: timezone-aware "today's schedule" filter. Previously
      // used `new Date().toDateString()` which compares in browser local
      // time, not the user's selected timezone. Now we format both the
      // post's scheduled_at and "now" in the user's selected timezone
      // before comparing date strings.
      const todayStr = formatTodayInTimezone();
      allTodayPosts = all.filter(p => {
        if (!p.scheduled_at) return false;
        return formatPostDateInTimezone(p.scheduled_at) === todayStr;
      });
      todayPosts = allTodayPosts.slice(0, 5);
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

  // v22 Phase 5: helper to format "today" in the user's selected timezone.
  // Falls back to browser local time if the timezone store isn't ready.
  function formatTodayInTimezone(): string {
    try {
      const tz = timezone.value;
      return new Date().toLocaleDateString('en-CA', { timeZone: tz }); // en-CA = YYYY-MM-DD
    } catch {
      return new Date().toDateString();
    }
  }

  function formatPostDateInTimezone(iso: string): string {
    try {
      const tz = timezone.value;
      return new Date(iso).toLocaleDateString('en-CA', { timeZone: tz });
    } catch {
      return new Date(iso).toDateString();
    }
  }

  let unsubscribers: (() => void)[] = [];

  onMount(() => {
    load();
    const events = ['post_created', 'post_scheduled', 'post_published', 'post_failed', 'post_deleted', 'post_stage_changed', 'lagged'];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  // v22 Phase 5: fixed providerIcon/providerColor bug. Previously these
  // were called with `post.integration_name` (e.g. "My X Account") which
  // returns the fallback for every post. Now we use `integration_id`-based
  // lookup if available, or fall back to the integration_name lowercased.
  // The providerMeta map keys on provider_identifier (e.g. "x"), not the
  // user-set account name.
  function providerIdFor(post: PostSummary): string {
    // PostSummary doesn't carry provider_identifier directly, but
    // integration_name often starts with the provider. As a best-effort,
    // lowercase + strip non-alphanumerics. If the integration list has
    // a match, use its provider_identifier.
    const match = integrations.find(i => i.id === post.integration_id);
    return match?.provider_identifier || post.integration_name?.toLowerCase()?.split(/\s+/)[0] || '';
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
      <button onclick={() => composer.openCreate()} class="px-4 py-2 bg-brand-500 hover:bg-brand-600 rounded-lg text-sm font-medium transition-colors">
        + New Post
      </button>
      <button onclick={() => goto('/analytics')} class="px-4 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm transition-colors">
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
    <!-- Getting Started checklist (U-7): persistent widget that auto-tracks
         onboarding progress. Dismissable; auto-hides when all items complete. -->
    <GettingStarted />

    <!-- Post Stats Row — v22 Phase 5: uses StatCard primitive + semantic tokens -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <StatCard label="Drafts" value={stats.draft} color="info" />
      <StatCard label="Queued" value={stats.queued} color="warning" />
      <StatCard label="Published" value={stats.published} color="success" />
      <StatCard label="Errors" value={stats.error} color="error" />
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
            <Icon name="comment-bubble" class="w-4 h-4 text-info" />
            <div>
              <div class="text-xl font-bold text-info">{totalEngagement.comments.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Comments (7d)</div>
            </div>
          </div>
        </div>
        <div class="stat-card bg-surface border border-line rounded-xl p-4">
          <div class="flex items-center gap-2">
            <Icon name="share" class="w-4 h-4 text-success" />
            <div>
              <div class="text-xl font-bold text-success">{totalEngagement.shares.toLocaleString()}</div>
              <div class="text-[10px] text-muted uppercase tracking-wider">Shares (7d)</div>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <!-- Two-column: Channel Performance + Alerts -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- Channel Performance — v22 Phase 5: uses provider brand colors -->
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
                    class="h-full rounded-full transition-all duration-500 flex items-center justify-end px-2"
                    style="width: {(prov.count / maxProviderCount) * 100}%; background-color: {providerColor(prov.provider)};"
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
              <a href="/posts?state=error" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-error"></span>
                <span class="text-xs text-error">{stats.error} failed post{stats.error > 1 ? 's' : ''}</span>
              </a>
            {/if}
            {#if stats.draft > 0}
              <a href="/posts?state=draft" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-info"></span>
                <span class="text-xs text-info">{stats.draft} draft{stats.draft > 1 ? 's' : ''} waiting</span>
              </a>
            {/if}
            {#if upcoming.length > 0}
              <a href="/calendar" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-warning"></span>
                <span class="text-xs text-warning">{upcoming.length} scheduled post{upcoming.length > 1 ? 's' : ''} upcoming</span>
              </a>
            {/if}
            {#if integrations.filter(i => i.refresh_needed).length > 0}
              <a href="/channels" class="flex items-center gap-2 p-2 rounded-lg hover:bg-surface-hover transition-colors">
                <span class="w-2 h-2 rounded-full bg-orange-400"></span>
                <span class="text-xs text-orange-400">{integrations.filter(i => i.refresh_needed).length} channel{integrations.filter(i => i.refresh_needed).length > 1 ? 's' : ''} need{integrations.filter(i => i.refresh_needed).length === 1 ? 's' : ''} reconnect</span>
              </a>
            {/if}
          </div>
        {/if}
      </div>
    </div>

    <!-- Needs Attention inbox (R-1 + U-2): aggregates failed posts,
         drafts waiting to be scheduled, and channels needing reconnect. -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <div class="flex items-center justify-between mb-3">
        <h3 class="font-medium text-sm">Needs Attention</h3>
        <span class="text-xs text-muted">{draftPosts.length + stats.error + integrations.filter(i => i.refresh_needed).length} item{(draftPosts.length + stats.error + integrations.filter(i => i.refresh_needed).length) !== 1 ? 's' : ''}</span>
      </div>
      {#if draftPosts.length === 0 && stats.error === 0 && integrations.filter(i => i.refresh_needed).length === 0}
        <div class="text-center py-4">
          <span class="text-xl">✅</span>
          <p class="text-xs text-muted mt-1">All clear — nothing needs your attention</p>
        </div>
      {:else}
        <div class="space-y-1.5">
          {#each draftPosts.slice(0, 5) as post (post.id)}
            <a href="/posts/{post.id}" class="flex items-center gap-2 py-1.5 px-2 -mx-2 rounded-lg hover:bg-surface-hover transition-colors group">
              <span class="w-1.5 h-1.5 rounded-full bg-info shrink-0"></span>
              <span class="flex-1 text-xs truncate text-content-secondary group-hover:text-content">{post.content || post.title || '(no content)'}</span>
              <span class="text-[10px] text-muted-dark">draft</span>
            </a>
          {/each}
          {#if stats.error > 0}
            <a href="/posts?state=error" class="flex items-center gap-2 py-1.5 px-2 -mx-2 rounded-lg hover:bg-surface-hover transition-colors group">
              <span class="w-1.5 h-1.5 rounded-full bg-error shrink-0"></span>
              <span class="flex-1 text-xs text-error">{stats.error} failed post{stats.error > 1 ? 's' : ''} need{stats.error === 1 ? 's' : ''} retry</span>
              <span class="text-[10px] text-muted-dark">→</span>
            </a>
          {/if}
          {#each integrations.filter(i => i.refresh_needed).slice(0, 3) as int (int.id)}
            <a href="/channels" class="flex items-center gap-2 py-1.5 px-2 -mx-2 rounded-lg hover:bg-surface-hover transition-colors group">
              <span class="w-1.5 h-1.5 rounded-full bg-orange-400 shrink-0"></span>
              <span class="flex-1 text-xs text-orange-400 truncate">{int.provider_name} token expiring</span>
              <span class="text-[10px] text-muted-dark">→</span>
            </a>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Two-column: Today's Schedule + Recent Activity -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <!-- Today's Schedule -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="font-medium text-sm mb-3">Today's Schedule <span class="text-[10px] text-muted-dark">({timezone.value})</span></h3>
        {#if todayPosts.length === 0}
          <p class="text-sm text-muted py-4 text-center">No posts scheduled for today</p>
        {:else}
          <div class="space-y-1">
            {#each todayPosts as post}
              <div class="flex items-center gap-3 py-2 border-b border-line last:border-0">
                <span class="text-xs text-muted w-12 font-mono">{post.scheduled_at ? formatDateTime(post.scheduled_at).slice(-5) : ''}</span>
                <span class="flex-1 text-sm truncate text-content-secondary">{post.content || post.title || '(no content)'}</span>
                <span class="text-xs text-muted">{post.integration_name}</span>
              </div>
            {/each}
          </div>
          {#if allTodayPosts.length > 5}
            <a href="/calendar" class="block text-center text-xs text-brand-400 hover:underline mt-2">
              View all ({allTodayPosts.length})
            </a>
          {/if}
        {/if}
      </div>

      <!-- Recent Activity — v22 Phase 5: fixed providerIcon/providerColor bug -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="font-medium text-sm mb-3">Recent Activity</h3>
        {#if recentPublished.length === 0}
          <p class="text-sm text-muted py-4 text-center">No posts published yet</p>
        {:else}
          <div class="space-y-1">
            {#each recentPublished as post}
              <div class="flex items-center gap-3 py-2 border-b border-line last:border-0">
                <span class="text-xs" style="color: {providerColor(providerIdFor(post))}">{providerIcon(providerIdFor(post))}</span>
                <span class="flex-1 text-sm truncate text-content-secondary">{post.content || post.title || '(no content)'}</span>
                <div class="flex gap-2 text-[10px] text-muted">
                  {#if post.likes != null && post.likes > 0}
                    <span class="flex items-center gap-0.5"><Icon name="heart" class="w-3 h-3 text-pink-400" /> {post.likes}</span>
                  {/if}
                  {#if post.comments != null && post.comments > 0}
                    <span class="flex items-center gap-0.5"><Icon name="comment-bubble" class="w-3 h-3 text-info" /> {post.comments}</span>
                  {/if}
                  {#if post.shares != null && post.shares > 0}
                    <span class="flex items-center gap-0.5"><Icon name="share" class="w-3 h-3 text-success" /> {post.shares}</span>
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
      <button onclick={() => composer.openCreate()} class="px-4 py-2 bg-brand-500 hover:bg-brand-600 rounded-lg text-sm font-medium transition-colors">
        <Icon name="post" class="w-4 h-4 inline" /> Compose Post
      </button>
      <button onclick={() => goto('/feed')} class="px-4 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm transition-colors">
        <Icon name="feed" class="w-4 h-4 inline" /> View Feed
      </button>
      <button onclick={() => goto('/channels')} class="px-4 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm transition-colors">
        <Icon name="channel" class="w-4 h-4 inline" /> Manage Channels
      </button>
      <button onclick={() => goto('/analytics')} class="px-4 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm transition-colors">
        <Icon name="analytics" class="w-4 h-4 inline" /> View Analytics
      </button>
    </div>
  {/if}
</div>
