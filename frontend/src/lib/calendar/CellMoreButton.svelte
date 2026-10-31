<script lang="ts">
  let { hidden, expanded, onToggle }: {
    hidden: number;
    expanded: boolean;
    onToggle: () => void;
  } = $props();
</script>

<!--
  The `+N more` row.

  A cell renders at most CELL_CAP chips. Everything past that is behind this
  row, which expands IN PLACE - the chips appear where the row was, and the
  rows below do not move. An expander that opened a dialog or scrolled the
  page would lose the founder's place in the week they were scanning.

  It is a real <button> rather than a clickable <div>, so it takes keyboard
  focus and announces itself. The count is exact, and it is in the accessible
  name, so a screen reader reads "+5 more posts" and not "more".
-->
{#if hidden > 0}
  <button
    type="button"
    onclick={(e) => { e.stopPropagation(); onToggle(); }}
    class="w-full text-left text-[10px] leading-tight text-muted hover:text-content
           bg-surface-hover/60 hover:bg-surface-hover rounded px-1 py-0.5
           transition-colors min-h-6 flex items-center justify-start
           focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
    aria-expanded={expanded}
    aria-label="{expanded ? 'Hide' : 'Show'} {hidden} more posts in this slot"
  >
    +{hidden} more
  </button>
{/if}
