<script lang="ts">
  let { state = "draft" }: { state?: string } = $props();
  const colors: Record<string, string> = {
    draft: "bg-gray-500/20 text-gray-400",
    queued: "bg-blue-500/20 text-blue-400",
    published: "bg-green-500/20 text-green-400",
    error: "bg-red-500/20 text-red-400",
  };
</script>
<span class="px-2 py-0.5 rounded text-xs font-medium {colors[state] || colors.draft}">
  {state}
</span>
