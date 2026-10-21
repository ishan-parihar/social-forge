<script lang="ts">
  // PostStatsModal - per-post analytics with charts (Phase 1, v19).
  //
  // Upgraded from a right-side slide-in panel to a centered ModalManager
  // modal. Shows:
  //   - Date-range selector (7 / 30 / 90 days)
  //   - Grid of 4 metric cards (Impressions, Likes, Shares, Comments)
  //     with totals + delta indicators
  //   - Per-metric bar charts with date labels
  //
  // Inspired by postiz-app's StatisticsModal, but uses inline bar charts
  // (no Chart.js dependency - YAGNI for 4 small charts).
  //
  // The API shape is the provider's: a list of series, one per metric,
  // each `[{label, data: [{total, date}], percentage_change}]` - the same
  // shape `/api/analytics?provider=` returns. The modal used to type it
  // as a flat `AnalyticsDataPoint[]` with `impressions`/`likes` fields,
  // which is NOT what the server sends: every metric read `undefined`,
  // the sums came out NaN and the chart labels rendered "Invalid Date".

  import { analyticsApi, type AnalyticsSeries } from "$lib/api/analytics";
  import { integrationsApi } from "$lib/api/integrations";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";

  let { postId, postTitle, integrationId, integrationName, onclose }: {
    postId: string;
    postTitle: string;
    integrationId?: string;
    integrationName?: string;
    onclose: () => void;
  } = $props();

  let days = $state<7 | 30 | 90>(7);
  let analyticsData = $state<AnalyticsSeries[] | null>(null);
  let loading = $state(false);
  let refreshing = $state(false);

  /** A provider-token 401 is an actionable state: the channel's token
   *  expired, and the fix is a refresh - not a trip to the Channels
   *  page to hunt for which one. Offer the refresh INLINE. When the
   *  refresh ITSELF fails (an expired X cookie session has no usable
   *  refresh token), the button can never succeed - clear the state
   * and offer the real next step: reconnect the channel. */
  let tokenExpired = $state(false);
  let needsReconnect = $state(false);

  /** Refresh the channel's token, then reload the analytics. */
  async function refreshChannelToken() {
    if (!integrationId || refreshing) return;
    refreshing = true;
    try {
      const r = await integrationsApi.refresh(integrationId);
      if (r.error) {
        // The refresh failed - for an expired cookie session it will
        // fail again every time. Stop offering it.
        tokenExpired = false;
        needsReconnect = true;
      } else {
        tokenExpired = false;
        needsReconnect = false;
        error = null;
        await fetchData();
      }
    } finally {
      refreshing = false;
    }
  }
  let error = $state<string | null>(null);
  let abortController: AbortController | null = null;

  /// The four metrics this modal charts, in display order. Each names the
  /// series label the provider uses and the token that colours it.
  const METRICS = [
    { key: "impressions", label: "Impressions", icon: "👁️", color: "rgb(var(--viz-impression-rgb))" },
    { key: "likes", label: "Likes", icon: "❤️", color: "rgb(var(--viz-like-rgb))" },
    { key: "shares", label: "Shares", icon: "🔄", color: "rgb(var(--viz-share-rgb))" },
    { key: "comments", label: "Comments", icon: "💬", color: "rgb(var(--hue-amber-rgb))" },
  ] as const;

  type MetricKey = (typeof METRICS)[number]["key"];

  /// One series per metric, matched by label. A provider that does not
  /// report a metric simply has no series for it.
  function seriesFor(key: MetricKey): AnalyticsSeries | undefined {
    return analyticsData?.find((s) => s.label.toLowerCase() === key);
  }

  /// The points of a metric's series, newest last. `total` arrives as a
  /// string from the provider, so it is parsed here - a string that
  /// survives into arithmetic is what produced NaN totals.
  function pointsFor(key: MetricKey): { total: number; date: string }[] {
    const series = seriesFor(key);
    if (!series) return [];
    return series.data.map((p) => ({
      total: Number(p.total) || 0,
      date: p.date || "",
    }));
  }

  function totalFor(key: MetricKey): number {
    const points = pointsFor(key);
    if (points.length === 0) return 0;
    // The provider's `total` on the last point is the running total, so
    // the LAST point is the total - summing them would double count.
    return points[points.length - 1].total;
  }

  /// Change between the first and the last point of the series.
  function deltaFor(key: MetricKey): number | null {
    const points = pointsFor(key);
    if (points.length < 2) return null;
    return points[points.length - 1].total - points[0].total;
  }

  function chartPointsFor(key: MetricKey): { total: number; date: string }[] {
    const points = pointsFor(key);
    if (points.length === 0) return [];
    if (points.length <= 10) return points;
    // Sample down to 10 bars so a 90-day range stays readable.
    const step = (points.length - 1) / 9;
    const out: { total: number; date: string }[] = [];
    for (let i = 0; i < 10; i++) {
      out.push(points[Math.min(Math.round(i * step), points.length - 1)]);
    }
    return out;
  }

  function maxFor(key: MetricKey): number {
    return Math.max(...chartPointsFor(key).map((p) => p.total), 1);
  }

  function shortDate(dateStr: string): string {
    if (!dateStr) return "";
    // The provider sends an ISO timestamp; `new Date` on a bare
    // "YYYY-MM-DD" is parsed as UTC, which can land on the previous day
    // in a western timezone. Pin it to noon so the day never shifts.
    const iso = dateStr.length === 10 ? `${dateStr}T12:00:00` : dateStr;
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return "";
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  async function fetchData() {
    abortController?.abort();
    abortController = new AbortController();
    const signal = abortController.signal;
    loading = true;
    error = null;
    tokenExpired = false;
    needsReconnect = false;
    try {
      const r = await analyticsApi.getPostAnalytics(postId, days, signal);
      if (signal.aborted) return;
      if (r.data?.data) {
        analyticsData = r.data.data;
      } else {
        error = r.error || "No data returned";
        // A provider-token 401: the client carries the envelope's
        // `code` alongside the unwrapped message, so branch on the
        // machine-readable reason, not the human string.
        if (r.code === "token_expired") {
          tokenExpired = true;
        }
        analyticsData = null;
      }
    } catch (e: unknown) {
      if (e instanceof Error && e.name === 'AbortError') return;
      error = (e instanceof Error ? e.message : String(e)) || "Failed to load analytics";
      analyticsData = null;
    } finally {
      if (!signal.aborted) loading = false;
    }
  }

  $effect(() => {
    fetchData();
  });
</script>

<div class="space-y-5">
  <!-- Date range selector -->
  <div class="flex bg-background-input rounded-lg border border-line overflow-hidden">
    {#each [7, 30, 90] as d (d)}
      <button
        onclick={() => days = d as 7 | 30 | 90}
        class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors
          {days === d ? 'bg-accent-fill text-accent-fg' : 'text-muted hover:text-content hover:bg-surface-hover'}"
      >{d}d</button>
    {/each}
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <span class="animate-spin text-muted text-2xl">⏳</span>
    </div>
  {:else if error && !analyticsData}
    <div class="text-center py-8 text-sm text-error">
      {error}
      {#if tokenExpired && integrationId}
        <div class="mt-3">
          <button
            onclick={refreshChannelToken}
            disabled={refreshing}
            class="min-h-11 px-4 py-2 rounded-lg bg-accent-fill hover:bg-accent-fill-hover text-accent-fg text-sm font-medium disabled:opacity-50 transition-colors"
          >
            {refreshing ? 'Refreshing…' : `Refresh ${integrationName || 'channel'} token`}
          </button>
          <div class="text-xs text-muted mt-2">
            {integrationName || 'The channel'}'s access token expired. Refresh it and the stats load again.
          </div>
        </div>
      {:else if needsReconnect}
        <!-- The refresh FAILED - for an expired cookie session it can
             never succeed, so the real next step is reconnecting the
             channel. A button that cannot succeed is worse than no
             button. -->
        <div class="mt-3">
          <a
            href="/channels"
            class="inline-flex items-center min-h-11 px-4 rounded-lg bg-accent-fill hover:bg-accent-fill-hover text-accent-fg text-sm font-medium"
          >
            Reconnect {integrationName || 'the channel'}
          </a>
          <div class="text-xs text-muted mt-2">
            The token refresh failed - {integrationName || 'the channel'} needs reconnecting from the Channels page.
          </div>
        </div>
      {/if}
    </div>
  {:else if !analyticsData || analyticsData.length === 0}
    <div class="text-center py-8 text-sm text-muted">
      No analytics data available yet.
      <br>
      <span class="text-xs">The feed refresher pulls engagement data every 30 minutes.</span>
    </div>
  {:else}
    <!-- Metric cards grid -->
    <div class="grid grid-cols-2 gap-3">
      {#each METRICS as metric (metric.key)}
        {@const delta = deltaFor(metric.key)}
        <div class="bg-background-input border border-line rounded-lg p-3">
          <div class="text-xs text-muted mb-1">{metric.icon} {metric.label}</div>
          <div class="text-lg font-semibold text-content">{formatMetricCount(totalFor(metric.key))}</div>
          {#if delta !== null}
            <div class="text-xs mt-1 {delta >= 0 ? 'text-success' : 'text-error'}">
              {delta >= 0 ? '↑' : '↓'} {formatMetricCount(Math.abs(delta))}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Per-metric bar charts -->
    <div class="space-y-4">
      {#each METRICS as metric (metric.key)}
        {@const points = chartPointsFor(metric.key)}
        {#if points.length > 0}
          <div>
            <div class="text-xs text-muted mb-2">{metric.icon} {metric.label}</div>
            <div class="flex items-end gap-1 h-20">
              {#each points as point, i (i)}
                <div class="flex-1 flex flex-col items-center justify-end h-full">
                  <div
                    class="w-full rounded-t transition-all duration-300"
                    style="height: {(point.total / maxFor(metric.key)) * 100}%; background: {metric.color}; min-height: {point.total > 0 ? '4px' : '0'}"
                    title="{metric.label}: {formatMetricCount(point.total)}"
                  ></div>
                  {#if points.length <= 7 || (points.length <= 10 && i % 2 === 0) || i % 3 === 0}
                    <span class="text-[8px] text-muted mt-1 truncate w-full text-center">{shortDate(point.date)}</span>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
