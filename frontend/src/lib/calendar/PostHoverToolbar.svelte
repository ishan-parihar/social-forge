<script lang="ts">
  import { goto } from "$app/navigation";

  let { eventId, onDuplicate, onStats, onDelete }: {
    eventId: string;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
  } = $props();
</script>

<div class="absolute top-1 right-1 flex items-center gap-0.5 bg-[#1e2435]/95 rounded-md shadow-lg z-10">
  <button
    onclick={() => goto(`/posts/${eventId}`)}
    class="text-[#9ca3af] hover:text-white px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Edit"
    aria-label="Edit post"
  >✏️</button>
  <button
    onclick={() => onDuplicate?.(eventId)}
    class="text-[#9ca3af] hover:text-white px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Duplicate"
    aria-label="Duplicate post"
  >📋</button>
  <button
    onclick={() => onStats?.(eventId)}
    class="text-[#9ca3af] hover:text-white px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Statistics"
    aria-label="View post statistics"
  >📊</button>
  <button
    onclick={() => onDelete?.(eventId)}
    class="text-[#9ca3af] hover:text-red-400 px-1.5 py-0.5 rounded text-xs transition-colors"
    title="Delete"
    aria-label="Delete post"
  >🗑️</button>
</div>
