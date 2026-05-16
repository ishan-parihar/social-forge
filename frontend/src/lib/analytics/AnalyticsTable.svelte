<script lang="ts">
  let { postsByProvider }: { postsByProvider: Array<{ provider: string; count: number }> } = $props();
</script>

<div class="bg-[#1a1f2e] border border-[#2a3045] rounded-lg overflow-hidden">
  <table class="w-full">
    <thead>
      <tr class="border-b border-[#2a3045]">
        <th class="text-left text-xs font-medium text-[#d1d5db] uppercase tracking-wider px-4 py-3">Provider</th>
        <th class="text-right text-xs font-medium text-[#d1d5db] uppercase tracking-wider px-4 py-3">Posts</th>
      </tr>
    </thead>
    <tbody>
      {#each postsByProvider as row}
        <tr class="border-b border-[#1e2435] hover:bg-[#1e2435]/50 transition-colors">
          <td class="px-4 py-3 text-sm text-[#e8edf5]">{row.provider}</td>
          <td class="px-4 py-3 text-sm text-right text-[#e8edf5]">{row.count}</td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
