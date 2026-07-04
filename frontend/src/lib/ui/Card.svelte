<script lang="ts">
  let { children, class: className = "", padding = "p-5" }: {
    children: import('svelte').Snippet;
    class?: string;
    padding?: string;
  } = $props();
</script>

<div class="bg-[#131720] border border-[#1e2435] rounded-xl {padding} {className}">
  {@render children()}
</div>
