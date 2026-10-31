<script lang="ts">
  import type { CalendarView } from "./types";
  import type { Tag } from "$lib/api/tags";
  import type { Integration } from "$lib/api/integrations";
  import type { Campaign } from "$lib/api/campaigns";

  let { view, currentDate, onPrev, onNext, onToday, onViewChange, tags = [], selectedTagId = null, onTagFilter,
         onNewPost, onGenerate,
         // v24-2: channel + campaign filter props.
         integrations = [], selectedIntegrationId = null, onIntegrationFilter,
         campaigns = [], selectedCampaignId = null, onCampaignFilter }: {
    view: CalendarView; currentDate: Date;
    onPrev: () => void; onNext: () => void; onToday: () => void;
    onViewChange: (v: CalendarView) => void;
    tags?: Tag[];
    selectedTagId?: string | null;
    onTagFilter?: (tagId: string | null) => void;
    /** Open the composer. Present when the header is the page's only route to it. */
    onNewPost?: () => void;
    /** Open the AI generator. */
    onGenerate?: () => void;
    // v24-2: channel + campaign filter props.
    integrations?: Integration[];
    selectedIntegrationId?: string | null;
    onIntegrationFilter?: (id: string | null) => void;
    campaigns?: Campaign[];
    selectedCampaignId?: string | null;
    onCampaignFilter?: (id: string | null) => void;
  } = $props();

  // v28: postiz's view switch carries no per-button glyphs - the label
  // is the whole control, and emoji rendered as color glyphs that fought
  // both themes. Text-only keeps the segmented control neutral.
  const views: { key: CalendarView; label: string }[] = [
    { key: "day", label: "Day" },
    { key: "week", label: "Week" },
    { key: "month", label: "Month" },
    { key: "list", label: "List" },
  ];

  let title = $derived.by(() => {
    const d = currentDate;
    const m = d.toLocaleDateString("en-US", { month: "long" });
    if (view === "month") return `${m} ${d.getFullYear()}`;
    if (view === "week") return `Week of ${m} ${d.getDate()}`;
    if (view === "day") return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
    return "Upcoming Posts";
  });

  // The select controls share one height and one minimum width so the row
  // reads as a set. `h-10` is 40px - verified MEASURED 2026-10-31 against the
  // nav group's own border box at 40px, and both the selects and the group
  // render 40px at 1440, so no control in this row sticks out of the line.
  //
  // `min-w-[120px]` is measured, not decorative: the selects come out 120px
  // wide with the shortest label and grow, which is why the row wraps.
  //
  // THE ROW WRAPS ANYWAY - 88px over TWO rows, measured at 1440, 1600, 1920
  // and 2560. It is not short of width: `max-w-6xl` in +layout.svelte caps the
  // row at 1040px at every one of those viewports, and it still wraps. The
  // equal 40px heights keep the wrap clean (two aligned lines, not a ragged
  // edge) - they do not prevent it. An earlier version of this comment claimed
  // "the row is one line tall" on the strength of the heights matching, and
  // the rendered app contradicted it at every width tested.
  const selectClass = "h-10 min-w-[120px] px-2 bg-background-input border border-line rounded-lg text-xs text-content focus:outline-none focus:border-accent";
</script>

<!-- C-2: ONE row, in postiz's order. The three rows this replaces were
     the `h1` + two action buttons (44px), this header (46px), and the two
     `space-y-6` gaps between them - 218px of chrome above the grid at 1440,
     which the page's own comment said was ~168px while the CSS fallback
     said 218px. Three numbers for one thing is how a constant goes stale.

     218px is HISTORY, NOT THE CURRENT VALUE. It describes what was removed.
     MEASURED 2026-10-31, this header occupies 88px of its own two rows and
     the grid starts at 180px from the top of the viewport - so this row, the
     page padding and the shell's own top margin account for 180px where the
     old three rows accounted for 218px. Anyone reading "218px" as a
     description of what is on screen now will be wrong by 38px; that is why
     the number is spelled out as past tense and the current number is too.

     What is left is the date range as the title, the nav, the filters, the
     view switcher, and the two actions - on one line that wraps to at most
     two rows at 768. READ THAT WITH CARE: the wrap is not a small-screen
     behaviour. Measured 2026-10-31 the header is 88px over TWO rows at 1440,
     1600, 1920 and 2560 as well - `max-w-6xl` caps it at 1040px at every one
     of those, and there is more content than that holds. Two rows is what it
     is everywhere; "at 768" was measuring the width where it was first
     noticed, not where it starts. Nothing here reaches for `new Date(...)`:
     the range still resolves through `wallClockToUtc` in the page (AGENTS.md
     9.17). -->
