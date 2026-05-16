<script lang="ts">
  import Button from '$lib/ui/Button.svelte';
  import Badge from '$lib/ui/Badge.svelte';
  import { rssApi, type RssFeed } from '$lib/api/rss';

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
    if (!confirm('Delete this RSS feed?')) return;
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

<div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-2 min-w-0">
      <h4 class="text-sm font-semibold truncate">{feed.title || 'Untitled Feed'}</h4>
      <Badge state={feed.enabled ? 'published' : 'draft'} />
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

  <div class="text-xs text-[#6b7280] space-y-1">
    <div class="truncate" title={feed.feed_url}>URL: {feed.feed_url}</div>
    <div>Interval: every {feed.poll_interval_min} min</div>
    <div>Last polled: {formatDate(feed.last_polled_at)}</div>
    <div>{itemsCount} items</div>
  </div>

  {#if error}
    <div class="text-xs text-red-400">{error}</div>
  {/if}
</div>
