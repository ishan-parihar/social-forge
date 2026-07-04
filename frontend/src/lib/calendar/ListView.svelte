<script lang="ts">
  import Badge from "$lib/ui/Badge.svelte";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";
  import type { CalendarEvent as CEvent } from "./types";

  let { events = [], selected = new Set(), onEventClick, onDuplicate, onStats, onDelete, onToggleSelect, page = 1, totalPages = 1, totalItems = 0, onPageChange, pageSize = 20, showActions = false }: {
    events?: CEvent[];
    selected?: Set<string>;
    onEventClick?: (id: string) => void;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
    onToggleSelect?: (id: string, e: Event) => void;
    page?: number;
    totalPages?: number;
    totalItems?: number;
    onPageChange?: (p: number) => void;
    pageSize?: number;
    showActions?: boolean;
  } = $props();

  let sorted = $derived([...events].sort((a, b) => {
    // Drafts without scheduled dates go to the end
    if (!a.date) return 1;
    if (!b.date) return -1;
    return a.date.localeCompare(b.date) || (a.time || "00:00").localeCompare(b.time || "00:00");
  }));
</script>

<div class="bg-surface border border-line rounded-xl overflow-hidden">
  <div class="text-xs text-muted px-4 py-2 border-b border-line">Upcoming posts</div>
  {#if sorted.length === 0}
    <div class="text-center py-12 text-sm text-muted">No posts scheduled</div>
  {:else}
    {#each sorted as event (event.id)}
      <div class="group relative w-full flex items-center gap-4 px-4 py-3 border-b border-line hover:bg-surface-hover transition-colors">
        {#if onToggleSelect}
          <input type="checkbox" checked={selected.has(event.id)} onclick={(e) => onToggleSelect?.(event.id, e)} class="rounded shrink-0" />
        {/if}
        <button onclick={() => onEventClick?.(event.id)} class="flex-1 flex items-center gap-4 text-left min-w-0">
          <div class="text-xs text-muted w-24 shrink-0">
            {#if event.date}
              {event.date}
              {#if event.time}<br><span class="text-indigo-400">{event.time}</span>{/if}
            {:else}
              <span class="text-muted-dark">Draft</span>
            {/if}
          </div>
          <div class="flex-1 min-w-0">
            <div class="text-sm truncate">{event.title}</div>
            <div class="text-xs text-muted">{event.integrationName}</div>
            {#if event.likes != null || event.comments != null || event.impressions != null}
              <div class="flex gap-2 mt-0.5">
                {#if event.likes != null}<span class="text-[10px] text-pink-400/60" title="{engagementLabel('likes', event.platform)}">{engagementIcon('likes', event.platform)} {formatMetricCount(event.likes)}</span>{/if}
                {#if event.comments != null}<span class="text-[10px] text-yellow-400/60" title="{engagementLabel('comments', event.platform)}">{engagementIcon('comments', event.platform)} {formatMetricCount(event.comments)}</span>{/if}
                {#if event.impressions != null}<span class="text-[10px] text-indigo-400/60" title="{engagementLabel('impressions', event.platform)}">{engagementIcon('impressions', event.platform)} {formatMetricCount(event.impressions)}</span>{/if}
              </div>
            {/if}
          </div>
          <Badge state={event.state as "draft" | "queued" | "published" | "error"} />
          {#if event.error}
            <span class="text-xs text-red-400" title={event.error}>⚠</span>
          {/if}
        </button>
        <div class="invisible group-hover:visible group-focus-within:visible transition-all duration-150 flex items-center gap-1 shrink-0">
          {#if event.state === 'published' && event.postUrl}
            <a href={event.postUrl} target="_blank" rel="noopener noreferrer"
               class="text-indigo-400 hover:text-indigo-300 px-1.5 py-0.5 rounded text-xs"
               title="View original post">🔗</a>
          {/if}
          <button onclick={() => onDuplicate?.(event.id)} class="text-[#9ca3af] hover:text-white px-1.5 py-0.5 rounded text-xs" title="Duplicate" aria-label="Duplicate post">📋</button>
          <button onclick={() => onStats?.(event.id)} class="text-[#9ca3af] hover:text-white px-1.5 py-0.5 rounded text-xs" title="Statistics" aria-label="View post statistics">📊</button>
          <button onclick={() => onDelete?.(event.id)} class="text-[#9ca3af] hover:text-red-400 px-1.5 py-0.5 rounded text-xs" title="Delete" aria-label="Delete post">🗑️</button>
        </div>
      </div>
    {/each}
  {/if}

  {#if totalPages > 1}
    <div class="flex items-center justify-between px-4 py-3 border-t border-line">
      <span class="text-sm text-muted">
        Showing {(page - 1) * pageSize + 1}–{Math.min(page * pageSize, totalItems)} of {totalItems}
      </span>
      <div class="flex gap-2">
        <button
          onclick={() => onPageChange?.(page - 1)}
          disabled={page <= 1}
          class="px-3 py-1 text-sm rounded bg-[#1e2435] text-content-secondary disabled:opacity-50 hover:bg-[#2a3045] transition-colors"
        >← Previous</button>
        <button
          onclick={() => onPageChange?.(page + 1)}
          disabled={page >= totalPages}
          class="px-3 py-1 text-sm rounded bg-[#1e2435] text-content-secondary disabled:opacity-50 hover:bg-[#2a3045] transition-colors"
        >Next →</button>
      </div>
    </div>
  {/if}
</div>
