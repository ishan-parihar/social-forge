<script lang="ts">
  import Badge from "$lib/ui/Badge.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { event, onclose }: { event?: CEvent | null; onclose: () => void } = $props();

  // Close on Escape
  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

{#if event}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-40 flex justify-end" role="dialog" aria-modal="true" onkeydown={handleKeydown}>
    <div class="absolute inset-0 bg-black/40" onclick={onclose}></div>
    <div class="relative w-96 bg-[#131720] border-l border-[#1e2435] p-6 overflow-y-auto" tabindex="-1">
      <div class="flex items-center justify-between mb-6">
        <h3 class="font-semibold">Post Details</h3>
        <button onclick={onclose} aria-label="Close" class="text-[#6b7280] hover:text-white text-xl">&times;</button>
      </div>
      <div class="space-y-4">
        <div>
          <div class="text-xs text-[#6b7280] mb-1">Platform</div>
          <div class="text-sm">{event.integrationName}</div>
        </div>
        <div>
          <div class="text-xs text-[#6b7280] mb-1">Status</div>
          <Badge state={event.state} />
        </div>
        <div>
          <div class="text-xs text-[#6b7280] mb-1">Scheduled</div>
          <div class="text-sm">{event.date} {event.time || ""}</div>
        </div>
        <div>
          <div class="text-xs text-[#6b7280] mb-1">Content</div>
          <div class="text-sm bg-[#0d1117] rounded-lg p-3 whitespace-pre-wrap">{event.content}</div>
        </div>
        {#if event.error}
          <div>
            <div class="text-xs text-red-400 mb-1">Error</div>
            <div class="text-sm text-red-300">{event.error}</div>
          </div>
        {/if}
        {#if event.postUrl}
          <a href={event.postUrl} target="_blank" class="text-indigo-400 text-sm hover:underline block">
            View on platform &rarr;
          </a>
        {/if}
      </div>
    </div>
  </div>
{/if}
