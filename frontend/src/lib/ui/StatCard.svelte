<script lang="ts">
  // v22 Phase 3: StatCard primitive — number + label + optional trend
  // delta. Used by the dashboard widgets.
  let {
    label,
    value,
    delta,
    deltaLabel,
    color = "default",
    icon,
  }: {
    label: string;
    value: string | number;
    delta?: number; // positive = up, negative = down
    deltaLabel?: string;
    color?: "default" | "success" | "warning" | "error" | "info";
    icon?: string;
  } = $props();

  const accent = {
    default: "text-content",
    success: "text-success",
    warning: "text-warning",
    error: "text-error",
    info: "text-info",
  };
</script>

<div class="stat-card bg-surface border border-line rounded-lg p-4">
  <div class="flex items-center justify-between mb-2">
    <span class="text-xs text-muted">{label}</span>
    {#if delta !== undefined}
      <span class="text-xs {delta >= 0 ? 'text-success' : 'text-error'}">
        {delta >= 0 ? "▲" : "▼"} {Math.abs(delta)}{deltaLabel ?? "%"}
      </span>
    {/if}
  </div>
  <div class="text-2xl font-semibold {accent[color]}">{value}</div>
</div>
