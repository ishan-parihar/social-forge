<script lang="ts">
  // v23-3: Campaign detail page.
  //
  // Shows a single campaign with Overview/Posts/Settings tabs.
  // - Overview: progress bar, KPIs, dates, description, goal.
  // - Posts: list of all posts in the campaign with state filter.
  // - Settings: edit form for all campaign fields.
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { campaignsApi, type Campaign } from '$lib/api/campaigns';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { goto } from '$app/navigation';
  import { realtime } from '$lib/stores/realtime';
  import { modals } from '$lib/stores/modals.svelte';
  import { toast } from '$lib/stores/toast';
  import { providerIcon, providerColor } from '$lib/providers';
  import Badge from '$lib/ui/Badge.svelte';
  import EmptyState from '$lib/ui/EmptyState.svelte';
  import PageHeader from '$lib/ui/PageHeader.svelte';
  import Skeleton from '$lib/ui/Skeleton.svelte';
  import Tabs from '$lib/ui/Tabs.svelte';

  let campaign = $state<Campaign | null>(null);
  let posts = $state<PostSummary[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let activeTab = $state<'overview' | 'posts' | 'settings'>('overview');

  // Edit form state.
  let formName = $state('');
  let formDescription = $state('');
  let formColor = $state('#6366f1');
  let formStartDate = $state('');
  let formEndDate = $state('');
  let formGoal = $state('');
  let formStatus = $state<'active' | 'paused' | 'archived' | 'completed'>('active');
  let formProgressMetric = $state<'posts' | 'engagement' | 'reach' | 'followers' | 'custom' | ''>('');
  let formProgressTarget = $state<number | ''>('');

  let postId = $derived($page.params.id);

  async function load() {
    loading = true;
    const [campRes, postsRes] = await Promise.all([
      campaignsApi.list(),
      postsApi.list({ limit: 200 }),
    ]);
    if (campRes.data) {
      campaign = campRes.data.find(c => c.id === postId) ?? null;
      if (campaign) {
        formName = campaign.name;
        formDescription = campaign.description ?? '';
        formColor = campaign.color;
        formStartDate = campaign.start_date ?? '';
        formEndDate = campaign.end_date ?? '';
        formGoal = campaign.goal ?? '';
        formStatus = campaign.status;
        formProgressMetric = campaign.progress_metric ?? '';
        formProgressTarget = campaign.progress_target ?? '';
      }
    }
    if (postsRes.data) {
      posts = postsRes.data.posts.filter(p => p.campaign_id === postId);
    }
    loading = false;
  }

  onMount(() => {
    load();
    const events = ['campaign_updated', 'post_stage_changed', 'post_created', 'post_deleted', 'lagged'];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  let unsubscribers: (() => void)[] = [];
  onDestroy(() => unsubscribers.forEach(fn => fn()));

  async function saveSettings() {
    if (!campaign) return;
    saving = true;
    const r = await campaignsApi.update(campaign.id, {
      name: formName,
      description: formDescription || undefined,
      color: formColor,
      start_date: formStartDate || undefined,
      end_date: formEndDate || undefined,
      goal: formGoal || undefined,
      status: formStatus,
      progress_metric: formProgressMetric || undefined,
      progress_target: typeof formProgressTarget === 'number' ? formProgressTarget : undefined,
    });
    saving = false;
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign updated', 'success');
      load();
    }
  }

  async function deleteCampaign() {
    if (!campaign) return;
    const ok = await modals.areYouSure({
      title: `Delete "${campaign.name}"?`,
      message: 'The campaign will be archived. Posts in it will be unassigned but not deleted.',
      confirmLabel: 'Archive',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    const r = await campaignsApi.delete(campaign.id);
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign archived', 'success');
      goto('/campaigns');
    }
  }

  function progressPct(): number {
    if (!campaign?.progress_target || campaign.progress_target === 0) return 0;
    const current = campaign.post_count ?? 0;
    return Math.min(100, (current / campaign.progress_target) * 100);
  }

  // Group posts by state for the Posts tab.
  let postsByState = $derived({
    idea: posts.filter(p => p.state === 'idea'),
    draft: posts.filter(p => p.state === 'draft' || p.state === 'error'),
    queued: posts.filter(p => p.state === 'queued'),
    published: posts.filter(p => p.state === 'published'),
  });

  const statusVariant: Record<string, 'default' | 'success' | 'warning' | 'error' | 'info'> = {
    active: 'success',
    paused: 'warning',
    archived: 'default',
    completed: 'info',
  };

  const tabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'posts', label: `Posts (${posts.length})` },
    { id: 'settings', label: 'Settings' },
  ];
</script>

<div class="page-enter space-y-6">
  {#if loading}
    <Skeleton width="100%" height="120px" rounded="lg" />
    <Skeleton width="100%" height="300px" rounded="lg" />
  {:else if !campaign}
    <EmptyState
      icon="analytics"
      title="Campaign not found"
      description="This campaign may have been archived or deleted."
      actionLabel="Back to Campaigns"
      onaction={() => goto('/campaigns')}
    />
  {:else}
    <PageHeader title={campaign.name} subtitle={campaign.description || 'No description'}>
      <Badge variant={statusVariant[campaign.status] || 'default'}>{campaign.status}</Badge>
      <button onclick={() => goto('/campaigns')} class="px-3 py-2 text-sm text-muted hover:text-content">
        ← Back
      </button>
    </PageHeader>

    <Tabs tabs={tabs} bind:value={activeTab} />

    {#if activeTab === 'overview'}
      <!-- Overview tab -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <!-- Progress card -->
        <div class="bg-surface border border-line rounded-lg p-5">
          <h3 class="font-medium text-sm mb-4">Progress</h3>
          {#if campaign.progress_target}
            <div class="mb-4">
              <div class="flex justify-between text-sm mb-2">
                <span class="text-muted">{campaign.progress_metric || 'posts'}: {campaign.post_count ?? 0} / {campaign.progress_target}</span>
                <span class="font-medium">{Math.round(progressPct())}%</span>
              </div>
              <div class="bg-background-input rounded-full h-3 overflow-hidden">
                <div class="h-full rounded-full transition-all duration-500" style="width: {progressPct()}%; background: {campaign.color}"></div>
              </div>
            </div>
          {:else}
            <p class="text-sm text-muted">{campaign.post_count ?? 0} posts in this campaign. Set a progress target in Settings to track a goal.</p>
          {/if}

          <div class="grid grid-cols-2 gap-4 mt-4 pt-4 border-t border-line">
            <div>
              <div class="text-[10px] text-muted-dark uppercase tracking-wider">Start</div>
              <div class="text-sm">{campaign.start_date ? new Date(campaign.start_date).toLocaleDateString() : '—'}</div>
            </div>
            <div>
              <div class="text-[10px] text-muted-dark uppercase tracking-wider">End</div>
              <div class="text-sm">{campaign.end_date ? new Date(campaign.end_date).toLocaleDateString() : '—'}</div>
            </div>
          </div>

          {#if campaign.goal}
            <div class="mt-4 pt-4 border-t border-line">
              <div class="text-[10px] text-muted-dark uppercase tracking-wider mb-1">Goal</div>
              <div class="text-sm">{campaign.goal}</div>
            </div>
          {/if}
        </div>

        <!-- Post counts by state -->
        <div class="bg-surface border border-line rounded-lg p-5">
          <h3 class="font-medium text-sm mb-4">Posts by State</h3>
          <div class="space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted">💡 Ideas</span>
              <Badge variant="default">{postsByState.idea.length}</Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted">📝 Drafts</span>
              <Badge variant="info">{postsByState.draft.length}</Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted">📅 Scheduled</span>
              <Badge variant="warning">{postsByState.queued.length}</Badge>
            </div>
            <div class="flex items-center justify-between">
              <span class="text-sm text-muted">✅ Published</span>
              <Badge variant="success">{postsByState.published.length}</Badge>
            </div>
          </div>
        </div>
      </div>

    {:else if activeTab === 'posts'}
      <!-- Posts tab -->
      {#if posts.length === 0}
        <EmptyState
          icon="post"
          title="No posts in this campaign"
          description="Assign posts to this campaign from the kanban board or post composer."
          actionLabel="Go to Kanban"
          onaction={() => goto('/kanban')}
        />
      {:else}
        <div class="bg-surface border border-line rounded-lg overflow-hidden">
          {#each posts as post (post.id)}
            <a href="/posts/{post.id}" class="flex items-center gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors">
              <span class="text-xs" style="color: {providerColor(post.integration_name?.toLowerCase()?.split(/\s+/)[0] || '')}">
                {providerIcon(post.integration_name?.toLowerCase()?.split(/\s+/)[0] || '')}
              </span>
              <span class="flex-1 text-sm truncate text-content-secondary">{post.content || post.title || '(no content)'}</span>
              <Badge state={post.state as 'draft' | 'queued' | 'published' | 'error' | 'idea'} />
            </a>
          {/each}
        </div>
      {/if}

    {:else if activeTab === 'settings'}
      <!-- Settings tab -->
      <div class="bg-surface border border-line rounded-lg p-5 space-y-4 max-w-2xl">
        <div>
          <label class="block text-sm text-muted mb-1">Name</label>
          <input type="text" bind:value={formName} class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none" />
        </div>
        <div>
          <label class="block text-sm text-muted mb-1">Description</label>
          <textarea bind:value={formDescription} rows="2" class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none"></textarea>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm text-muted mb-1">Color</label>
            <div class="flex items-center gap-2">
              <input type="color" bind:value={formColor} class="w-10 h-9 rounded cursor-pointer bg-transparent border border-line" />
              <input type="text" bind:value={formColor} class="flex-1 px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none" />
            </div>
          </div>
          <div>
            <label class="block text-sm text-muted mb-1">Status</label>
            <select bind:value={formStatus} class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none">
              <option value="active">Active</option>
              <option value="paused">Paused</option>
              <option value="archived">Archived</option>
              <option value="completed">Completed</option>
            </select>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm text-muted mb-1">Start Date</label>
            <input type="date" bind:value={formStartDate} class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none" />
          </div>
          <div>
            <label class="block text-sm text-muted mb-1">End Date</label>
            <input type="date" bind:value={formEndDate} class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none" />
          </div>
        </div>
        <div>
          <label class="block text-sm text-muted mb-1">Goal (free text)</label>
          <input type="text" bind:value={formGoal} placeholder="e.g. 1000 new followers" class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none" />
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="block text-sm text-muted mb-1">Progress Metric</label>
            <select bind:value={formProgressMetric} class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none">
              <option value="">None</option>
              <option value="posts">Posts</option>
              <option value="engagement">Engagement</option>
              <option value="reach">Reach</option>
              <option value="followers">Followers</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div>
            <label class="block text-sm text-muted mb-1">Progress Target</label>
            <input type="number" bind:value={formProgressTarget} placeholder="e.g. 20" class="w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-brand-500 outline-none" />
          </div>
        </div>
        <div class="flex items-center justify-between pt-4 border-t border-line">
          <button onclick={deleteCampaign} class="text-sm text-error hover:text-error/80 transition-colors">
            Archive Campaign
          </button>
          <button onclick={saveSettings} disabled={saving || !formName.trim()} class="px-4 py-2 text-sm bg-brand-500 hover:bg-brand-600 disabled:opacity-50 rounded transition-colors">
            {saving ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </div>
    {/if}
  {/if}
</div>
