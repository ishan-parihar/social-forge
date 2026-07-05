  <script lang="ts">
  import type { CalendarEvent as CEvent } from "./types";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";
  import RepeatingBadge from "./RepeatingBadge.svelte";
  import PostHoverToolbar from "./PostHoverToolbar.svelte";

  let { event, compact = false, onDuplicate, onStats, onDelete }: {
    event: CEvent;
    compact?: boolean;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
  } = $props();

  let visibleTags = $derived((event.tags || []).slice(0, 2));
  let overflowCount = $derived((event.tags?.length || 0) - 2);

  let isPast = $derived(event.date ? event.date < todayStr() : false);
  function todayStr() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }
</script>

<div class="group relative">
  <div
    class="event-chip {event.state} {isPast ? 'opacity-50' : ''} {event.error ? 'ring-1 ring-red-500/50' : ''}"
    style={event.tags && event.tags.length > 0 ? "border-left: 3px solid " + event.tags[0].color : ""}
    title={event.content.length > 200 ? event.content.slice(0, 200) + '...' : event.content}
  >
    {#if !compact && event.tags && event.tags.length > 0}
      <span class="event-tags">
        {#each visibleTags as tag (tag.id)}
          <span class="tag-dot" style="background: {tag.color}"></span>
        {/each}
        {#if overflowCount > 0}
          <span class="tag-overflow">+{overflowCount}</span>
        {/if}
      </span>
    {/if}
    {#if !compact && event.repeatIntervalDays}
      <RepeatingBadge intervalDays={event.repeatIntervalDays} />
    {/if}
    {#if !compact}
      <span class="event-time">{event.time || ""}</span>
    {/if}
    <span class="event-content">{event.title}</span>
    {#if !compact && (event.likes != null || event.impressions != null)}
      <span class="event-metrics">
        {#if event.likes != null}
          <span class="metric-item" title="{engagementLabel('likes', event.platform)}">
            {engagementIcon('likes', event.platform)} {formatMetricCount(event.likes)}
          </span>
        {/if}
        {#if event.impressions != null}
          <span class="metric-item" title="{engagementLabel('impressions', event.platform)}">
            {engagementIcon('impressions', event.platform)} {formatMetricCount(event.impressions)}
          </span>
        {/if}
      </span>
    {/if}
  </div>

  {#if !compact}
    <div class="opacity-0 group-hover:opacity-100 transition-opacity duration-150 flex items-center gap-1">
      {#if event.state === 'published' && event.postUrl}
        <a href={event.postUrl} target="_blank" rel="noopener noreferrer"
           class="text-[8px] text-indigo-400/60 hover:text-indigo-300 px-1 py-0.5 rounded transition-colors"
           title="View original post" onclick={(e) => e.stopPropagation()}>
          🔗
        </a>
      {/if}
      <PostHoverToolbar eventId={event.id} {onDuplicate} {onStats} {onDelete} />
    </div>
  {/if}
</div>

<style>
  .event-chip {
    display: flex; align-items: center; gap: 0.25rem;
    padding: 0.125rem 0.375rem; border-radius: 0.25rem;
    font-size: 0.6875rem; cursor: grab; white-space: nowrap; overflow: hidden;
  }
  .event-chip.draft { background: rgba(107,114,128,0.15); color: #9ca3af; }
  .event-chip.queued { background: rgba(99,102,241,0.15); color: #818cf8; }
  .event-chip.published { background: rgba(34,197,94,0.15); color: #4ade80; }
  .event-chip.error { background: rgba(239,68,68,0.15); color: #f87171; }
  .event-tags { display: flex; align-items: center; gap: 1px; flex-shrink: 0; }
  .tag-dot { width: 3px; height: 3px; border-radius: 50%; flex-shrink: 0; }
  .tag-overflow { font-size: 0.5625rem; opacity: 0.6; margin-left: 1px; }
  .event-time { opacity: 0.7; flex-shrink: 0; }
  .event-content { overflow: hidden; text-overflow: ellipsis; }
  .event-metrics { display: flex; align-items: center; gap: 0.25rem; flex-shrink: 0; margin-left: auto; }
  .metric-item { font-size: 0.5625rem; opacity: 0.6; white-space: nowrap; }
</style>
