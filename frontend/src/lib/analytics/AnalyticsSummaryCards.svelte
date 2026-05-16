<script lang="ts">
  import type { AnalyticsSummary } from '$lib/api/analytics';

  let { data }: { data: AnalyticsSummary } = $props();
</script>

<div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
  <div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4">
    <p class="text-sm text-[#d1d5db] mb-1">Total Posts</p>
    <p class="text-2xl font-bold text-indigo-400">{data.total_posts}</p>
  </div>
  <div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4">
    <p class="text-sm text-[#d1d5db] mb-1">Published</p>
    <p class="text-2xl font-bold text-green-400">{data.published}</p>
  </div>
  <div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4">
    <p class="text-sm text-[#d1d5db] mb-1">Failed</p>
    <p class="text-2xl font-bold text-red-400">{data.failed}</p>
  </div>
  <div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg p-4">
    <p class="text-sm text-[#d1d5db] mb-1">Best Channel</p>
    <p class="text-2xl font-bold text-amber-400">{data.best_provider?.count ?? 0}</p>
    {#if data.best_provider}
      <p class="text-xs text-[#d1d5db] mt-1">{data.best_provider.provider}</p>
    {/if}
  </div>
</div>
