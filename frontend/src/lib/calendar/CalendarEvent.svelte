  <script lang="ts">
  import type { CalendarEvent as CEvent } from "./types";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";
  import RepeatingBadge from "./RepeatingBadge.svelte";
  import PostHoverToolbar from "./PostHoverToolbar.svelte";
  import { inkOnFill } from "$lib/chip-colors";
  // v23-8: import calendarState so isPast re-evaluates on the 2min nowTick.
  import { calendarState } from "$lib/stores/calendar.svelte";

  let { event, compact = false, onDuplicate, onStats, onDelete }: {
    event: CEvent;
    compact?: boolean;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
  } = $props();

  let visibleTags = $derived((event.tags || []).slice(0, 2));
  let overflowCount = $derived((event.tags?.length || 0) - 2);

  // v23-8: read calendarState.nowTick so isPast re-evaluates every 2min.
  // Without this, a tab left open overnight wouldn't flip yesterday's
  // cells to greyscale until a manual refresh.
  let isPast = $derived.by(() => {
    void calendarState.state.nowTick; // dependency - recomputes when tick changes
    return event.date ? event.date < todayStr() : false;
  });
  function todayStr() {
    const d = new Date();
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  }

  // v22 Phase 7: postiz-style tag-color top bar. If the post has tags
  // (and no campaign color takes precedence), use the first tag's color
  // as a top-bar background with mix-blend-difference for the label.
  let tagBarColor = $derived(
    event.campaignColor
      ? event.campaignColor
      : (event.tags && event.tags.length > 0 ? event.tags[0].color : null)
  );
</script>

<!-- The hover actions are a SIBLING of the chip, absolutely positioned
     over it. They used to be an overlay inside the chip, so they sat on
     top of the event title: a week cell is only ~130px wide and the
     four-button toolbar is 116px, so hovering a chip hid the very text
     the chip exists to show (the probe reported the title overlapping
     all four buttons on every cell).

     Now the chip's own content - the tag chips, the time, the title and
     the metric counts - fades out on hover and the actions take the
     chip's place. Nothing overlaps anything; the title is still on the
     chip's `title` attribute and the click target opens the detail
     panel. -->
