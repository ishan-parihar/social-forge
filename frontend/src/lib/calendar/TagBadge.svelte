<script lang="ts">
  let { tag, size = "sm" }: {
    tag: { name: string; color: string };
    size?: "sm" | "md";
  } = $props();
</script>

<span
  class="rounded-full inline-flex items-center font-medium {size === 'sm' ? 'text-xs px-1.5 py-0.5' : 'text-sm px-2 py-1'}"
  style="background: {tag.color}4D; color: {tag.color}; border: 1px solid {tag.color}33"
>
  {tag.name}
</span>
