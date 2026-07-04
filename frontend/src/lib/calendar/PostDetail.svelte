<script lang="ts">
  import { tick } from "svelte";
  import { goto } from "$app/navigation";
  import Badge from "$lib/ui/Badge.svelte";
  import { engagementIcon, engagementLabel } from "./engagement";
  import type { CalendarEvent as CEvent } from "./types";

  let { event, onclose, onDuplicate, duplicating = false }: { event?: CEvent | null; onclose: () => void; onDuplicate?: (id: string) => void; duplicating?: boolean } = $props();

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
      class="relative w-96 bg-surface border-l border-line p-6 overflow-y-auto outline-none"
    >
      <div class="flex items-center justify-between mb-6">
        <h3 class="font-semibold">Post Details</h3>
        <button onclick={onclose} aria-label="Close" class="text-muted hover:text-white text-xl">&times;</button>
      </div>
      <div class="space-y-4">
        <div>
          <div class="text-xs text-muted mb-1">Platform</div>
          <div class="text-sm">{event.integrationName}</div>
        </div>
        <div>
          <div class="text-xs text-muted mb-1">Status</div>
          <Badge state={event.state as "draft" | "queued" | "published" | "error"} />
        </div>
        <div>
          <div class="text-xs text-muted mb-1">{event.state === 'published' ? 'Posted' : 'Scheduled'}</div>
          <div class="text-sm">{#if event.date}{event.date} {event.time || ""}{:else}<span class="text-muted">Not scheduled (draft)</span>{/if}</div>
        </div>
        <div>
          <div class="text-xs text-muted mb-1">Content</div>
          <div class="text-sm bg-background-input rounded-lg p-3 whitespace-pre-wrap">{event.content}</div>
        </div>
        {#if event.error}
          <div>
            <div class="text-xs text-red-400 mb-1">Error</div>
            <div class="text-sm text-red-300">{event.error}</div>
          </div>
        {/if}
        {#if event.likes != null || event.comments != null || event.shares != null || event.impressions != null}
          <div>
            <div class="text-xs text-muted mb-2">Engagement</div>
            <div class="grid grid-cols-2 gap-2">
              <!-- Positive feedback: likes/upvotes/reactions unified -->
              {#if event.likes != null}
                <div class="bg-background-input border border-line rounded-lg p-2 text-center" title="{engagementLabel('likes', event.platform)}">
                  <div class="text-xs text-pink-400">{engagementIcon('likes', event.platform)}</div>
                  <div class="text-sm font-semibold">{event.likes.toLocaleString()}</div>
                  <div class="text-[10px] text-muted">{engagementLabel('likes', event.platform)}</div>
                </div>
              {/if}
              <!-- Comments/replies unified -->
              {#if event.comments != null}
                <div class="bg-background-input border border-line rounded-lg p-2 text-center" title="{engagementLabel('comments', event.platform)}">
                  <div class="text-xs text-yellow-400">{engagementIcon('comments', event.platform)}</div>
                  <div class="text-sm font-semibold">{event.comments.toLocaleString()}</div>
                  <div class="text-[10px] text-muted">{engagementLabel('comments', event.platform)}</div>
                </div>
              {/if}
              <!-- Shares/reposts/quotes unified -->
              {#if event.shares != null}
                <div class="bg-background-input border border-line rounded-lg p-2 text-center" title="{engagementLabel('shares', event.platform)}">
                  <div class="text-xs text-green-400">{engagementIcon('shares', event.platform)}</div>
                  <div class="text-sm font-semibold">{event.shares.toLocaleString()}</div>
                  <div class="text-[10px] text-muted">{engagementLabel('shares', event.platform)}</div>
                </div>
              {/if}
              <!-- Views/impressions unified -->
              {#if event.impressions != null}
                <div class="bg-background-input border border-line rounded-lg p-2 text-center" title="{engagementLabel('impressions', event.platform)}">
                  <div class="text-xs text-indigo-400">{engagementIcon('impressions', event.platform)}</div>
                  <div class="text-sm font-semibold">{event.impressions.toLocaleString()}</div>
                  <div class="text-[10px] text-muted">{engagementLabel('impressions', event.platform)}</div>
                </div>
              {/if}
            </div>
          </div>
        {/if}
        {#if event.postUrl}
          <a href={event.postUrl} target="_blank" class="inline-flex items-center gap-1.5 text-indigo-400 text-sm hover:text-indigo-300 hover:underline transition-colors">
            <svg class="w-3.5 h-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M6 3l5 5-5 5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            View original post &rarr;
          </a>
        {/if}
        <button onclick={() => onDuplicate?.(event.id)} disabled={duplicating} class="w-full px-3 py-2 bg-[#1a1f2e] hover:bg-[#242b3d] border border-line rounded-lg text-sm text-indigo-400 transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed">
          {#if duplicating}
            <span class="animate-spin">⏳</span> Duplicating...
          {:else}
            📋 Duplicate
          {/if}
        </button>
        <button onclick={() => goto(`/posts/${event.id}`)} class="w-full px-3 py-2 bg-[#1a1f2e] hover:bg-[#242b3d] border border-line rounded-lg text-sm text-indigo-400 transition-colors flex items-center justify-center gap-2">
          ✏️ Edit
        </button>
      </div>
    </div>
  </div>
{/if}