<div class="group relative flex-1 min-w-0 flex items-center">
  <div
    class="event-chip {event.state} {isPast ? 'opacity-50' : ''} {event.error ? 'ring-2 ring-error' : ''}
           group-hover:opacity-0 transition-opacity duration-150"
    style={tagBarColor ? "border-left: 3px solid " + tagBarColor : ""}
    title={event.error
      ? `Failed to publish: ${event.error}`
      : (event.content.length > 200 ? event.content.slice(0, 200) + '...' : event.content)}
  >
    {#if event.error}
      <!--
        A red "!" badge in the top-left corner makes failed posts
        immediately visible on the calendar without needing to hover.
        The chip itself also gets ring-2 ring-error (above) and the
        title attribute shows the error message on hover.

        The glyph is 10px, not 8px: an 8px character is below the 10px
        floor the probe reports, and the badge is the ONE thing on the
        chip that has to be readable at a glance - it is the difference
        between "a failed post" and "a post" being noticed at all.
        The badge box grows to match so the glyph is not squeezed.
      -->
      <span
        class="absolute -top-1 -left-1 min-w-[1.125rem] h-[1.125rem] px-0.5 rounded-full bg-error text-error-fg text-[10px] leading-none font-bold flex items-center justify-center shadow-sm z-10"
        title={`Publishing failed: ${event.error}`}
      >!</span>
    {/if}
    {#if event.tags && event.tags.length > 0}
      <span class="event-tags">
        {#if compact}
          <!-- Month cells are space-starved: a 3px dot carries the hue without
               eating the row. The name is still one hover away. -->
          {#each visibleTags as tag (tag.id)}
            <span class="tag-dot" style="background: {tag.color}" title={tag.name}></span>
          {/each}
        {:else}
          <!-- v25 F3: readable tag-color chips. The old 3px dot was a color
               nobody could name - two tags in a week cell looked identical
               unless you already knew what you tagged them. A chip with the
               name in it is self-describing; the color still does the fast
               visual grouping job.

               The chip is a SOLID fill of the tag colour with the ink
               computed from it. It used to be the tag colour as TEXT on a
               13%-alpha wash of itself, which measures 4.01:1 for a
               mid-tone hue like indigo - under the WCAG AA floor for
               small text, and the probe gates on it. Black or white on a
               solid fill always clears 4.5:1, whatever the hue. -->
          {#each visibleTags as tag (tag.id)}
            <span class="tag-chip" style="background-color: {tag.color}; color: {inkOnFill(tag.color)}" title={tag.name}>
              {tag.name}
            </span>
          {/each}
        {/if}
        {#if overflowCount > 0}
          <span class="tag-overflow" title="{event.tags?.length} tags">+{overflowCount}</span>
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
    <!-- The actions are OUT OF FLOW (`absolute`), so they never steal width
         from the event title - a week cell is only ~130px wide and the
         action strip is ~40px, which is a third of the title.

         They used to be an `absolute top-1 right-1` strip laid over the
         chip's own content, and before that an in-flow sibling. Either
         way the buttons sat on top of the event title: hovering a chip
         hid the very text the chip exists to show, and the buttons
         swallowed clicks aimed at that title. The probe reported the
         title overlapping all four buttons on every calendar cell.

         Now the chip's content fades out on hover and the actions take
         its place. The chip's `title` attribute still carries the full
         text, and the click target opens the detail panel.

         `invisible` (not just `opacity-0`) is deliberate: an opacity-0
         element is still laid out and still swallows clicks, so the
         actions were intercepting clicks meant for the title even when
         nobody could see them.

         `visible` is set on the wrapper AND on the menu, because
         `visibility` inherits: a `visibility: hidden` ancestor keeps the
         dropdown menu hidden even when it is open. -->
    <div class="absolute top-0 right-0 invisible group-hover:visible group-focus-within:visible [&_.dropdown-menu]:visible flex items-center gap-0.5">
      {#if event.state === 'published' && event.postUrl}
        <a href={event.postUrl} target="_blank" rel="noopener noreferrer"
           class="text-[10px] text-accent/60 hover:text-accent-strong px-1 py-0.5 rounded transition-colors min-h-11 min-w-11 inline-flex items-center justify-center"
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
    /* Fill the row so the title collapsing on hover does not reflow the
       cell - the chip used to shrink to the width of its tags, which moved
       the whole cell's content sideways mid-hover. */
    flex: 1 1 auto; min-width: 0;
    container-type: inline-size;
  }  /* v22 Phase 3: use CSS variables for state colors so the calendar
     chips retheme correctly in light mode. Previously these hardcoded
     dark-mode hex values. */
  .event-chip.draft { background: rgb(var(--info-rgb) / 0.15); color: rgb(var(--info-rgb)); }
  .event-chip.queued { background: rgb(var(--warning-rgb) / 0.15); color: rgb(var(--warning-rgb)); }
  .event-chip.published { background: rgb(var(--success-rgb) / 0.15); color: rgb(var(--success-rgb)); }
  .event-chip.error { background: rgb(var(--error-rgb) / 0.15); color: rgb(var(--error-rgb)); }
  /* `flex-shrink: 0` here meant the tag block REFUSED to shrink, so in a
     narrow week cell it pushed the time and the title out of the chip:
     at 768px the chip showed "launch 11:5" - the time clipped mid-value.
     The tag chips shrink first (they are the least load-bearing: the
     full name is in the tooltip and on the detail panel); the time and
     the title keep their space. */
  .event-tags { display: flex; align-items: center; gap: 2px; flex-shrink: 0; min-width: 0; overflow: hidden; }
  .tag-dot { width: 4px; height: 4px; border-radius: 50%; flex-shrink: 0; }
  .tag-chip {
    max-width: 72px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    padding: 0 4px; border-radius: 3px; font-size: 0.625rem; line-height: 1.3;
    font-weight: 500; flex-shrink: 0;
  }
  /* A week cell is ~130px wide and holds the tag, the time and the title.
     The title is the only one of the three that can be shortened without
     losing information the reader cannot get elsewhere (the title
     attribute carries the full text), so the TITLE shrinks first and the
     tag keeps its label. Before this the tag chip was squeezed to 4px -
     a coloured smear, not a label - while the title still had room. */
  .event-content { overflow: hidden; text-overflow: ellipsis; min-width: 0; flex-shrink: 1; }
  /* A tag chip squeezed to 4px is a coloured smear, not a label. The
     tag block is dropped when the CHIP itself is too narrow to hold one
     (a phone-width cell): the colour is still carried by the chip's
     left border (the tag bar), and the full name is in the tooltip and
     on the detail panel. */
  @container (max-width: 90px) {
    .tag-chip { display: none; }
  }
  .tag-overflow { font-size: 0.625rem; opacity: 0.6; flex-shrink: 0; }
  .event-time { opacity: 0.7; flex-shrink: 0; }
  .event-metrics { display: flex; align-items: center; gap: 0.25rem; flex-shrink: 0; margin-left: auto; overflow: hidden; }
  .metric-item { font-size: 0.625rem; opacity: 0.6; white-space: nowrap; }
</style>
