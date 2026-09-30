<script lang="ts">
  // v25 F1: error primitive — the third leg of the route-state triplet
  // (Skeleton / EmptyState / ErrorState).
  //
  // Before this existed, 25 route pages each rendered their own failure
  // surface: 9 as bare red text, 5 as a `bg-error/10` banner, the rest as a
  // toast that disappears before it can be read. The problem is not the colour
  // — it is that a failed load and an empty result look almost identical, so
  // the user cannot tell "nothing here yet" from "this did not load".
  //
  // ErrorState makes the distinction structural: a border, an alert icon, the
  // message, and a retry affordance. `onretry` is what separates it from
  // EmptyState at the call site — pass it when the data is refetchable.
  import Icon from "./Icon.svelte";

  let {
    title = "Something went wrong",
    message,
    actionLabel,
    onaction,
  }: {
    title?: string;
    /** The server/transport message. Shown verbatim — it is diagnostic text,
     *  and hiding it behind a generic string is what makes errors unreportable. */
    message?: string | null;
    actionLabel?: string;
    onaction?: () => void;
  } = $props();
</script>

<!-- role="alert" so a screen reader announces the failure as it appears,
     without stealing focus from whatever the user was doing. -->
<div class="flex items-start gap-3 rounded-lg border border-error/30 bg-error/10 p-4" role="alert">
  <Icon name="alert" class="w-4 h-4 text-error flex-shrink-0 mt-0.5" />
  <div class="min-w-0 flex-1">
    <p class="text-sm font-medium text-content">{title}</p>
    {#if message}
      <p class="text-xs text-muted mt-0.5 break-words">{message}</p>
    {/if}
  </div>
  {#if actionLabel && onaction}
    <button
      onclick={onaction}
      class="flex-shrink-0 px-2.5 py-1.5 bg-surface border border-line rounded-md text-xs text-content hover:bg-surface-hover transition-colors"
    >
      {actionLabel}
    </button>
  {/if}
</div>
