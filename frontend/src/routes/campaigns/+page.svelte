<script lang="ts">
  // v23-3: Campaign list page.
  //
  // Shows all campaigns as a grid of cards with progress bars, status
  // badges, and post counts. Clicking a card navigates to the detail
  // page. This is the entry point for campaign management — previously
  // campaigns were only accessible via the kanban filter.
  import { onMount, onDestroy } from 'svelte';
  import { campaignsApi, type Campaign } from '$lib/api/campaigns';
  import { goto } from '$app/navigation';
  import { realtime } from '$lib/stores/realtime';
  import { modals } from '$lib/stores/modals.svelte';
  import { toast } from '$lib/stores/toast';
  import Badge from '$lib/ui/Badge.svelte';
  import EmptyState from '$lib/ui/EmptyState.svelte';
  import PageHeader from '$lib/ui/PageHeader.svelte';
  import Skeleton from '$lib/ui/Skeleton.svelte';

  let campaigns = $state<Campaign[]>([]);
  let loading = $state(true);
  let statusFilter = $state<'all' | 'active' | 'paused' | 'archived' | 'completed'>('all');

  // Create-campaign inline modal state.
  let showCreateModal = $state(false);
  let newCampaignName = $state('');
  let creating = $state(false);

  async function load() {
    loading = true;
    const r = await campaignsApi.list();
    if (r.data) campaigns = r.data;
    loading = false;
  }

  onMount(() => {
    load();
    const events = ['campaign_created', 'campaign_updated', 'campaign_deleted', 'post_stage_changed', 'lagged'];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  let unsubscribers: (() => void)[] = [];
  onDestroy(() => unsubscribers.forEach(fn => fn()));

  let filteredCampaigns = $derived(
    statusFilter === 'all'
      ? campaigns
      : campaigns.filter(c => c.status === statusFilter)
  );

  function openCreateModal() {
    newCampaignName = '';
    showCreateModal = true;
  }

  async function confirmCreate() {
    const name = newCampaignName.trim();
    if (!name) return;
    creating = true;
    const r = await campaignsApi.create({ name, color: '#6366f1', status: 'active' });
    creating = false;
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else if (r.data) {
      toast('Campaign created', 'success');
      showCreateModal = false;
      goto(`/campaigns/${r.data.id}`);
    }
  }

  async function deleteCampaign(id: string, name: string) {
    const ok = await modals.areYouSure({
      title: `Delete "${name}"?`,
      message: 'The campaign will be archived. Posts in it will be unassigned but not deleted.',
      confirmLabel: 'Archive',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    const r = await campaignsApi.delete(id);
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign archived', 'success');
      load();
    }
  }

  // Progress percentage for the progress bar.
  function progressPct(c: Campaign): number {
    if (!c.progress_target || c.progress_target === 0) return 0;
    const current = c.post_count ?? 0;
    return Math.min(100, (current / c.progress_target) * 100);
  }

  // Days remaining until end_date.
  function daysRemaining(c: Campaign): number | null {
    if (!c.end_date) return null;
    const end = new Date(c.end_date);
    const now = new Date();
    const diff = Math.ceil((end.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
    return diff;
  }

  const statusVariant: Record<string, 'default' | 'success' | 'warning' | 'error' | 'info'> = {
    active: 'success',
    paused: 'warning',
    archived: 'default',
    completed: 'info',
  };
</script>

<div class="page-enter space-y-6">
  <PageHeader title="Campaigns" subtitle="Strategic content campaigns with goals and progress tracking">
    <button onclick={openCreateModal} class="px-4 py-2 bg-brand-500 hover:bg-brand-600 rounded-lg text-sm font-medium transition-colors">
      + New Campaign
    </button>
  </PageHeader>

  <!-- Status filter -->
  <div class="flex gap-2">
    {#each ['all', 'active', 'paused', 'archived', 'completed'] as s (s)}
      <button
        onclick={() => statusFilter = s as typeof statusFilter}
        class="px-3 py-1 text-xs rounded-md transition-colors {statusFilter === s ? 'bg-brand-500 text-white' : 'bg-surface-hover text-muted hover:text-content'}"
      >
        {s.charAt(0).toUpperCase() + s.slice(1)}
      </button>
    {/each}
  </div>

  {#if loading}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each [1, 2, 3, 4, 5, 6] as _}
        <Skeleton width="100%" height="180px" rounded="lg" />
      {/each}
    </div>
  {:else if filteredCampaigns.length === 0}
    <EmptyState
      icon="analytics"
      title="No campaigns yet"
      description="Create a campaign to group posts around a goal, track progress, and organize your content strategy."
      actionLabel="Create Campaign"
      onaction={openCreateModal}
    />
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each filteredCampaigns as c (c.id)}
        <div
          class="bg-surface border border-line rounded-lg p-5 cursor-pointer hover:border-line-hover transition-colors group"
          onclick={() => goto(`/campaigns/${c.id}`)}
          role="button"
          tabindex={0}
          onkeydown={(e) => { if (e.key === 'Enter') goto(`/campaigns/${c.id}`); }}
        >
          <div class="flex items-start justify-between mb-3">
            <div class="flex items-center gap-2">
              <span class="w-3 h-3 rounded-full" style="background: {c.color}"></span>
              <h3 class="font-medium text-content group-hover:text-brand-400 transition-colors">{c.name}</h3>
            </div>
            <Badge variant={statusVariant[c.status] || 'default'}>{c.status}</Badge>
          </div>

          {#if c.description}
            <p class="text-xs text-muted mb-3 line-clamp-2">{c.description}</p>
          {/if}

          <!-- Progress bar -->
          {#if c.progress_target}
            <div class="mb-3">
              <div class="flex justify-between text-xs text-muted mb-1">
                <span>{c.post_count ?? 0} / {c.progress_target} posts</span>
                <span>{Math.round(progressPct(c))}%</span>
              </div>
              <div class="bg-background-input rounded-full h-1.5 overflow-hidden">
                <div class="h-full rounded-full transition-all duration-500" style="width: {progressPct(c)}%; background: {c.color}"></div>
              </div>
            </div>
          {:else}
            <div class="text-xs text-muted mb-3">{c.post_count ?? 0} posts</div>
          {/if}

          <!-- Footer: dates + delete -->
          <div class="flex items-center justify-between text-[10px] text-muted-dark">
            <span>
              {#if c.start_date && c.end_date}
                {new Date(c.start_date).toLocaleDateString()} → {new Date(c.end_date).toLocaleDateString()}
              {:else if c.start_date}
                From {new Date(c.start_date).toLocaleDateString()}
              {:else if daysRemaining(c) !== null}
                {daysRemaining(c)} days left
              {/if}
            </span>
            <button
              onclick={(e) => { e.stopPropagation(); deleteCampaign(c.id, c.name); }}
              class="text-muted hover:text-error transition-colors opacity-0 group-hover:opacity-100"
              title="Archive campaign"
            >
              ✕
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Create campaign modal -->
{#if showCreateModal}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-md">
      <h3 class="text-lg font-semibold mb-4">Create Campaign</h3>
      <input
        type="text"
        bind:value={newCampaignName}
        onkeydown={(e) => { if (e.key === 'Enter') confirmCreate(); }}
        placeholder="Campaign name (e.g. Q4 Launch)"
        class="w-full mb-4 px-3 py-2 bg-surface-hover border border-line rounded text-sm focus:border-brand-500 outline-none"
        autofocus
      />
      <div class="flex gap-3 justify-end">
        <button onclick={() => showCreateModal = false} class="px-4 py-2 text-sm text-muted hover:text-content">Cancel</button>
        <button onclick={confirmCreate} disabled={creating || !newCampaignName.trim()} class="px-4 py-2 text-sm bg-brand-500 hover:bg-brand-600 disabled:opacity-50 rounded transition-colors">
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
{/if}

