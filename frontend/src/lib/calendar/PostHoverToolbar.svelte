<script lang="ts">
  import { composer } from "$lib/stores/composer.svelte";

  let { eventId, onDuplicate, onStats, onDelete }: {
    eventId: string;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
  } = $props();
</script>

<!-- v25-1: replaced hardcoded bg-[#1e2435]/95 and text-[#9ca3af] with
     semantic tokens (bg-surface/95 + text-muted) so the toolbar rethemes
     correctly in light mode. Hover states use text-content / text-error. -->
<div class="absolute top-1 right-1 flex items-center gap-0.5 bg-surface/95 border border-line rounded-md shadow-lg z-10">
  <button
    onclick={() => composer.openEdit(eventId)}
    class="text-muted hover:text-content px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Edit"
    aria-label="Edit post"
  >✏️</button>
  <button
    onclick={() => onDuplicate?.(eventId)}
    class="text-muted hover:text-content px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Duplicate"
    aria-label="Duplicate post"
  >📋</button>
  <button
    onclick={() => onStats?.(eventId)}
    class="text-muted hover:text-content px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Statistics"
    aria-label="View post statistics"
  >📊</button>
  <button
    onclick={() => onDelete?.(eventId)}
    class="text-muted hover:text-error px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Delete"
    aria-label="Delete post"
  >🗑️</button>
</div>
