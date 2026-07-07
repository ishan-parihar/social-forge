<script lang="ts">
  // v22 Phase 3: Avatar primitive — image with fallback initial.
  // Used by feed, comments, channel selector.
  let {
    src,
    name,
    size = "md",
    alt,
  }: {
    src?: string | null;
    name?: string;
    size?: "xs" | "sm" | "md" | "lg";
    alt?: string;
  } = $props();

  const sizeClass = {
    xs: "w-6 h-6 text-[10px]",
    sm: "w-8 h-8 text-xs",
    md: "w-10 h-10 text-sm",
    lg: "w-12 h-12 text-base",
  };

  // Derive up to 2 initials from name, uppercase.
  let initials = $derived(
    name
      ? name
          .split(/\s+/)
          .filter(Boolean)
          .slice(0, 2)
          .map((w) => w[0]?.toUpperCase() ?? "")
          .join("")
      : "?"
  );

  let errored = $state(false);
</script>

<div
  class="inline-flex items-center justify-center rounded-full bg-surface-hover text-muted font-medium {sizeClass[size]} overflow-hidden flex-shrink-0"
  title={alt ?? name}
>
  {#if src && !errored}
    <img {src} alt={alt ?? name ?? "avatar"} class="w-full h-full object-cover" onerror={() => (errored = true)} />
  {:else}
    {initials}
  {/if}
</div>
