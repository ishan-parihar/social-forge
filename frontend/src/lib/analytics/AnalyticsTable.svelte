<script lang="ts">
  let { postsByProvider }: { postsByProvider: Array<{ provider: string; count: number }> } = $props();
</script>

<div class="bg-surface border border-line rounded-lg overflow-hidden">
  <table class="w-full">
    <thead>
      <tr class="border-b border-line">
        <th class="text-left text-xs font-medium text-content-secondary uppercase tracking-wider px-4 py-3">Provider</th>
        <th class="text-right text-xs font-medium text-content-secondary uppercase tracking-wider px-4 py-3">Posts</th>
      </tr>
    </thead>
    <tbody>
      {#each postsByProvider as row (row.provider)}
        <tr class="border-b border-line hover:bg-[#1e2435]/50 transition-colors">
          <td class="px-4 py-3 text-sm text-content">{row.provider}</td>
          <td class="px-4 py-3 text-sm text-right text-content">{row.count}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
