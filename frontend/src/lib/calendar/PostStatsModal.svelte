<script lang="ts">
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
  let panelEl: HTMLDivElement | undefined = $state();
  let dialogEl: HTMLDivElement | undefined = $state();
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

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
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
    } catch (e: any) {
      if (e.name === "AbortError") return;
      error = e.message || "Failed to load analytics";
      analyticsData = null;
    } finally {
      if (!signal.aborted) loading = false;
    }
  }

  $effect(() => {
    fetchData();
  });

  $effect(() => {
    document.body.style.overflow = "hidden";
    dialogEl?.focus();
    return () => {
      document.body.style.overflow = "";
    };
  });
</script>

<div bind:this={dialogEl} class="fixed inset-0 z-40 flex justify-end" role="dialog" aria-modal="true" aria-labelledby="stats-modal-title" tabindex="-1" onkeydown={handleKeydown}>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="absolute inset-0 bg-black/40" onclick={onclose}></div>
  <div bind:this={panelEl} tabindex="-1" class="relative w-96 bg-[#131720] border-l border-[#1e2435] p-6 overflow-y-auto outline-none">
    <div class="flex items-center justify-between mb-6">
      <h3 id="stats-modal-title" class="font-semibold text-sm truncate pr-2" title={postTitle}>{postTitle}</h3>
      <button onclick={onclose} aria-label="Close" class="text-[#6b7280] hover:text-white text-xl shrink-0">&times;</button>
    </div>

    <div class="flex bg-[#0d1117] rounded-lg border border-[#1e2435] overflow-hidden mb-6">
      {#each [7, 30, 90] as d (d)}
        <button
          onclick={() => days = d as 7 | 30 | 90}
          class="flex-1 px-3 py-1.5 text-xs font-medium transition-colors
            {days === d ? 'bg-indigo-600 text-white' : 'text-[#6b7280] hover:text-white hover:bg-[#1a1f2e]'}"
        >{d}d</button>
      {/each}
    </div>

    {#if loading}
      <div class="flex items-center justify-center py-12">
        <span class="animate-spin text-[#6b7280] text-2xl">&#x23F3;</span>
      </div>
    {:else if error && !analyticsData}
      <div class="text-center py-8 text-sm text-red-400">{error}</div>
    {:else if analyticsData && analyticsData.length === 0}
      <div class="text-center py-8 text-sm text-[#6b7280]">No analytics data available yet.</div>
    {:else if analyticsData}
      <div class="grid grid-cols-2 gap-3 mb-6">
        {#each metricCards as card (card.label)}
          <div class="bg-[#0d1117] border border-[#1e2435] rounded-lg p-3">
            <div class="text-xs text-[#6b7280] mb-1">{card.icon} {card.label}</div>
            <div class="text-lg font-semibold text-white">{card.value.toLocaleString()}</div>
            {#if analyticsData.length >= 2}
              {@const first = analyticsData[0][card.key] as number}
              {@const last = analyticsData[analyticsData.length - 1][card.key] as number}
              <div class="text-xs mt-1 {last >= first ? 'text-green-400' : 'text-red-400'}">
                {last >= first ? '\u2191' : '\u2193'} {Math.abs(last - first).toLocaleString()}
              </div>
            {/if}
          </div>
        {/each}
      </div>

      <div class="space-y-5">
        {#each metricCards as card (card.label)}
          <div>
            <div class="text-xs text-[#6b7280] mb-2">{card.icon} {card.label}</div>
            <div class="flex items-end gap-1 h-24">
              {#each chartData as point, i (i)}
                <div class="flex-1 flex flex-col items-center justify-end h-full">
                  <div
                    class="w-full rounded-t transition-all duration-300"
                    style="height: {((point[card.key] as number) / (card.key === 'impressions' ? maxImpressions : card.key === 'likes' ? maxLikes : card.key === 'shares' ? maxShares : maxComments)) * 100}%; background: {card.color}; min-height: {(point[card.key] as number) > 0 ? '4px' : '0'}"
                  ></div>
                  {#if chartData.length <= 7 || (chartData.length <= 10 && i % 2 === 0) || i % 3 === 0}
                    <span class="text-[8px] text-[#6b7280] mt-1 truncate w-full text-center">{shortDate(point.date)}</span>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>