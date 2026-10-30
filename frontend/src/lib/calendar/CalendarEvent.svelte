  <script lang="ts">
  import type { CalendarEvent as CEvent } from "./types";
  import { engagementIcon, engagementLabel, formatMetricCount } from "./engagement";
  import RepeatingBadge from "./RepeatingBadge.svelte";
  import PostHoverToolbar from "./PostHoverToolbar.svelte";
  import { inkOnFill } from "$lib/chip-colors";
  // v23-8: import calendarState so isPast re-evaluates on the 2min nowTick.
  import { calendarState } from "$lib/stores/calendar.svelte";
  import { timezone } from "$lib/stores/timezone.svelte";
  import { todayKeyInZone } from "./utils";

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
  /**
   * Today's date in the APP's timezone.
   *
   * The event's `date` comes from `wallClock(iso, timezone.value)` - the
   * app's zone. Comparing it against a BROWSER-local today said "the
   * 21st" when the app's zone said "the 20th", so a post that was
   * today in New York was greyed out as past.
   */
  function todayStr() {
    return todayKeyInZone(timezone.value);
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
    class="event-chip {compact ? 'compact' : ''} {event.state} {isPast ? 'event-chip-past' : ''} {event.error ? 'ring-2 ring-error' : ''}
           group-hover:opacity-0 transition-opacity duration-150"
    style={tagBarColor ? "border-left: 3px solid " + tagBarColor : ""}
    title={event.error
      ? `Failed to publish: ${event.error}`
      : (event.content.length > 200 ? event.content.slice(0, 200) + '...' : event.content)}
  >
    {#if event.error}
      <!--
        A red "!" badge makes failed posts immediately visible on the
        calendar without needing to hover. The chip itself also gets
        ring-2 ring-error (above) and the title attribute shows the error
        message on hover.

        The badge sits on the RIGHT. It was on the left, hanging off the
        chip's top-left corner, where it covered the tag chip - and the
        tag is what tells the founder WHICH post this is. The right end
        of the chip holds the metric counts, which are the least
        load-bearing thing on it.

        The glyph is 10px, not 8px: an 8px character is below the 10px
        floor the probe reports, and the badge is the ONE thing on the
        chip that has to be readable at a glance - it is the difference
        between "a failed post" and "a post" being noticed at all.
      -->
      <span
        class="absolute -top-1 -right-1 min-w-[1.125rem] h-[1.125rem] px-0.5 rounded-full bg-error text-error-fg text-[10px] leading-none font-bold flex items-center justify-center shadow-sm z-10 transition-opacity group-hover:opacity-0"
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
       the whole cell's content sideways mid-hover.

       `flex-basis: 0%` is the load-bearing part. With `flex: 1 1 auto` the
       chip's base size is its CONTENT width, and in a narrow week cell
       (80px) the chip shrank to 23px - the 18px error badge plus the 26px
       time - and the TITLE got ZERO pixels. The chip was on the calendar
       showing "06:49" and nothing else.

       With a zero basis the chip takes the cell's width, and the title
       truncates INSIDE it (the ellipsis is the point). */
    flex: 1 1 0%; min-width: 0;
    container-type: inline-size;
  }

  /* The chip is 24px tall in EVERY grid, not just the month one.
     The week and day chips were 21px, and a 21px target is under the WCAG 2.2
     "Target Size (Minimum)" floor of 24x24 (2.5.8) - AGENTS.md 9.37: below 24
     is a FAIL, not a warning. The month grid below had carried the floor all
     along, in `.event-chip.compact`, with the comment that explains it; the
     non-compact chip never got the same rule, and the probe measures the
     rendered box, so the week and day grids were the two that failed.

     `padding: 0.125rem` on an 11px line is what made 21px, so the vertical
     padding carries the box to 24px and the type size does not move. The week
     grid's cells are `min-h-24`, so the extra 3px is absorbed by the cell
     rather than pushing the row apart - the measured acceptance for this
     change is that the CELL does not grow.

     Do NOT "fix" this by removing anything from the chip: the time, the tag
     colour bar, the tag name chips and the metric counts were each proposed
     for deletion by an audit draft that had not rendered the page. They are
     all present, they all clear contrast, and the 21px box was the only
     defect. */
  .event-chip {
    min-height: 24px;
    padding: 0.3125rem 0.375rem;
  }

  /* The COMPACT chip - the month grid - is 24px tall.
     It was 21px, and a 21px target is under even the WCAG 2.2
     "Target Size (Minimum)" floor of 24x24 (2.5.8).

     The month cell is `min-h-24` (96px), which is a MINIMUM: three
     24px chips plus the day number need 112px, and the cell grows
     to fit. The month grid scrolls anyway. */
  .event-chip.compact { min-height: 24px; }

  /* The month grid's chip is 24px tall - see `.event-chip.compact`.
     A pseudo-element hit-area was tried and dropped: the chip has
     `overflow: hidden` (so the title truncates with an ellipsis
     instead of spilling), and an `overflow: hidden` box CLIPS its own
     pseudo-element, so the extension measured back as 24px. */
  /* v22 Phase 3: use CSS variables for state colors so the calendar
     chips retheme correctly in light mode. Previously these hardcoded
     dark-mode hex values. */
  /* The chip's state is carried by the WASH and the LEFT BORDER, and the
     TEXT takes the content ink.
     
     It used to be `color: rgb(var(--error-rgb))` - the state hue as the
     chip's own text colour. That measures 5.00:1 for the light theme's
     error, which sounds fine, but the chip's TIME and METRIC spans
     carried `opacity: 0.7` on top of it, so the RENDERED ink was
     3.24:1 - under the floor, and the probe read the computed colour
     and passed it.
     
     The de-emphasised parts (the time, the metric counts) now take
     `text-muted`, which clears AA on every state wash in both themes
     (5.30:1 to 6.20:1), and the state hue stays in the wash and the
     border where it is a signal and not a word.
  */
  .event-chip { color: rgb(var(--text-rgb)); }
  /* A PAST post is de-emphasised with a lighter INK, not with a group
     `opacity`. An opacity on the chip fades the whole group - wash,
     text and all - into the page, and the text landed at 1.31:1 in the
     light theme: the post was on the calendar and unreadable. The
     muted ink on the state wash is 5.30:1 or better in both themes,
     and the muted tier still reads as "not today".

     The WASH stays. The chip's fill is the state signal, and a past
     post's fill is a legitimate part of that signal - the founder
     still wants to know it was a draft, a queued post or a failure.
     Only the TEXT recedes, and it recedes to a colour that is still
     legible. */
  .event-chip-past { color: rgb(var(--text-muted-rgb)); }
  .event-chip-past .event-time,
  .event-chip-past .metric-item,
  .event-chip-past .tag-overflow { color: rgb(var(--text-muted-rgb)); }
  .event-chip.draft { background: rgb(var(--info-rgb) / 0.15); }
  .event-chip.queued { background: rgb(var(--warning-rgb) / 0.15); }
  .event-chip.published { background: rgb(var(--success-rgb) / 0.15); }
  .event-chip.error { background: rgb(var(--error-rgb) / 0.15); }
  /* `flex-shrink: 0` here meant the tag block REFUSED to shrink, so in a
     narrow week cell it pushed the time and the title out of the chip:
     at 768px the chip showed "launch 11:5" - the time clipped mid-value.
     The tag chips shrink first (they are the least load-bearing: the
     full name is in the tooltip and on the detail panel); the time and
     the title keep their space. */
  /* The tag block can shrink, the time cannot, and the title keeps at
     least a few characters.

     The tag block was `flex-shrink: 0` and the tag chip inside it was
     too, so in a narrow week cell the tags held their width, the time
     held its 26px, the error badge took 18px, and the title was left
     with ZERO pixels. The chip read "06:49" and nothing else.

     So the tag chip keeps a FLOOR of about three characters (a tag that
     reads "r…" is a coloured smear, not a label) and the title yields
     first, down to its own floor of a couple of characters. The full
     name is in the tooltip and on the detail panel either way. */
  .event-tags { display: flex; align-items: center; gap: 2px; flex-shrink: 1; min-width: 0; overflow: hidden; }
  .tag-dot { width: 4px; height: 4px; border-radius: 50%; flex-shrink: 0; }
  .tag-chip {
    max-width: 72px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    padding: 0 4px; border-radius: 3px; font-size: 0.625rem; line-height: 1.3;
    font-weight: 500; flex-shrink: 0; min-width: 1.75rem;
  }
  /* A week cell is ~130px wide and holds the tag, the time and the title.
     The title is the only one of the three that can be shortened without
     losing information the reader cannot get elsewhere (the title
     attribute carries the full text), so the TITLE shrinks first and the
     tag keeps its label. Before this the tag chip was squeezed to 4px -
     a coloured smear, not a label - while the title still had room. */
  /* The title keeps a floor of a couple of characters. Without one it can
     be squeezed to zero - a chip that read "06:49" and no title at all -
     and a title that is gone is worse than a title that is truncated. */
  .event-content { overflow: hidden; text-overflow: ellipsis; min-width: 1.5rem; flex-shrink: 1; }
  /* A tag chip squeezed to 4px is a coloured smear, not a label. The
     tag block is dropped when the CHIP itself is too narrow to hold one
     (a phone-width cell): the colour is still carried by the chip's
     left border (the tag bar), and the full name is in the tooltip and
     on the detail panel. */
  @container (max-width: 90px) {
    .tag-chip { display: none; }
  }
  .tag-overflow { font-size: 0.625rem; color: rgb(var(--text-muted-rgb)); flex-shrink: 0; }
  /* The de-emphasised parts take the MUTED ink, not a dimmed state hue.
     An `opacity` on top of a coloured ink renders lighter than the
     stylesheet says, and the calendar's chip time measured 3.24:1
     that way. `text-muted` on the state wash is 5.30:1 or better in
     both themes. */
  .event-time { color: rgb(var(--text-muted-rgb)); flex-shrink: 0; }
  .event-metrics { display: flex; align-items: center; gap: 0.25rem; flex-shrink: 0; margin-left: auto; overflow: hidden; }
  .metric-item { font-size: 0.625rem; color: rgb(var(--text-muted-rgb)); white-space: nowrap; }
  /* A phone-width cell cannot hold time + title + engagement badges:
     the badges measured OVERLAPPING the time text at 375px (the probe's
     "\u{1f441}\u{2022} 2k overlaps 06:50" FAIL). The badges are secondary
     data - the same floor that drops the tag chips drops them, and the
     full numbers stay in the tooltip, the detail panel and Post Stats. */
  @container (max-width: 90px) {
    .event-metrics { display: none; }
  }
</style>
