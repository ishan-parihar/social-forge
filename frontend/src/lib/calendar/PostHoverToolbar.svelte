<script lang="ts">
  import { composer } from "$lib/stores/composer.svelte";
  import Dropdown from "$lib/ui/Dropdown.svelte";

  let { eventId, onDuplicate, onStats, onDelete }: {
    eventId: string;
    onDuplicate?: (id: string) => void;
    onStats?: (id: string) => void;
    onDelete?: (id: string) => void;
  } = $props();

  // The four actions used to be four inline buttons in an absolutely
  // positioned strip laid over the event chip. A week cell is about 130px
  // wide and the strip was 116px, so the buttons sat ON TOP of the event
  // title - hovering a chip hid the very text the chip exists to show,
  // and the buttons swallowed clicks aimed at that title.
  //
  // A single "..." trigger opens a menu instead: one glyph fits any cell,
  // the menu is positioned by Dropdown (right-aligned, so it opens left
  // into the calendar instead of off the right edge), and nothing is
  // ever laid over the title.
  let items = $derived([
    { label: "Edit post", onclick: () => composer.openEdit(eventId) },
    ...(onDuplicate ? [{ label: "Duplicate", onclick: () => onDuplicate(eventId) }] : []),
    ...(onStats ? [{ label: "Statistics", onclick: () => onStats(eventId) }] : []),
    ...(onDelete ? [{ label: "Delete", onclick: () => onDelete(eventId), variant: "danger" as const }] : []),
  ]);
</script>

<!-- v25-1: replaced hardcoded bg-[#1e2435]/95 and text-[#9ca3af] with
     semantic tokens (bg-surface/95 + text-muted) so the toolbar rethemes
     correctly in light mode. Hover states use text-content / text-error.

     The click/keydown stop lives on the TRIGGER, not on a wrapper div:
     the chip around it opens the post detail panel on click, so the menu
     trigger has to keep its own clicks to itself. Putting the handler on
     a plain div also earned an a11y warning (a static element with a
     click handler and no role). -->
<div class="shrink-0">
  <Dropdown items={items} align="right" placement="above">
    <button
      aria-label="Post actions"
      class="text-muted hover:text-content transition-colors text-sm leading-none px-1 py-0.5 rounded"
      title="Post actions"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >&#8942;</button>
  </Dropdown>
</div>
