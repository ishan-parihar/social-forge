<script lang="ts">
  // v22 Phase 3: PageHeader primitive — title + optional subtitle +
  // actions slot. Standardizes route headers.
  let {
    title,
    subtitle,
    children,
  }: {
    title: string;
    subtitle?: string;
    children?: import("svelte").Snippet;
  } = $props();
</script>

<div class="flex items-start justify-between gap-4 mb-6">
  <div>
    <h1 class="text-xl font-semibold text-content">{title}</h1>
    {#if subtitle}
      <p class="text-sm text-muted mt-1">{subtitle}</p>
    {/if}
  </div>
  {#if children}
    <div class="flex items-center gap-2">
      {@render children()}
    </div>
  {/if}
</div>
