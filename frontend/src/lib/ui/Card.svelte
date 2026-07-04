<script lang="ts">
  let { children, class: className = "", padding = "p-5" }: {
    children: import('svelte').Snippet;
    class?: string;
    padding?: string;
  } = $props();
</script>

<div class="bg-surface border border-line rounded-xl {padding} {className}">
  {@render children()}
</div>