<div class="flex items-center flex-wrap gap-x-2 gap-y-2">
  <!-- The nav arrows and the date are ONE bordered group, so the date reads
       as the thing being navigated rather than a title sitting between two
       loose buttons. `divide-x` gives the seams without a border per child.

       The date is the one part that must never be truncated - "Week of
       October 31" is what the page is for. The ARROWS are the part that can
       give way, so they are icon-width rather than 44px buttons: a 44px tap
       target for a chevron inside a 40px row is not reachable anyway, and
       this row has a `Today` that is a comfortable target for the same
       intent. The nav group measured 296px with generous arrows; trimming
       them keeps the whole header on one line at 1440. -->
  <div class="flex items-stretch border border-line rounded-lg overflow-hidden h-10 shrink-0">
    <button
      onclick={onPrev}
      aria-label="Previous"
      class="px-2 text-sm text-muted hover:text-content hover:bg-surface-hover transition-colors flex items-center justify-center"
    >&larr;</button>
    <button
      onclick={onNext}
      aria-label="Next"
      class="px-2 text-sm text-muted hover:text-content hover:bg-surface-hover transition-colors border-l border-line flex items-center justify-center"
    >&rarr;</button>
    <!-- The DATE is the page's h1. It was a span inside a loose row of
         buttons, with a separate `h1` reading "Content Calendar" above it -
         two headings for one idea, and the one that carried no information
         was the one marked up as the landmark. The date range says which week
         the founder is looking at, which is the thing the heading exists to
         say. The nav rail's active item carries the "Calendar" name, and the
         probe's own rule (no `<h1>` is a FAIL) is satisfied by the range. -->
    <h1 class="px-2.5 text-sm font-semibold border-l border-line flex items-center whitespace-nowrap">{title}</h1>
    <button
      onclick={onToday}
      class="px-2.5 text-xs text-muted hover:text-content hover:bg-surface-hover transition-colors border-l border-line flex items-center"
    >Today</button>
  </div>

  <!-- v24-2: channel filter. Native semantics kept; only the geometry is
       uniform now (h-10 + min-w), which is what makes the row read as a set. -->
  {#if integrations.length > 0 && onIntegrationFilter}
    <select
      value={selectedIntegrationId ?? ""}
      onchange={(e) => onIntegrationFilter(e.currentTarget.value || null)}
      class={selectClass}
      aria-label="Filter by channel"
    >
      <option value="">All channels</option>
      {#each integrations as integ (integ.id)}
        <option value={integ.id}>{integ.provider_name || integ.provider_identifier}</option>
      {/each}
    </select>
  {/if}
  <!-- v24-2: campaign filter -->
  {#if campaigns.length > 0 && onCampaignFilter}
    <select
      value={selectedCampaignId ?? ""}
      onchange={(e) => onCampaignFilter(e.currentTarget.value || null)}
      class={selectClass}
      aria-label="Filter by campaign"
    >
      <option value="">All campaigns</option>
      {#each campaigns as camp (camp.id)}
        <option value={camp.id}>{camp.name}</option>
      {/each}
    </select>
  {/if}
  {#if tags.length > 0 && onTagFilter}
    <select
      value={selectedTagId ?? ""}
      onchange={(e) => onTagFilter(e.currentTarget.value || null)}
      class={selectClass}
      aria-label="Filter by tag"
    >
      <option value="">All tags</option>
      {#each tags as tag}
        <option value={tag.id}>{tag.name}</option>
      {/each}
    </select>
  {/if}

  <!-- The view switcher is a SEGMENTED CONTROL: one bordered group, one
       accent pill for the active view, `aria-pressed` on every option so the
       state is announced rather than only drawn. It was four independent
       buttons inside a border, which is the same geometry but none of the
       semantics. -->
  <div class="flex items-center border border-line rounded-lg overflow-hidden h-10 shrink-0" role="group" aria-label="Calendar view">
    {#each views as v}
      <button
        onclick={() => onViewChange(v.key)}
        aria-pressed={view === v.key}
        class="px-2.5 text-xs font-medium transition-colors h-full
          {view === v.key ? 'bg-accent-fill text-accent-fg' : 'text-muted hover:text-content hover:bg-surface-hover'}"
      >{v.label}</button>
    {/each}
  </div>

  <!-- The two actions live here now, not in a second row above. They were on
       their own 44px line that existed only to carry them. -->
  {#if onGenerate}
    <button
      onclick={onGenerate}
      class="h-10 px-2.5 text-xs border border-line rounded-lg text-muted hover:text-content hover:bg-surface-hover transition-colors shrink-0"
      title="Generate multiple posts from a topic using AI"
    >Generate Posts</button>
  {/if}
  {#if onNewPost}
    <button
      onclick={onNewPost}
      class="h-10 px-2.5 bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg text-xs font-medium transition-colors shrink-0"
    >+ New Post</button>
  {/if}
</div>
