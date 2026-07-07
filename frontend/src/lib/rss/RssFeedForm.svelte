<script lang="ts">
  import Button from '$lib/ui/Button.svelte';
  import { rssApi } from '$lib/api/rss';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { onMount } from 'svelte';

  let { onSuccess }: { onSuccess?: () => void } = $props();

  let saving = $state(false);
  let error = $state('');
  let integrations = $state<Integration[]>([]);
  let loadingIntegrations = $state(true);

  let feedUrl = $state('');
  let integrationId = $state('');
  let title = $state('');
  let useAiSummary = $state(false);

  onMount(async () => {
    const r = await integrationsApi.list();
    if (r.data?.integrations) {
      integrations = r.data.integrations;
    }
    loadingIntegrations = false;
  });

  async function handleSubmit(e: Event) {
    e.preventDefault();
    if (!feedUrl.trim()) { error = 'Feed URL is required'; return; }
    if (!integrationId) { error = 'Integration is required'; return; }

    saving = true;
    error = '';
    try {
      const r = await rssApi.create({
        feed_url: feedUrl.trim(),
        integration_id: integrationId,
        title: title.trim() || undefined,
        use_ai_summary: useAiSummary || undefined,
      });
      if (r.error) {
        error = r.error;
      } else {
        feedUrl = '';
        integrationId = '';
        title = '';
        useAiSummary = false;
        if (onSuccess) onSuccess();
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to create feed';
    } finally {
      saving = false;
    }
  }
</script>

<div class="bg-surface border border-line rounded-xl p-5">
  <h3 class="text-sm font-semibold mb-4">Add RSS Feed</h3>

  <form onsubmit={handleSubmit} class="space-y-4">
    <div>
      <label for="feed-url" class="block text-xs text-muted mb-1">Feed URL</label>
      <input
        id="feed-url"
        type="url"
        bind:value={feedUrl}
        placeholder="https://example.com/rss"
        class="w-full bg-background border border-line rounded-lg px-3 py-2 text-sm text-content placeholder:text-muted-dark focus:outline-none focus:border-indigo-500"
        required
      />
    </div>

    <div>
      <label for="integration" class="block text-xs text-muted mb-1">Integration</label>
      {#if loadingIntegrations}
        <div class="text-xs text-muted">Loading integrations...</div>
      {:else if integrations.length === 0}
        <div class="text-xs text-warning">No integrations found. Connect a channel first.</div>
      {:else}
        <select
          id="integration"
          bind:value={integrationId}
          class="w-full bg-background border border-line rounded-lg px-3 py-2 text-sm text-content focus:outline-none focus:border-indigo-500"
          required
        >
          <option value="" disabled>Select integration</option>
          {#each integrations as int}
            <option value={int.id}>
              {int.profile_name || int.provider_name} ({int.provider_identifier})
            </option>
          {/each}
        </select>
      {/if}
    </div>

    <div>
      <label for="feed-title" class="block text-xs text-muted mb-1">Title (optional)</label>
      <input
        id="feed-title"
        type="text"
        bind:value={title}
        placeholder="My RSS Feed"
        class="w-full bg-background border border-line rounded-lg px-3 py-2 text-sm text-content placeholder:text-muted-dark focus:outline-none focus:border-indigo-500"
      />
    </div>

    <label class="flex items-center gap-2 cursor-pointer">
      <input type="checkbox" bind:checked={useAiSummary} class="rounded bg-background border-line" />
      <span class="text-xs text-muted">Use AI summary (experimental)</span>
    </label>

    {#if error}
      <div class="text-xs text-red-400">{error}</div>
    {/if}

    <button type="submit" disabled={saving || loadingIntegrations}
      class="px-4 py-2 text-sm font-medium bg-brand-500 hover:bg-brand-600 text-white rounded-lg disabled:opacity-50 transition-colors">
      {saving ? 'Adding...' : 'Add Feed'}
    </button>
  </form>
</div>
