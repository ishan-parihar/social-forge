<script lang="ts">
let { intervalDays }: { intervalDays?: number | null } = $props();
</script>

{#if intervalDays}
  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium bg-brand-500/20 text-brand-400"
        title="Repeats every {intervalDays} days">
    <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
      <path d="M1 4v6h6M23 20v-6h-6" />
      <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
    </svg>
    {intervalDays}d
  </span>
{/if}
