<script lang="ts">
  // v25 F1: the single top bar.
  //
  // Before this, the brand mark + StreakBadge + NotificationBell existed in two
  // places: inside the desktop sidebar header, and again in a separate
  // `lg:hidden` bar above the page content. Two mount points for the same
  // live widgets means the notification count and streak can disagree between
  // them (they were separate component instances), and any fix had to be made
  // twice. The desktop one was also buried in the sidebar, where a user on a
  // wide screen had to look left to see an unread badge on the right.
  //
  // This component owns the whole cluster. The layout renders it once per
  // breakpoint via `variant`:
  //   "bar"   — full width, sticky, hamburger + brand + widgets (mobile)
  //   "rail"  — the brand mark only, no widgets; used inside the collapsed
  //             sidebar, where there is no room for a badge
  // The desktop sidebar header keeps the `rail` variant; the page-level bar is
  // the one place the widgets live on desktop too.
  import StreakBadge from '$lib/streak/StreakBadge.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';

  let {
    variant = "bar",
    ontogglenav,
  }: {
    variant?: "bar" | "rail";
    ontogglenav?: () => void;
  } = $props();
</script>

{#if variant === "bar"}
  <div class="h-14 flex items-center justify-between px-4 border-b border-line bg-surface flex-shrink-0">
    <div class="flex items-center gap-2 min-w-0">
      {#if ontogglenav}
        <button
          onclick={ontogglenav}
          class="text-content hover:text-accent transition-colors p-1 -ml-1"
          aria-label="Toggle navigation"
        >
          <svg class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/>
          </svg>
        </button>
      {/if}
      <span class="text-accent font-bold text-lg truncate">Social Forge</span>
    </div>
    <!-- The live widgets. One instance, one source of truth. -->
    <div class="flex items-center gap-1 flex-shrink-0">
      <StreakBadge />
      <NotificationBell />
    </div>
  </div>
{:else}
  <!-- rail: brand mark only. Streak + notifications live in the top bar. -->
  <span class="text-accent font-bold text-lg truncate">Social Forge</span>
{/if}
