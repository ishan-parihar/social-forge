<script lang="ts">
  let { hidden, expanded, onToggle }: {
    hidden: number;
    expanded: boolean;
    onToggle: () => void;
  } = $props();
</script>

<!--
  The `+N more` row, and - once expanded - the row that puts the cap back.

  A cell renders at most CELL_CAP chips. Everything past that is behind this
  row, which expands IN PLACE - the chips appear where the row was, and the
  rows below do not move. An expander that opened a dialog or scrolled the page
  would lose the founder's place in the week they were scanning.

  WHILE EXPANDED there is nothing left to hide, so `hidden` is 0 - and a
  component that returned nothing then left the cell with no way back to the
  capped state (MEASURED 2026-10-31: expanding "+7 more" removed its own
  control, and the only recovery was leaving and re-entering the month view).
  So the expanded state keeps the row, reads as the inverse action, and names
  the count it will hide again.

  It is a real <button> rather than a clickable <div>, so it takes keyboard
  focus and announces itself. The count is exact and in the accessible name,
  so a screen reader reads "+5 more posts" and not "more".
-->
{#if expanded}
  <button
    type="button"
    onclick={(e) => { e.stopPropagation(); onToggle(); }}
    class="w-full text-left text-[10px] leading-tight text-muted hover:text-content
           bg-surface-hover/60 hover:bg-surface-hover rounded px-1 py-0.5
           transition-colors min-h-6 flex items-center justify-start
           focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
    aria-expanded={true}
    aria-label="Collapse this slot"
  >
    Show less
  </button>
{:else if hidden > 0}
  <button
    type="button"
    onclick={(e) => { e.stopPropagation(); onToggle(); }}
    class="w-full text-left text-[10px] leading-tight text-muted hover:text-content
           bg-surface-hover/60 hover:bg-surface-hover rounded px-1 py-0.5
           transition-colors min-h-6 flex items-center justify-start
           focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
    aria-expanded={false}
    aria-label="Show {hidden} more posts in this slot"
  >
    +{hidden} more
  </button>
{/if}
