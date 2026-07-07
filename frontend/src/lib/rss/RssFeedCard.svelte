<script lang="ts">
  import Button from '$lib/ui/Button.svelte';
  import { rssApi, type RssFeed } from '$lib/api/rss';
  import { modals } from '$lib/stores/modals.svelte';

  let {
    feed,
    onToggle,
    onDelete,
    onPoll,
    onViewItems,
    itemsCount = 0,
  }: {
    feed: RssFeed;
    onToggle?: (id: string, enabled: boolean) => void;
    onDelete?: (id: string) => void;
    onPoll?: (id: string) => void;
    onViewItems?: (id: string) => void;
    itemsCount?: number;
  } = $props();

  let toggling = $state(false);
  let deleting = $state(false);
  let polling = $state(false);
  let error = $state('');

  function formatDate(d: string | null) {
    if (!d) return 'Never';
    return new Date(d).toLocaleString();
  }

  async function handleToggle() {
    toggling = true;
    error = '';
    try {
      const r = await rssApi.toggle(feed.id);
      if (r.error) {
        error = r.error;
      } else if (r.data) {
        if (onToggle) onToggle(feed.id, r.data.enabled);
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to toggle';
    } finally {
      toggling = false;
    }
  }

  async function handleDelete() {
    if (!(await modals.areYouSure({
      title: 'Delete this RSS feed?',
      message: 'The feed and its cached items will be permanently deleted. Posts already imported from it are unaffected.',
      confirmLabel: 'Delete',
      cancelLabel: 'Cancel',
      danger: true,
    }))) return;
    deleting = true;
    error = '';
    try {
      const r = await rssApi.delete(feed.id);
      if (r.error) {
        error = r.error;
      } else {
        if (onDelete) onDelete(feed.id);
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to delete';
    } finally {
      deleting = false;
    }
  }

  async function handlePoll() {
    polling = true;
    error = '';
    try {
      const r = await rssApi.poll(feed.id);
      if (r.error) {
        error = r.error;
      } else {
        if (r.data?.new_items !== undefined) {
          alert(`Polled successfully. ${r.data.new_items} new item(s) found.`);
        }
        if (onPoll) onPoll(feed.id);
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to poll';
    } finally {
      polling = false;
    }
  }
</script>

<div class="bg-surface border border-line rounded-xl p-5 space-y-3">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-2 min-w-0">
      <h4 class="text-sm font-semibold truncate">{feed.title || 'Untitled Feed'}</h4>
      <!-- R-6: Use an explicit Enabled/Disabled pill instead of reusing the
           post-state Badge component (which would show "Published"/"Draft"
           — semantically wrong for an RSS feed that has no publication state). -->
      <span class="px-2 py-0.5 rounded text-[10px] font-medium border {feed.enabled
        ? 'bg-emerald-500/10 border-emerald-500/30 text-emerald-400'
        : 'bg-gray-500/10 border-gray-500/30 text-gray-400'}">
        {feed.enabled ? 'Enabled' : 'Disabled'}
      </span>
    </div>
    <div class="flex items-center gap-2 flex-shrink-0">
      <Button size="sm" variant="ghost" disabled={polling} onclick={handlePoll}>
        {polling ? '...' : 'Poll'}
      </Button>
      <Button size="sm" variant="ghost" disabled={toggling} onclick={handleToggle}>
        {feed.enabled ? 'Disable' : 'Enable'}
      </Button>
      <Button size="sm" variant="ghost" onclick={() => onViewItems?.(feed.id)}>
        Items
      </Button>
      <Button size="sm" variant="danger" disabled={deleting} onclick={handleDelete}>
        {deleting ? '...' : 'Delete'}
      </Button>
    </div>
  </div>

  <div class="text-xs text-muted space-y-1">
    <div class="truncate" title={feed.feed_url}>URL: {feed.feed_url}</div>
    <div>Interval: every {feed.poll_interval_min} min</div>
    <div>Last polled: {formatDate(feed.last_polled_at)}</div>
    <div>{itemsCount} items</div>
  </div>

  {#if error}
    <div class="text-xs text-error">{error}</div>
  {/if}
</div>
