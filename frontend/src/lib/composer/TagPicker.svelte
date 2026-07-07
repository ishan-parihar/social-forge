<script lang="ts">
  import { onMount } from 'svelte';
  import { tagsApi, type Tag } from '$lib/api/tags';

  let { selected = [], onToggle }: {
    selected?: string[];
    onToggle?: (tagId: string) => void;
  } = $props();

  let tags = $state<Tag[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  onMount(async () => {
    try {
      const r = await tagsApi.list();
      if (r.data) tags = r.data;
      else error = r.error || 'Failed to load tags';
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || 'Failed to load tags';
    }
    loading = false;
  });
</script>

<div>
  {#if loading}
    <div class="text-sm text-muted py-2">Loading tags...</div>
  {:else if error}
    <div class="text-sm text-error py-2">{error}</div>
  {:else if tags.length === 0}
    <div class="text-sm text-muted py-2">
      No tags yet.
      <a href="/tags" class="text-brand-400 hover:underline">Create some in the Tags page.</a>
    </div>
  {:else}
    <div class="flex flex-wrap gap-2">
      {#each tags as tag (tag.id)}
        {@const isSelected = selected.includes(tag.id)}
        <button
          onclick={() => onToggle?.(tag.id)}
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs border transition-colors
            {isSelected
              ? 'border-transparent text-white'
              : 'border-line text-muted hover:text-white hover:bg-surface-hover'}"
          style={isSelected ? `background: ${tag.color}; border-color: ${tag.color};` : ''}
          aria-label="{isSelected ? 'Remove' : 'Add'} tag {tag.name}"
        >
          <span
            class="w-2 h-2 rounded-full"
            style="background: {isSelected ? 'white' : tag.color}"
          ></span>
          {tag.name}
          {#if isSelected}
            <span class="text-xs ml-0.5">&#10003;</span>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>
