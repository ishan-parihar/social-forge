<script lang="ts">
  import { analyticsApi, type AnalyticsSummary } from '$lib/api/analytics';
  import DateRangePicker from '$lib/analytics/DateRangePicker.svelte';
  import AnalyticsSummaryCards from '$lib/analytics/AnalyticsSummaryCards.svelte';
  import AnalyticsCharts from '$lib/analytics/AnalyticsCharts.svelte';
  import AnalyticsTable from '$lib/analytics/AnalyticsTable.svelte';
  import { toast } from "$lib/stores/toast";

  let days = $state(30);
  let data = $state<AnalyticsSummary | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedProvider = $state<string>("all");

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
    const res = await analyticsApi.getSummary(days, signal);
    if (signal?.aborted) return;
    if (res.error) {
      error = res.error;
      data = null;
    } else if (res.data) {
      data = res.data;
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
</script>

<div class="space-y-6">
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
        <div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4 animate-pulse">
          <div class="h-3 bg-[#2a3045] rounded w-16 mb-3"></div>
          <div class="h-8 bg-[#2a3045] rounded w-12"></div>
        </div>
      {/each}
    </div>
    <div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4 animate-pulse">
      <div class="h-3 bg-[#2a3045] rounded w-32 mb-4"></div>
      <div class="flex items-end gap-1 h-40">
        {#each [1, 2, 3, 4, 5, 6, 7] as _}
          <div class="flex-1 bg-[#2a3045] rounded-t" style="height: {Math.random() * 60 + 10}%"></div>
        {/each}
      </div>
    </div>
  {:else if data && data.total_posts === 0}
    <div class="text-center py-16">
      <p class="text-[#d1d5db] mb-4">No analytics data yet. Start posting!</p>
      <a href="/posts/new" class="inline-flex items-center gap-2 px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-500 transition-colors text-sm">
        Create Post
      </a>
    </div>
  {:else if data}
    <AnalyticsSummaryCards data={data} />
    <AnalyticsCharts postsByDay={data.posts_by_day} />
    <AnalyticsTable postsByProvider={selectedProvider === "all" ? data.posts_by_provider : data.posts_by_provider.filter(p => p.provider === selectedProvider)} />
  {/if}
</div>
