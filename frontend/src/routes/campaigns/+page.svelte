<script lang="ts">
  // v23-3: Campaign list page.
  //
  // Shows all campaigns as a grid of cards with progress bars, status
  // badges, and post counts. Clicking a card navigates to the detail
  // page. This is the entry point for campaign management - previously
  // campaigns were only accessible via the kanban filter.
  import { onMount, onDestroy } from 'svelte';
  import { focusTrap } from '$lib/ui/focus-trap';
  import { campaignsApi, formatDateOnly, type Campaign } from '$lib/api/campaigns';
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
  let nameInput = $state<HTMLInputElement>();
  $effect(() => {
    nameInput?.focus();
  });

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

  async function archiveCampaign(id: string, name: string) {
    const ok = await modals.areYouSure({
      title: `Archive "${name}"?`,
      message: 'The campaign will be archived. Its posts stay exactly as they are.',
      confirmLabel: 'Archive',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    // Status, not DELETE: the DELETE route also sets deleted_at, which hides the
    // campaign from every list, so the Archived filter could never show it.
    const r = await campaignsApi.update(id, { status: 'archived' });
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign archived', 'success');
      load();
    }
  }

  // Progress percentage for the progress bar.
  //
  // The bar measures `progress_metric` - the metric the founder chose.
  // It used to always measure the POST count, so a campaign with
  // `progress_metric = "engagement"` and a target of 1000 measured 4
  // posts against it: "4 / 1000 posts" on a bar that read 0%, and the
  // detail page labelled the same numbers "engagement".
  //
  // A metric with no measured value (reach, followers, custom) has no
  // number to show, so the bar reports nothing rather than a wrong
  // number.
  function progressValue(c: Campaign): { current: number | null; unit: string } {
    switch (c.progress_metric) {
      case 'engagement':
        return { current: c.engagement_total ?? null, unit: 'engagement' };
      case 'reach':
      case 'followers':
      case 'custom':
        // Not measured yet. Showing the post count under these labels
        // would be a different number wearing the metric's name.
        return { current: null, unit: c.progress_metric };
      case 'posts':
      default:
        return { current: c.post_count ?? 0, unit: 'posts' };
    }
  }

  function progressPct(c: Campaign): number {
    if (!c.progress_target || c.progress_target === 0) return 0;
    const { current } = progressValue(c);
    if (current === null) return 0;
    return Math.min(100, (current / c.progress_target) * 100);
  }

  // Days remaining until end_date.
  //
  // `end_date` is a CALENDAR DATE ("2026-09-30"). `new Date("2026-09-30")`
  // parses it as UTC midnight, so in a negative-offset zone the
  // difference was a day short - and the final day read as "-1 days
  // left". Both dates are read as calendar days (UTC midnight) and the
  // result is floored at zero.
  function daysRemaining(c: Campaign): number | null {
    if (!c.end_date) return null;
    const end = Date.parse(`${c.end_date}T00:00:00Z`);
    if (Number.isNaN(end)) return null;
    const now = new Date();
    const today = Date.UTC(
      now.getFullYear(),
      now.getMonth(),
      now.getDate(),
    );
    return Math.max(0, Math.round((end - today) / (1000 * 60 * 60 * 24)));
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
    <button onclick={openCreateModal} class="px-4 py-2 min-h-11 whitespace-nowrap bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg text-sm font-medium transition-colors">
      + New Campaign
    </button>
  </PageHeader>

  <!-- Status filter -->
  <div class="flex gap-2">
    {#each ['all', 'active', 'paused', 'archived', 'completed'] as s (s)}
      <button
        onclick={() => statusFilter = s as typeof statusFilter}
        class="px-3 py-1 text-xs rounded-md transition-colors min-h-11 {statusFilter === s ? 'bg-accent-fill text-accent-fg' : 'bg-surface-hover text-muted hover:text-content'} min-w-11 inline-flex items-center justify-center"
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
    <!--
      A filtered list that comes back empty is a DIFFERENT state from
      having no campaigns at all. It used to say "No campaigns yet /
      Create Campaign" for both, so filtering three active campaigns
      down to "Paused" (none) told the user they had no campaigns.
    -->
    {#if campaigns.length > 0}
      <EmptyState
        icon="analytics"
        title="No {statusFilter} campaigns"
        description="None of your {campaigns.length} campaigns are {statusFilter}."
        actionLabel="Show all"
        onaction={() => (statusFilter = 'all')}
      />
    {:else}
      <EmptyState
        icon="analytics"
        title="No campaigns yet"
        description="Create a campaign to group posts around a goal, track progress, and organize your content strategy."
        actionLabel="Create Campaign"
        onaction={openCreateModal}
      />
    {/if}
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {#each filteredCampaigns as c (c.id)}
        <div class="relative bg-surface border border-line rounded-lg p-5 hover:border-line-hover transition-colors group">
          <div class="flex items-start justify-between mb-3">
            <div class="flex items-center gap-2">
              <span class="w-3 h-3 rounded-full" style="background: {c.color}"></span>
              <h2 class="font-medium">
                <!-- Stretched link: the whole card is the link, and the ✕ sits above it (z-10). -->
                <a href="/campaigns/{c.id}" class="text-content group-hover:text-accent transition-colors no-underline min-h-11 inline-flex items-center underline after:absolute after:inset-0 after:content-['']">{c.name}</a>
              </h2>
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
                <span>
                  {#if progressValue(c).current === null}
                    {c.progress_target} {progressValue(c).unit} goal
                  {:else}
                    {progressValue(c).current} / {c.progress_target} {progressValue(c).unit}
                  {/if}
                </span>
                <!--
                  A metric with no data source gets no percentage and
                  no bar. "Reach", "followers" and "custom" are accepted
                  by the server and offered in the Settings tab, but
                  nothing measures them - so a 0% bar against a target
                  reads as "you have achieved nothing", which is a
                  claim about the campaign rather than about the metric.
                -->
                {#if progressValue(c).current !== null}
                  <span>{Math.round(progressPct(c))}%</span>
                {/if}
              </div>
              {#if progressValue(c).current === null}
                <div class="h-1.5 rounded-full bg-background-input flex items-center justify-center">
                  <span class="text-[10px] text-faint">Not measured</span>
                </div>
              {:else}
              <div class="bg-background-input rounded-full h-1.5 overflow-hidden">
                <div class="h-full rounded-full transition-all duration-500" style="width: {progressPct(c)}%; background: {c.color}"></div>
              </div>
              {/if}
            </div>
          {:else}
            <div class="text-xs text-muted mb-3">
              {c.post_count ?? 0} posts
              {#if (c.failed_count ?? 0) > 0}
                <span class="text-error">· {c.failed_count} failed</span>
              {/if}
            </div>
          {/if}

          <!-- Footer: dates + delete -->
          <div class="flex items-center justify-between text-[10px] text-faint">
            <span>
              {#if c.start_date && c.end_date}
                {formatDateOnly(c.start_date)} → {formatDateOnly(c.end_date)}
              {:else if c.start_date}
                From {formatDateOnly(c.start_date)}
              {:else if daysRemaining(c) !== null}
                {daysRemaining(c)} days left
              {/if}
            </span>
            <button
              onclick={(e) => { e.stopPropagation(); archiveCampaign(c.id, c.name); }}
              class="relative z-10 text-muted hover:text-error transition-colors min-h-11 min-w-11 inline-flex items-center justify-center opacity-0 group-hover:opacity-100"
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
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <!--
  The scrim is a separate, aria-hidden BUTTON and the panel is its
  SIBLING.

  The wrapper used to carry `aria-hidden="true"`, which made the whole
  dialog invisible to a screen reader, and the shared focus trap skips
  anything in an aria-hidden subtree - so it found no focusable child
  and the focus never entered the dialog. See AGENTS.md 9.39.

  The scrim carries the aria-hidden and the tabindex="-1": a screen
  reader should not offer a nameless "close" button, and Escape
  already closes the dialog.
-->
    <button
      type="button"
      class="absolute inset-0 bg-overlay cursor-default"
      aria-hidden="true"
      tabindex="-1"
      onclick={() => { showCreateModal = false }}
    ></button>
<div class="relative bg-background-input border border-line rounded-xl p-6 w-full max-w-md"
      role="dialog"
      aria-modal="true"
      aria-labelledby="create-campaign-title"
      tabindex="-1"
      onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); showCreateModal = false; } }}
      use:focusTrap
    >
      <h2 id="create-campaign-title" class="text-lg font-semibold mb-4">Create Campaign</h2>
      <input
        type="text"
        bind:value={newCampaignName}
        onkeydown={(e) => { if (e.key === 'Enter') confirmCreate(); }}
        placeholder="Campaign name (e.g. Q4 Launch)"
        class="w-full mb-4 px-3 py-2 bg-surface-hover border border-line rounded text-sm focus:border-accent outline-none"
        bind:this={nameInput}
      />
      <div class="flex gap-3 justify-end">
        <button onclick={() => showCreateModal = false} class="px-4 py-2 text-sm text-muted hover:text-content min-h-11">Cancel</button>
        <button onclick={confirmCreate} disabled={creating || !newCampaignName.trim()} class="px-4 py-2 text-sm bg-accent-fill hover:bg-accent-fill-hover text-accent-fg disabled:opacity-50 rounded transition-colors min-h-11">
          {creating ? 'Creating...' : 'Create'}
        </button>
      </div>
    </div>
  </div>
{/if}

