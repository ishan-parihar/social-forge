<script lang="ts">
  // v22 Phase 3: EmptyState primitive — icon + title + description +
  // optional action button. Replaces ad-hoc "No X yet" divs.
  import Icon from "./Icon.svelte";

  let {
    icon = "inbox",
    title,
    description,
    actionLabel,
    onaction,
  }: {
    icon?: string;
    title: string;
    description?: string;
    actionLabel?: string;
    onaction?: () => void;
  } = $props();
</script>

<div class="text-center py-12 px-4">
  <div class="inline-flex items-center justify-center w-12 h-12 rounded-full bg-surface-hover mb-3">
    <Icon name={icon} class="w-5 h-5 text-muted" />
  </div>
  <h3 class="text-sm font-medium text-content mb-1">{title}</h3>
  {#if description}
    <p class="text-xs text-muted max-w-sm mx-auto">{description}</p>
  {/if}
  {#if actionLabel && onaction}
    <button
      onclick={onaction}
      class="mt-4 px-3 py-1.5 bg-brand-500 hover:bg-brand-600 text-white text-xs rounded-md transition-colors"
    >
      {actionLabel}
    </button>
  {/if}
</div>
