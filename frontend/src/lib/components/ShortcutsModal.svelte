<script lang="ts">
  // Keyboard shortcuts cheat-sheet (Phase 0 — migrated to ModalManager).
  //
  // Previously this was an inline hand-rolled overlay in +layout.svelte.
  // Now it's a component rendered by ModalManager via the modals store,
  // so it stacks correctly with other modals and Escape is handled
  // centrally by ModalManager's keydown listener.

  let { close } = $props<{ close: (confirmed?: boolean) => void }>();
</script>

<div class="space-y-2 text-sm">
  <div class="flex justify-between"><span class="text-muted">New post</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">n</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Search</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">/</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Go to Calendar</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g c</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Go to Posts</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g p</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Go to Feed</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g f</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Go to Analytics</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g a</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Go to Media</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g m</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Show this help</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">?</kbd></div>
  <div class="flex justify-between"><span class="text-muted">Close modal</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">Esc</kbd></div>
</div>
<p class="text-xs text-muted-dark mt-4 text-center">Shortcuts are disabled while typing in inputs.</p>
