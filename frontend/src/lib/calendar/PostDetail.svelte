<script lang="ts">
  import { tick } from "svelte";
  import Badge from "$lib/ui/Badge.svelte";
  import type { CalendarEvent as CEvent } from "./types";

  let { event, onclose, onDuplicate }: { event?: CEvent | null; onclose: () => void; onDuplicate?: (id: string) => void } = $props();

  let panelEl: HTMLDivElement | undefined = $state();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }

  $effect(() => {
    if (event) {
      tick().then(() => panelEl?.focus());
      document.body.style.overflow = "hidden";
      return () => {
        document.body.style.overflow = "";
      };
    }
  });
</script>

{#if event}
  <div class="fixed inset-0 z-40 flex justify-end" role="dialog" aria-modal="true" onkeydown={handleKeydown}>
    <div class="absolute inset-0 bg-black/40" onclick={onclose} onkeydown={(e) => e.key === "Escape" && onclose()}></div>
    <div
      bind:this={panelEl}
      tabindex="-1"
      class="relative w-96 bg-[#131720] border-l border-[#1e2435] p-6 overflow-y-auto outline-none"
    >
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
        <button onclick={() => onDuplicate?.(event.id)} class="w-full px-3 py-2 bg-[#1a1f2e] hover:bg-[#242b3d] border border-[#2a3045] rounded-lg text-sm text-indigo-400 transition-colors flex items-center justify-center gap-2">
          📋 Duplicate
        </button>
      </div>
    </div>
  </div>
{/if}
