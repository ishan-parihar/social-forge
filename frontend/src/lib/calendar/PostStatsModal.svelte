<script lang="ts">
  // PostStatsModal — per-post analytics with charts (Phase 1, v19).
  //
  // Upgraded from a right-side slide-in panel to a centered ModalManager
  // modal. Shows:
  //   - Date-range selector (7 / 30 / 90 days)
  //   - Grid of 4 metric cards (Impressions, Likes, Shares, Comments)
  //     with totals + delta indicators
  //   - Per-metric bar charts with date labels
  //
  // Inspired by postiz-app's StatisticsModal, but uses inline bar charts
  // (no Chart.js dependency — YAGNI for 4 small charts).

  import { analyticsApi } from "$lib/api/analytics";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";
  import type { AnalyticsDataPoint } from "$lib/api/analytics";

  let { postId, postTitle, onclose }: {
    postId: string;
    postTitle: string;
    onclose: () => void;
  } = $props();

  let days = $state<7 | 30 | 90>(7);
  let analyticsData = $state<AnalyticsDataPoint[] | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let abortController: AbortController | null = null;

  let totalImpressions = $derived(analyticsData?.reduce((s, d) => s + d.impressions, 0) ?? 0);
  let totalLikes = $derived(analyticsData?.reduce((s, d) => s + d.likes, 0) ?? 0);
  let totalShares = $derived(analyticsData?.reduce((s, d) => s + d.shares, 0) ?? 0);
  let totalComments = $derived(analyticsData?.reduce((s, d) => s + d.comments, 0) ?? 0);

  let chartData = $derived.by(() => {
    if (!analyticsData || analyticsData.length === 0) return [];
    let data: AnalyticsDataPoint[];
    if (days === 7) {
      data = analyticsData.slice(-7);
    } else if (days === 30) {
      data = sampleData(analyticsData, 10);
    } else {
      data = sampleData(analyticsData, 10);
    }
    return data;
  });

  function sampleData(data: AnalyticsDataPoint[], count: number): AnalyticsDataPoint[] {
    if (data.length <= count) return data;
    const step = (data.length - 1) / (count - 1);
    const result: AnalyticsDataPoint[] = [];
    for (let i = 0; i < count; i++) {
      const idx = Math.round(i * step);
      result.push(data[Math.min(idx, data.length - 1)]);
    }
    return result;
  }

  let maxImpressions = $derived(Math.max(...chartData.map(d => d.impressions), 1));
  let maxLikes = $derived(Math.max(...chartData.map(d => d.likes), 1));
  let maxShares = $derived(Math.max(...chartData.map(d => d.shares), 1));
  let maxComments = $derived(Math.max(...chartData.map(d => d.comments), 1));

  let metricCards = $derived<{ label: string; icon: string; value: number; color: string; key: keyof AnalyticsDataPoint }[]>([
    { label: "Impressions", icon: '👁️', value: totalImpressions, color: "#818cf8", key: "impressions" },
    { label: "Likes", icon: '❤️', value: totalLikes, color: "#f472b6", key: "likes" },
    { label: "Shares", icon: '🔄', value: totalShares, color: "#34d399", key: "shares" },
    { label: "Comments", icon: '💬', value: totalComments, color: "#fbbf24", key: "comments" },
  ]);

  function shortDate(dateStr: string): string {
    const d = new Date(dateStr + "T00:00:00");
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  }

  async function fetchData() {
    abortController?.abort();
    abortController = new AbortController();
    const signal = abortController.signal;
    loading = true;
    error = null;
    try {
      const r = await analyticsApi.getPostAnalytics(postId, days, signal);
      if (signal.aborted) return;
      if (r.data?.data) {
        analyticsData = r.data.data;
      } else {
        error = r.error || "No data returned";
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
          {days === d ? 'bg-indigo-600 text-white' : 'text-muted hover:text-white hover:bg-surface-hover'}"
      >{d}d</button>
    {/each}
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <span class="animate-spin text-muted text-2xl">⏳</span>
    </div>
  {:else if error && !analyticsData}
    <div class="text-center py-8 text-sm text-red-400">{error}</div>
  {:else if analyticsData && analyticsData.length === 0}
    <div class="text-center py-8 text-sm text-muted">
      No analytics data available yet.
      <br>
      <span class="text-xs">The feed refresher pulls engagement data every 30 minutes.</span>
    </div>
  {:else if analyticsData}
    <!-- Metric cards grid -->
    <div class="grid grid-cols-2 gap-3">
      {#each metricCards as card (card.label)}
        <div class="bg-background-input border border-line rounded-lg p-3">
          <div class="text-xs text-muted mb-1">{card.icon} {card.label}</div>
          <div class="text-lg font-semibold text-white">{card.value.toLocaleString()}</div>
          {#if analyticsData.length >= 2}
            {@const first = analyticsData[0][card.key] as number}
            {@const last = analyticsData[analyticsData.length - 1][card.key] as number}
            <div class="text-xs mt-1 {last >= first ? 'text-green-400' : 'text-red-400'}">
              {last >= first ? '↑' : '↓'} {Math.abs(last - first).toLocaleString()}
            </div>
          {/if}
        </div>
      {/each}
    </div>

    <!-- Per-metric bar charts -->
    <div class="space-y-4">
      {#each metricCards as card (card.label)}
        <div>
          <div class="text-xs text-muted mb-2">{card.icon} {card.label}</div>
          <div class="flex items-end gap-1 h-20">
            {#each chartData as point, i (i)}
              <div class="flex-1 flex flex-col items-center justify-end h-full">
                <div
                  class="w-full rounded-t transition-all duration-300"
                  style="height: {((point[card.key] as number) / (card.key === 'impressions' ? maxImpressions : card.key === 'likes' ? maxLikes : card.key === 'shares' ? maxShares : maxComments)) * 100}%; background: {card.color}; min-height: {(point[card.key] as number) > 0 ? '4px' : '0'}"
                ></div>
                {#if chartData.length <= 7 || (chartData.length <= 10 && i % 2 === 0) || i % 3 === 0}
                  <span class="text-[8px] text-muted mt-1 truncate w-full text-center">{shortDate(point.date)}</span>
                {/if}
              </div>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
