<script lang="ts">
  // v22 Phase 3: Skeleton primitive — shimmer placeholder for loading
  // states. Uses the .skeleton class from app.css.
  let {
    width = "100%",
    height = "1rem",
    rounded = "md",
  }: {
    width?: string;
    height?: string;
    rounded?: "sm" | "md" | "lg" | "full";
  } = $props();

  const r = { sm: "rounded-sm", md: "rounded-md", lg: "rounded-lg", full: "rounded-full" };
</script>

<div
  class="skeleton {r[rounded]}"
  style="width: {width}; height: {height};"
></div>
