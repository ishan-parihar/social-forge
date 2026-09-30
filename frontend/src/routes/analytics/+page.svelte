<script lang="ts">
  // v25 F4 — analytics, rebuilt engagement-first.
  //
  // The v22 version answered "how many posts" and buried the numbers that
  // actually matter (likes, comments, shares, impressions) in a conditional
  // block that only appeared if one of them was non-zero. Plan §4 F4 calls for
  // engagement totals, a rate, a per-channel breakdown, cadence against a goal,
  // what is scheduled today, and recent activity.
  //
  // All six read from endpoints that ALREADY EXIST (B2 added them; the root
  // dashboard consumes engagement/adherence/cadence/events today). This phase
  // is wiring and hierarchy, not new backend surface. Nothing here polls:
  // fetchData is abort-safe and idempotent, and re-runs only on a range or
  // provider change, or on an SSE post event.
  import Icon from "$lib/ui/Icon.svelte";
  import Skeleton from '$lib/ui/Skeleton.svelte';
  import EmptyState from '$lib/ui/EmptyState.svelte';
  import Sparkline from "$lib/ui/Sparkline.svelte";
  import {
    analyticsApi,
    type AnalyticsSummary,
    type EngagementResponse,
    type AdherenceResponse,
    type CadenceResponse,
    type EventLogEntry,
  } from '$lib/api/analytics';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { feedApi } from '$lib/api/feed';
  import { providerLabel } from '$lib/providers';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import DateRangePicker from '$lib/analytics/DateRangePicker.svelte';
  import { timezone } from '$lib/stores/timezone.svelte';
  import { realtime } from "$lib/stores/realtime";
  import { onMount, onDestroy } from "svelte";

  let days = $state(30);
  let data = $state<AnalyticsSummary | null>(null);
  let engagement = $state<EngagementResponse | null>(null);
  let adherence = $state<AdherenceResponse | null>(null);
  let cadence = $state<CadenceResponse | null>(null);
  let recentEvents = $state<EventLogEntry[]>([]);
  let feedEngagement = $state<{ total_likes: number; total_comments: number; total_shares: number; total_views: number; posts_with_engagement: number } | null>(null);
  let allPosts = $state<PostSummary[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedProvider = $state<string>("all");
  let topPosts = $state<PostSummary[]>([]);
  let providerAnalytics = $state<import("$lib/api/analytics").ProviderAnalytics | null>(null);
  let connectedIntegrations = $state<Integration[]>([]);

  // Build provider list dynamically from connected integrations
  let providers = $derived.by(() => {
    const unique = [...new Set(
      connectedIntegrations.filter(i => !i.disabled).map(i => i.provider_identifier)
    )];
    // Phase v21: use the central providerLabel() from $lib/providers
    // instead of a duplicated 26-entry labels map.
    return [
      { value: "all", label: "All Platforms" },
      ...unique.map(p => ({ value: p, label: providerLabel(p) })),
    ];
  });

  async function fetchData(signal?: AbortSignal) {
    loading = true;
    error = null;
    // v25 F4: one parallel round-trip for the whole dashboard. The previous
    // version made a second round-trip on its own for the provider detail.
    const [summaryRes, engagementRes, adherenceRes, cadenceRes, eventsRes, feedRes, postsRes, provRes] =
      await Promise.all([
        analyticsApi.getSummary(days, signal),
        analyticsApi.getEngagement(days, signal),
        analyticsApi.getAdherence(days, signal),
        analyticsApi.getCadence(days, signal),
        analyticsApi.getRecentEvents(8, signal),
        feedApi.analytics(days),
        postsApi.list({ limit: 100 }),
        selectedProvider === "all"
          ? Promise.resolve({ data: null, error: undefined })
          : analyticsApi.getProvider(selectedProvider, days, signal),
      ]);
    if (signal?.aborted) return;

    if (summaryRes.error) {
      error = summaryRes.error;
      data = null;
    } else if (summaryRes.data) {
      data = summaryRes.data;
    }
    if (engagementRes.data) engagement = engagementRes.data;
    if (adherenceRes.data) adherence = adherenceRes.data;
    if (cadenceRes.data) cadence = cadenceRes.data;
    if (eventsRes.data) recentEvents = eventsRes.data;
    if (feedRes.data) feedEngagement = feedRes.data;

    if (postsRes.data) {
      allPosts = postsRes.data.posts;
      const published = allPosts.filter(p => p.state === "published");
      const pool = selectedProvider === "all"
        ? published
        : published.filter(p => p.integration_name?.toLowerCase().includes(selectedProvider));
      topPosts = pool
        .map(p => ({ ...p, _engagement: (p.likes || 0) + (p.comments || 0) + (p.shares || 0) }))
        .sort((a, b) => (b._engagement || 0) - (a._engagement || 0))
        .slice(0, 8) as PostSummary[];
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

  // ── Realtime ──
  // Analytics are derived from post state, so any post state change means the
  // numbers are stale. fetchData() is idempotent and abort-safe, so rapid
  // publishes collapse into the latest fetch rather than stacking requests.
  let unsubscribers: (() => void)[] = [];

  onMount(() => {
    // U-7: mark analytics as visited so the Getting Started checklist can
    // check off "View your analytics".
    try { localStorage.setItem('social-forge-analytics-visited', 'true'); } catch { /* ignore */ }
    for (const evt of ['post_published', 'post_failed', 'post_deleted', 'post_created', 'post_scheduled', 'lagged']) {
      unsubscribers.push(realtime.on(evt, () => fetchData()));
    }
  });

  onMount(async () => {
    const integRes = await integrationsApi.list();
    if (integRes.data) connectedIntegrations = integRes.data.integrations;
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  // ── Derived: engagement totals ──
  //
  // Two sources contribute and they are ADDED, not preferred: `engagement`
  // counts analytics_cache rows (posts Forge published and then polled),
  // `feedEngagement` counts imported external posts. Before F4 these were two
  // disconnected cards, so a user with both kinds of activity saw a total that
  // was wrong in both directions depending on which block they read.
  let totals = $derived.by(() => {
    const api = engagement;
    const feed = feedEngagement;
    const byDay = api?.by_day?.length ? api.by_day : [];
    return {
      impressions: (api?.total_impressions ?? 0) + (feed?.total_views ?? 0),
      likes: (api?.total_likes ?? 0) + (feed?.total_likes ?? 0),
      comments: (api?.total_comments ?? 0) + (feed?.total_comments ?? 0),
      shares: (api?.total_shares ?? 0) + (feed?.total_shares ?? 0),
      deltas: {
        impressions: api?.impressions_delta ?? 0,
        likes: api?.likes_delta ?? 0,
        comments: api?.comments_delta ?? 0,
        shares: api?.shares_delta ?? 0,
      },
      series: {
        impressions: byDay.map(d => d.impressions),
        likes: byDay.map(d => d.likes),
        comments: byDay.map(d => d.comments),
        shares: byDay.map(d => d.shares),
      },
    };
  });

  // ── Derived: engagement rate ──
  //
  // The single number the v22 page never showed: interactions earned per post
  // published. Without it, 500 likes across 2 posts and 500 likes across 100
  // posts look identical.
  //
  // The denominator MUST be windowed to the same `days` as the numerator.
  // `allPosts` is the newest 100 posts regardless of age, so dividing an
  // N-day engagement total by an all-time count produces a rate wrong by
  // exactly the ratio of the two windows, and silently so, because both
  // numbers look plausible on their own. Guarded against divide-by-zero, and
  // reported as "Not yet" (not 0) when nothing has published, because "no
  // posts" is not a 0% rate.
  let publishedInWindow = $derived.by(() => {
    const cutoff = Date.now() - days * 86_400_000;
    return allPosts.filter(p =>
      p.state === "published" &&
      p.published_at &&
      Date.parse(p.published_at) >= cutoff
    );
  });
  let publishedCount = $derived(publishedInWindow.length);
  let engagementRate = $derived.by(() => {
    if (publishedCount === 0) return null;
    const interactions = totals.likes + totals.comments + totals.shares;
    return interactions / publishedCount;
  });

  // ── Derived: cadence vs goal ──
  let goalPerDay = $derived(cadence?.goal_per_day ?? null);
  let actualPerDay = $derived(cadence?.actual_per_day ?? 0);
  // Uncapped ratio, for the sentence under the meter. `goalProgressPct` is the
  // capped version that drives the bar, so a 400%-of-goal window reads as
  // "goal met" rather than overflowing the track.
  let goalRatioPct = $derived(
    goalPerDay && goalPerDay > 0 ? (actualPerDay / goalPerDay) * 100 : null
  );
  let goalProgressPct = $derived(goalRatioPct === null ? null : Math.min(100, goalRatioPct));
  let cadenceSeries = $derived(cadence?.by_day?.map(d => d.count) ?? []);

  // ── Derived: scheduled today (timezone-aware) ──
  // Compares date strings formatted in the user's selected timezone, not the
  // browser's, so "today" matches the calendar the rest of the app shows.
  function tzDate(iso: string): string {
    try {
      return new Date(iso).toLocaleDateString('en-CA', { timeZone: timezone.value });
    } catch {
      return new Date(iso).toDateString();
    }
  }
  let todayStr = $derived.by(() => {
    try {
      return new Date().toLocaleDateString('en-CA', { timeZone: timezone.value });
    } catch {
      return new Date().toDateString();
    }
  });
  let todayPosts = $derived(
    allPosts
      .filter(p => p.scheduled_at && tzDate(p.scheduled_at) === todayStr)
      .sort((a, b) => (a.scheduled_at ?? "").localeCompare(b.scheduled_at ?? ""))
  );

  // ── Derived: channel mix ──
  let maxCount = $derived(data ? Math.max(...data.posts_by_day.map(d => d.count), 1) : 1);
  let maxProviderCount = $derived(data ? Math.max(...data.posts_by_provider.map(p => p.count), 1) : 1);
  let filteredProviders = $derived(
    data
      ? (selectedProvider === "all"
        ? data.posts_by_provider
        : data.posts_by_provider.filter(p => p.provider === selectedProvider))
      : []
  );

  /**
   * Render an events_log row as a sentence.
   *
   * events_log stores the same event names the SSE broadcaster emits, but the
   * payload shape differs per event and will keep changing. So this maps the
   * names it knows, reads only fields it recognises, and falls back to a
   * readable de-snake_cased label for anything new rather than rendering an
   * empty row. A wrong-shaped payload degrades to less detail; it never invents
   * a value.
   */
  const EVENT_SENTENCES: Record<string, (p: Record<string, unknown>) => string> = {
    post_published: (p) => `Published to ${labelOf(p.provider)}`,
    post_failed: (p) => `Failed on ${labelOf(p.provider)}`,
    post_scheduled: (p) => `Scheduled for ${labelOf(p.provider)}`,
    post_created: (p) => `Draft created${p.provider ? ` for ${labelOf(p.provider)}` : ""}`,
    post_deleted: () => "Post deleted",
    post_stage_changed: () => "Post moved back to drafts",
    integration_connected: (p) => `${labelOf(p.provider ?? p.name)} connected`,
    integration_disconnected: (p) => `${labelOf(p.provider ?? p.name)} disconnected`,
  };
  function labelOf(v: unknown): string {
    if (typeof v === "string" && v.trim()) return providerLabel(v.trim());
    return "a channel";
  }
  function eventSentence(type: string, payload: Record<string, unknown>): string {
    const fn = EVENT_SENTENCES[type];
    if (fn) return fn(payload ?? {});
    return type.replace(/_/g, " ").replace(/^\w/, c => c.toUpperCase());
  }

  // ── Empty state ──
  //
  // v25 F4: the old condition was `total_posts === 0 && no feed engagement`,
  // which blanked the whole page even when the cadence and event log had real
  // content. Now ANY populated source keeps the page alive, and each widget
  // renders its own "nothing yet" line.
  let hasAnything = $derived(
    (data?.total_posts ?? 0) > 0 ||
    (feedEngagement?.posts_with_engagement ?? 0) > 0 ||
    (totals.impressions + totals.likes + totals.comments + totals.shares) > 0 ||
    (cadence?.total_posts ?? 0) > 0 ||
    recentEvents.length > 0
  );

  const STATE_TONE: Record<string, string> = {
    published: "badge-published",
    queued: "badge-queued",
    draft: "badge-draft",
    error: "badge-error",
    failed: "badge-error",
  };
  const STATE_LABEL: Record<string, string> = {
    published: "Published",
    queued: "Scheduled",
    draft: "Draft",
    error: "Failed",
    failed: "Failed",
  };
</script>

<div class="page-enter space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between gap-3 flex-wrap">
    <div>
      <h1 class="text-xl font-bold text-content">Analytics</h1>
      <p class="text-sm text-muted mt-1">What your posts earned, over the last {days} days.</p>
    </div>
    <div class="flex gap-3 items-center">
      <label for="provider-filter" class="sr-only">Filter by platform</label>
      <select
        id="provider-filter"
        bind:value={selectedProvider}
        class="px-3 py-1.5 text-sm bg-surface border border-line rounded-lg text-content"
      >
        {#each providers as p}
          <option value={p.value}>{p.label}</option>
        {/each}
      </select>
      <DateRangePicker selected={days} onChange={(d) => { days = d; }} />
    </div>
  </div>

  {#if error}
    <div class="bg-error/10 border border-error/30 rounded-lg p-4" role="alert">
      <p class="text-error text-sm">{error}</p>
      <button onclick={() => fetchData()} class="mt-2 text-sm text-accent hover:text-accent-strong">Retry</button>
    </div>
  {:else if loading && !hasAnything}
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      {#each [1, 2, 3, 4] as i (i)}
        <Skeleton height="6rem" rounded="lg" />
      {/each}
    </div>
    <Skeleton height="12rem" rounded="lg" />
  {:else if !hasAnything}
    <EmptyState
      icon="analytics"
      title="No analytics yet"
      description="Publish a post, or import your existing feed to start measuring engagement."
    />
    <div class="flex gap-2 justify-center">
      <a href="/feed" class="px-4 py-2 bg-surface-hover text-content rounded-lg hover:bg-surface-hover border border-line transition-colors text-sm">
        Import feed
      </a>
      <a href="/posts/new" class="px-4 py-2 bg-accent-fill text-accent-fg rounded-lg hover:bg-accent-fill-hover transition-colors text-sm">
        Create post
      </a>
    </div>
  {:else}
    <!-- 1. Engagement totals (the headline) -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
      <!-- `as const` keeps the literal key types so `totals[metric.key]` stays
           a number lookup instead of widening the key to string. -->
      {#each [
        { key: 'impressions', label: 'Impressions', icon: 'eye', tone: 'text-viz-impression' },
        { key: 'likes', label: 'Likes', icon: 'heart', tone: 'text-viz-like' },
        { key: 'comments', label: 'Comments', icon: 'comment-bubble', tone: 'text-viz-comment' },
        { key: 'shares', label: 'Shares', icon: 'share', tone: 'text-viz-share' },
      ] as const as metric (metric.key)}
        <div class="stat-card bg-surface border border-line rounded-xl p-4">
          <div class="flex items-center gap-1.5 text-xs text-muted mb-2">
            <Icon name={metric.icon} class="w-3.5 h-3.5 {metric.tone}" />
            {metric.label}
          </div>
          <div class="flex items-baseline gap-2">
            <span class="text-2xl font-bold text-content">{totals[metric.key].toLocaleString()}</span>
            {#if totals.deltas[metric.key] !== 0}
              <span class="text-xs {totals.deltas[metric.key] > 0 ? 'text-success' : 'text-error'}">
                {totals.deltas[metric.key] > 0 ? '+' : ''}{totals.deltas[metric.key]}%
              </span>
            {/if}
          </div>
          {#if totals.series[metric.key].length > 1}
            <div class="mt-2 {metric.tone}">
              <Sparkline
                data={totals.series[metric.key]}
                height={22}
                class="w-full"
                ariaLabel="{metric.label} over the last {days} days"
              />
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- 2. Rate + posting health -->
    <div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
      <!-- Cadence against goal -->
      <div class="bg-surface border border-line rounded-xl p-5 lg:col-span-2">
        <div class="flex items-center justify-between mb-1">
          <h3 class="text-sm font-medium text-content">Cadence against goal</h3>
          {#if cadence && cadence.streak_days > 0}
            <span class="text-xs text-success">{cadence.streak_days}-day streak</span>
          {/if}
        </div>

        {#if !cadence || cadence.total_posts === 0}
          <p class="text-sm text-muted py-6 text-center">No posts in this window yet.</p>
        {:else}
          <div class="flex items-baseline gap-2 mb-3">
            <span class="text-2xl font-bold text-content">{actualPerDay.toFixed(1)}</span>
            <span class="text-sm text-muted">posts per day</span>
            {#if goalPerDay}
              <span class="text-sm text-muted ml-auto">goal {goalPerDay.toFixed(1)}/day</span>
            {/if}
          </div>

          {#if goalProgressPct !== null}
            <!--
              A goal meter is a ratio, so a track is the right component here
              (the "no filled progress bars" rule targets marketing pages, not a
              dashboard metric). Track uses --line, fill uses --brand: both
              tokens, so it rethemes.
            -->
            <div
              class="h-2 rounded-full bg-line overflow-hidden"
              role="progressbar"
              aria-valuenow={Math.round(goalProgressPct)}
              aria-valuemin="0"
              aria-valuemax="100"
              aria-label="Progress toward the daily posting goal"
            >
              <div
                class="h-full rounded-full transition-all duration-500 {goalProgressPct >= 100 ? 'bg-success' : 'bg-accent-fill'}"
                style="width: {goalProgressPct}%"
              ></div>
            </div>
            <p class="text-xs text-muted mt-2">
              {#if goalRatioPct !== null && goalRatioPct >= 100}
                Goal met. Average is {Math.round(goalRatioPct)}% of target.
              {:else if goalRatioPct !== null}
                {Math.round(goalRatioPct)}% of the daily goal.
              {/if}
            </p>
          {:else}
            <p class="text-xs text-muted">
              No daily goal set. Average {actualPerDay.toFixed(1)} posts per day across this window.
            </p>
          {/if}

          {#if cadenceSeries.length > 1}
            <div class="mt-4 text-accent">
              <Sparkline
                data={cadenceSeries}
                height={30}
                class="w-full"
                ariaLabel="Posts per day over the last {days} days"
              />
            </div>
          {/if}
        {/if}
      </div>

      <!-- Engagement rate + adherence -->
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">Efficiency</h3>
        <dl class="space-y-4">
          <div>
            <dt class="text-xs text-muted">Interactions per post</dt>
            <dd class="text-2xl font-bold text-content mt-0.5">
              {engagementRate === null ? 'Not yet' : engagementRate.toFixed(1)}
            </dd>
            <dd class="text-xs text-faint mt-0.5">
              {#if engagementRate === null}
                Nothing published in this window
              {:else}
                across {publishedCount} published {publishedCount === 1 ? 'post' : 'posts'}
              {/if}
            </dd>
          </div>

          <div class="pt-3 border-t border-line">
            <dt class="text-xs text-muted">Publish success rate</dt>
            <dd class="text-2xl font-bold text-content mt-0.5">
              {adherence && adherence.scheduled > 0
                ? `${Math.round(adherence.adherence_rate)}%`
                : 'Not yet'}
            </dd>
            <dd class="text-xs text-faint mt-0.5">
              {#if adherence && adherence.scheduled > 0}
                {adherence.published} of {adherence.scheduled} scheduled posts went out
                {#if adherence.failed > 0}
                  <span class="text-error">({adherence.failed} failed)</span>
                {/if}
              {:else}
                No scheduled posts in this window
              {/if}
            </dd>
          </div>
        </dl>
      </div>
    </div>

    <!-- 3. Scheduled today + recent activity -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <div class="bg-surface border border-line rounded-xl p-5">
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-sm font-medium text-content">Scheduled today</h3>
          <a href="/calendar" class="text-xs text-accent hover:text-accent-strong">Open calendar</a>
        </div>
        {#if todayPosts.length === 0}
          <p class="text-sm text-muted py-6 text-center">Nothing queued for today.</p>
        {:else}
          <ul class="space-y-2">
            {#each todayPosts as post (post.id)}
              <li class="flex items-start gap-3 py-1.5">
                <span class="text-xs text-muted w-14 shrink-0 pt-0.5 tabular-nums">
                  {timezone.formatTime(post.scheduled_at!)}
                </span>
                <span class="text-sm text-content-secondary flex-1 min-w-0 truncate">
                  {post.content || post.title || '(no content)'}
                </span>
                <span class="text-[10px] px-1.5 py-0.5 rounded shrink-0 {STATE_TONE[post.state] ?? 'badge-draft'}">
                  {STATE_LABEL[post.state] ?? post.state}
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">Recent activity</h3>
        {#if recentEvents.length === 0}
          <p class="text-sm text-muted py-6 text-center">No events logged yet.</p>
        {:else}
          <ul class="space-y-3">
            {#each recentEvents as evt (evt.id)}
              <li class="flex items-baseline gap-3">
                <span class="text-xs text-faint shrink-0 w-24" title={timezone.formatDateTime(evt.created_at)}>
                  {timezone.format(evt.created_at, { month: "short", day: "numeric", hour: "numeric", minute: "2-digit" })}
                </span>
                <span class="text-sm text-content-secondary flex-1 min-w-0 truncate">
                  {eventSentence(evt.event_type, evt.payload)}
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>

    <!-- 4. Channel mix + top posts -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">Posts by channel</h3>
        {#if filteredProviders.length === 0}
          <p class="text-sm text-muted py-4 text-center">No posts in this window</p>
        {:else}
          <ul class="space-y-2.5">
            {#each filteredProviders as prov (prov.provider)}
              <li class="flex items-center gap-3">
                <span class="text-xs text-muted w-28 shrink-0 truncate">{providerLabel(prov.provider)}</span>
                <div class="flex-1 bg-background-input rounded-full h-6 overflow-hidden">
                  <!--
                    Solid fill, not /60. A 60%-alpha brand over the light track
                    composites to a pale indigo (~#928EEE), so the count label
                    rendered white-on-that at roughly 2.2:1. app.css already
                    documents --brand-fg (white) on a full --brand fill as
                    4.7:1 in BOTH themes, and a data mark reads better solid.
                  -->
                  <div
                    class="h-full bg-accent-fill rounded-full transition-all duration-500 flex items-center justify-end px-2"
                    style="width: {Math.max((prov.count / maxProviderCount) * 100, 6)}%"
                  >
                    <span class="text-[10px] font-medium text-accent-fg">{prov.count}</span>
                  </div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">Top posts by engagement</h3>
        {#if topPosts.length === 0}
          <p class="text-sm text-muted py-4 text-center">No published posts with engagement data yet</p>
        {:else}
          <ul>
            {#each topPosts as post, i (post.id)}
              <li class="flex items-start gap-3 py-2 border-b border-line last:border-0">
                <span class="text-xs text-faint w-4 shrink-0 mt-0.5 tabular-nums">{i + 1}</span>
                <div class="flex-1 min-w-0">
                  <p class="text-sm text-content-secondary truncate">{post.content || post.title || '(no content)'}</p>
                  <div class="flex gap-3 mt-1 text-[10px] text-muted">
                    <span class="truncate">{post.integration_name}</span>
                    {#if (post.likes ?? 0) > 0}
                      <span class="flex items-center gap-0.5 text-viz-like"><Icon name="heart" class="w-3 h-3" /> {post.likes}</span>
                    {/if}
                    {#if (post.comments ?? 0) > 0}
                      <span class="flex items-center gap-0.5 text-viz-comment"><Icon name="comment-bubble" class="w-3 h-3" /> {post.comments}</span>
                    {/if}
                    {#if (post.shares ?? 0) > 0}
                      <span class="flex items-center gap-0.5 text-viz-share"><Icon name="share" class="w-3 h-3" /> {post.shares}</span>
                    {/if}
                  </div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>

    <!-- 5. Volume over time -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <h3 class="text-sm font-medium text-content mb-4">Posting volume</h3>
      {#if !data || data.posts_by_day.length === 0}
        <p class="text-content-secondary text-sm py-8 text-center">No data for this period</p>
      {:else}
        <div class="flex items-end gap-1 h-40">
          {#each data.posts_by_day as day (day.date)}
            <div class="flex-1 flex flex-col items-center justify-end h-full">
              <div
                class="w-full bg-accent-fill rounded-t hover:bg-accent-fill-hover transition-colors min-h-[4px]"
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

    <!-- Per-provider metric breakdown, only when a single platform is picked -->
    {#if providerAnalytics && providerAnalytics.data && providerAnalytics.data.length > 0}
      <div class="bg-surface border border-line rounded-xl p-5">
        <h3 class="text-sm font-medium text-content mb-4">
          {providers.find(p => p.value === selectedProvider)?.label || selectedProvider} metrics
        </h3>
        <div class="space-y-3">
          {#each providerAnalytics.data as metric (metric.label)}
            {@const values = metric.data.map(d => parseFloat(d.total) || 0)}
            {@const peak = Math.max(...values, 1)}
            <div>
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs text-muted capitalize">{metric.label}</span>
                {#if metric.percentage_change !== 0}
                  <span class="text-xs {metric.percentage_change > 0 ? 'text-success' : 'text-error'}">
                    {metric.percentage_change > 0 ? '+' : ''}{metric.percentage_change.toFixed(1)}%
                  </span>
                {/if}
              </div>
              <div class="flex items-end gap-1 h-20">
                {#each metric.data.slice(-14) as point (point.date)}
                  <div class="flex-1 flex flex-col items-center justify-end h-full">
                    <div
                      class="w-full bg-accent-fill rounded-t hover:bg-accent-fill-hover transition-colors min-h-[2px]"
                      style="height: {Math.max(((parseFloat(point.total) || 0) / peak) * 100, 2)}%"
                      title="{point.date}: {point.total}"
                    ></div>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  {/if}
</div>
