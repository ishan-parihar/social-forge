<script lang="ts">
  let { size = "md" }: { size?: "sm" | "md" | "lg" } = $props();
  const dims = { sm: "h-4 w-4", md: "h-6 w-6", lg: "h-8 w-8" };
</script>

<svg class="animate-spin {dims[size]} text-indigo-500" fill="none" viewBox="0 0 24 24" role="status">
  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
</svg>
<span class="sr-only">Loading...</span>
