<script lang="ts">
  // v22 Phase 3: Tabs primitive — segmented control with active state.
  // Replaces ad-hoc tab strips in settings/posts/analytics.
  //
  // v25 F5: `aria-label` is now a prop rather than a fixed string. Four call
  // sites render this with different tab sets, and a tablist all called
  // "Tabs" told a screen-reader user nothing about which strip they were in.
  // `orientation` lets a caller that lays the strip out vertically say so.
  let {
    tabs,
    value = $bindable(),
    label = "Tabs",
  }: {
    tabs: { id: string; label: string }[];
    value: string;
    label?: string;
  } = $props();

  // F5: the ARIA tabs pattern requires arrow-key movement, not Tab-through.
  // Without it the strip is operable but hostile — N tabs means N stops, and
  // the convention every other tablist on the platform breaks is Left/Right.
  function onKeydown(e: KeyboardEvent, index: number) {
    const last = tabs.length - 1;
    let next: number | null = null;
    if (e.key === "ArrowRight" || e.key === "ArrowDown") next = index === last ? 0 : index + 1;
    else if (e.key === "ArrowLeft" || e.key === "ArrowUp") next = index === 0 ? last : index - 1;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = last;
    if (next === null) return;
    e.preventDefault();
    value = tabs[next].id;
    // Roving focus: the moved-to tab takes the caret, so the next arrow press
    // continues from where the user is rather than from the old index.
    (e.currentTarget as HTMLElement)
      .closest('[role="tablist"]')
      ?.querySelectorAll<HTMLElement>('[role="tab"]')
      [next]?.focus();
  }
</script>

<div class="inline-flex bg-surface-hover rounded-lg p-0.5 border border-line" role="tablist" aria-label={label}>
  {#each tabs as tab, i (tab.id)}
    <button
      role="tab"
      aria-selected={value === tab.id}
      tabindex={value === tab.id ? 0 : -1}
      onclick={() => (value = tab.id)}
      onkeydown={(e) => onKeydown(e, i)}
      class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors {value === tab.id ? 'bg-accent-fill text-accent-fg' : 'text-muted hover:text-content'}"
    >
      {tab.label}
    </button>
  {/each}
</div>
