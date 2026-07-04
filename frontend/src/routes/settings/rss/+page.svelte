<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/ui/Button.svelte';
  import Spinner from '$lib/ui/Spinner.svelte';
  import Modal from '$lib/ui/Modal.svelte';
  import { rssApi, type RssFeed, type RssFeedItem } from '$lib/api/rss';
  import RssFeedForm from '$lib/rss/RssFeedForm.svelte';
  import RssFeedCard from '$lib/rss/RssFeedCard.svelte';
  import { toast } from "$lib/stores/toast";

  let feeds = $state<RssFeed[]>([]);
  let loading = $state(true);
  let error = $state('');
  let showForm = $state(false);
  let showItemsFor = $state<string | null>(null);
  let items = $state<RssFeedItem[]>([]);
  let itemsLoading = $state(false);
  let itemsError = $state('');

  onMount(loadFeeds);

  async function loadFeeds() {
    loading = true;
    error = '';
    try {
      const r = await rssApi.list();
      if (r.error) {
        error = r.error;
      } else if (r.data) {
        feeds = r.data;
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to load feeds';
    } finally {
      loading = false;
    }
  }

  function handleFormSuccess() {
    showForm = false;
    loadFeeds();
  }

  function handleToggle(_id: string, _enabled: boolean) {
    loadFeeds();
  }

  function handleDelete(_id: string) {
    loadFeeds();
  }

  function handlePoll(_id: string) {
    loadFeeds();
  }

  async function handleViewItems(feedId: string) {
    showItemsFor = feedId;
    itemsLoading = true;
    itemsError = '';
    items = [];
    try {
      const r = await rssApi.listItems(feedId);
      if (r.error) {
        itemsError = r.error;
      } else if (r.data) {
        items = r.data;
      }
    } catch (e: unknown) {
      itemsError = e instanceof Error ? e.message : 'Failed to load items';
    } finally {
      itemsLoading = false;
    }
  }

  async function handleImport(feedId: string, guid: string) {
    try {
      const r = await rssApi.importItem(feedId, guid);
      if (r.error) {
        alert(`Import failed: ${r.error}`);
      } else {
        alert(`Item imported as post: ${r.data?.post_id}`);
        // Refresh items to show updated status
        handleViewItems(feedId);
      }
    } catch (e: unknown) {
      alert(`Import failed: ${e instanceof Error ? e.message : 'Unknown error'}`);
    }
  }

  function formatDate(d: string | null) {
    if (!d) return 'Unknown';
    return new Date(d).toLocaleString();
  }
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">RSS Autopost</h2>
      <p class="text-sm text-[#6b7280] mb-6">Automatically fetch and publish content from RSS feeds to your connected channels.</p>
    </div>
    <Button onclick={() => (showForm = !showForm)}>
      {showForm ? 'Cancel' : 'Add Feed'}
    </Button>
  </div>

  {#if showForm}
    <RssFeedForm onSuccess={handleFormSuccess} />
  {/if}

  {#if loading}
    <div class="flex justify-center py-12">
      <Spinner size="lg" />
    </div>
  {:else if error}
    <div class="bg-[#131720] border border-red-500/30 rounded-xl p-5 text-sm text-red-400">
      {error}
      <button onclick={loadFeeds} class="ml-2 underline">Retry</button>
    </div>
  {:else if feeds.length === 0}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-8 text-center">
      <p class="text-[#6b7280] text-sm">No RSS feeds configured. Add a feed to automatically post new content.</p>
    </div>
  {:else}
    <div class="page-enter space-y-3">
      {#each feeds as feed (feed.id)}
        <RssFeedCard
          {feed}
          onToggle={handleToggle}
          onDelete={handleDelete}
          onPoll={handlePoll}
          onViewItems={handleViewItems}
          itemsCount={0}
        />
      {/each}
    </div>
  {/if}
</div>

<Modal open={showItemsFor !== null} title="Feed Items" onclose={() => (showItemsFor = null)}>
  {#if itemsLoading}
    <div class="flex justify-center py-8">
      <Spinner />
    </div>
  {:else if itemsError}
    <div class="text-sm text-red-400">{itemsError}</div>
  {:else if items.length === 0}
    <p class="text-sm text-[#6b7280]">No items found for this feed.</p>
  {:else}
    <div class="page-enter space-y-2 max-h-96 overflow-y-auto">
      {#each items as item (item.guid)}
        <div class="bg-[#0b0e14] border border-[#1e2435] rounded-lg p-3 space-y-1">
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0">
              <p class="text-sm font-medium truncate">{item.title}</p>
              <p class="text-xs text-[#6b7280] truncate" title={item.url}>{item.url}</p>
              <p class="text-xs text-[#6b7280]">Published: {formatDate(item.published_at)}</p>
            </div>
            <div class="flex-shrink-0">
              {#if item.is_imported}
                <span class="text-xs text-green-400">Imported</span>
              {:else}
                <Button
                  size="sm"
                  variant="secondary"
                  onclick={() => handleImport(showItemsFor!, item.guid)}
                >
                  Import
                </Button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</Modal>
