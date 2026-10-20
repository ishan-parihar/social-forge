<script lang="ts">
  // v23-3: Campaign detail page.
  //
  // Shows a single campaign with Overview/Posts/Settings tabs.
  // - Overview: progress bar, KPIs, dates, description, goal.
  // - Posts: list of all posts in the campaign with state filter.
  // - Settings: edit form for all campaign fields.
  import { onMount, onDestroy } from 'svelte';
  import { page } from '$app/stores';
  import { campaignsApi, formatDateOnly, type Campaign } from '$lib/api/campaigns';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { goto } from '$app/navigation';
  import { realtime } from '$lib/stores/realtime';
  import { modals } from '$lib/stores/modals.svelte';
  import { toast } from '$lib/stores/toast';
  import { providerIcon, providerColor, providerInkOnCard } from '$lib/providers';
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
  // The audience persona, the content pillars, the KPI targets and the
  // budget.
  //
  // The server has persisted all four since the campaign feature
  // landed, and every screen's type declares them - but NO screen
  // rendered or edited them. So a founder who set a persona through
  // the API opened the campaign, saw nothing, and concluded it had
  // never been saved. And "Save Changes" omitted them, which means
  // "keep", so the round-trip was a no-op that reported success.
  //
  // They are edited as TEXT here - the stored shapes are JSON, and a
  // free-text box that holds JSON is honest about what the app
  // actually does with them.
  let formPersona = $state('');
  let formPillars = $state('');
  let formKpiTargets = $state('');
  let formBudget = $state('');

  let postId = $derived($page.params.id);

  async function load() {
    loading = true;
    // The campaign's OWN posts, fetched by the server's campaign filter.
    //
    // The page used to fetch the newest 200 posts of the WHOLE
    // account and filter them in the browser by `campaign_id`. So a
    // campaign with 250 posts showed the progress bar reading
    // "250 / 20 - 100%" (the server's count over all its posts) next
    // to a "Posts by State" panel that listed only the few of the
    // newest 200 that were filed under it. Two panels on one screen
    // describing different populations, neither labelled.
    //
    // The filter is the server's, so both numbers now come from the
    // same population.
    const [campRes, postsRes] = await Promise.all([
      campaignsApi.list(),
      postsApi.list({ limit: 200, campaign_id: postId }),
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
        formPersona = campaign.audience_persona
          ? JSON.stringify(campaign.audience_persona, null, 2)
          : '';
        formPillars = campaign.content_pillars
          ? JSON.stringify(campaign.content_pillars, null, 2)
          : '';
        formKpiTargets = campaign.kpi_targets
          ? JSON.stringify(campaign.kpi_targets, null, 2)
          : '';
        formBudget = campaign.budget_cents != null ? String(campaign.budget_cents) : '';
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

  /**
   * Parse a JSON field for the campaign settings.
   *
   * An empty box means "clear it" (`null`). A malformed box means
   * "leave the stored value alone" AND a message - a silent drop
   * would leave the old value in place and the founder would think
   * their edit had saved.
   */
  function parseJsonField(text: string, label: string): { ok: true; value: unknown } | { ok: false } {
    const trimmed = text.trim();
    if (trimmed === '') return { ok: true, value: null };
    try {
      const parsed = JSON.parse(trimmed);
      if (parsed === null || typeof parsed !== 'object') {
        toast(`${label} must be a JSON object or array`, 'error');
        return { ok: false };
      }
      return { ok: true, value: parsed };
    } catch {
      toast(`${label} is not valid JSON`, 'error');
      return { ok: false };
    }
  }

  async function saveSettings() {
    if (!campaign) return;
    // The JSON fields are validated FIRST, so a malformed box is
    // reported before anything is written - and nothing is written at
    // all if one of them is bad.
    const persona = parseJsonField(formPersona, 'Audience persona');
    if (!persona.ok) return;
    const pillars = parseJsonField(formPillars, 'Content pillars');
    if (!pillars.ok) return;
    const kpis = parseJsonField(formKpiTargets, 'KPI targets');
    if (!kpis.ok) return;
    saving = true;
    const r = await campaignsApi.update(campaign.id, {
      name: formName,
      // `null` means "the user cleared this"; omitting the key means
      // "leave it alone". The old `|| undefined` sent "omit" for an
      // emptied field, so the server kept the old value and the save
      // reported success while nothing changed.
      description: formDescription || null,
      color: formColor,
      start_date: formStartDate || null,
      end_date: formEndDate || null,
      goal: formGoal || null,
      status: formStatus,
      progress_metric: formProgressMetric || null,
      progress_target: typeof formProgressTarget === 'number' ? formProgressTarget : null,
      // The four fields that were stored and never editable.
      //
      // They are JSON, so a malformed box has to be REPORTED rather
      // than silently dropped - a silent drop would leave the stored
      // value in place and the founder would think their edit saved.
      audience_persona: persona.value as Record<string, unknown> | null,
      content_pillars: pillars.value as Array<{ title: string }> | null,
      kpi_targets: kpis.value as Record<string, unknown> | null,
      budget_cents: formBudget.trim() === '' ? null : Number(formBudget),
    });
    saving = false;
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign updated', 'success');
      load();
    }
  }

  async function archiveCampaign() {
    if (!campaign) return;
    const ok = await modals.areYouSure({
      title: `Archive "${campaign.name}"?`,
      message: 'The campaign will be archived. Its posts stay exactly as they are.',
      confirmLabel: 'Archive',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    // Status, not DELETE: the DELETE route also sets deleted_at, which hides the
    // campaign from every list, so the Archived filter could never show it.
    const r = await campaignsApi.update(campaign.id, { status: 'archived' });
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign archived', 'success');
      goto('/campaigns');
    }
  }

  // The bar measures `progress_metric` - the metric the founder chose.
  // It used to always measure the POST count, so a campaign with
  // `progress_metric = "engagement"` showed its post count against an
  // engagement target: "engagement: 4 / 1000" on a bar at 0%.
  function progressValue(): { current: number | null; unit: string } {
    const metric = campaign?.progress_metric ?? 'posts';
    switch (metric) {
      case 'engagement':
        return { current: campaign?.engagement_total ?? null, unit: 'engagement' };
      case 'reach':
      case 'followers':
      case 'custom':
        // Not measured yet. The post count under these labels would be a
        // different number wearing the metric's name.
        return { current: null, unit: metric };
      case 'posts':
      default:
        return { current: campaign?.post_count ?? 0, unit: 'posts' };
    }
  }

  function progressPct(): number {
    if (!campaign?.progress_target || campaign.progress_target === 0) return 0;
    const { current } = progressValue();
    if (current === null) return 0;
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

  const tabs = $derived([
    { id: 'overview', label: 'Overview' },
    { id: 'posts', label: `Posts (${posts.length})` },
    { id: 'settings', label: 'Settings' },
  ]);
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
      <button onclick={() => goto('/campaigns')} class="px-3 py-2 text-sm text-muted hover:text-content whitespace-nowrap min-h-11">
        ← Back
      </button>
    </PageHeader>

    <Tabs tabs={tabs} bind:value={activeTab} />

    {#if activeTab === 'overview'}
      <!-- Overview tab -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <!-- Progress card -->
        <div class="bg-surface border border-line rounded-lg p-5">
          <h2 class="font-medium text-sm mb-4">Progress</h2>
          {#if campaign.progress_target}
            <div class="mb-4">
              <div class="flex justify-between text-sm mb-2">
                <span class="text-muted">
                  {#if progressValue().current === null}
                    {campaign.progress_target} {progressValue().unit} goal
                  {:else}
                    {progressValue().current} / {campaign.progress_target} {progressValue().unit}
                  {/if}
                </span>
                {#if progressValue().current !== null}
                  <span class="font-medium">{Math.round(progressPct())}%</span>
                {/if}
              </div>
              {#if progressValue().current === null}
                <!--
                  The metric has no data source. "Reach", "followers" and
                  "custom" are accepted by the server and offered in the
                  Settings tab, but nothing in the codebase measures
                  them.

                  So there is no bar and no percentage: a 0% bar against a
                  target reads as "you have achieved nothing", which is
                  a claim about the campaign rather than about the
                  metric. The founder is told the metric is not measured.
                -->
                <div class="h-2 bg-background-input rounded-full flex items-center justify-center">
                  <span class="text-[10px] text-faint">Not measured</span>
                </div>
              {:else}
              <div class="bg-background-input rounded-full h-3 overflow-hidden">
                <div class="h-full rounded-full transition-all duration-500" style="width: {progressPct()}%; background: {campaign.color}"></div>
              </div>
              {/if}
            </div>
          {:else}
            <p class="text-sm text-muted">
              {campaign.post_count ?? 0} posts in this campaign.
              {#if (campaign.failed_count ?? 0) > 0}
                <span class="text-error">{campaign.failed_count} failed to publish</span> -
                they are not counted as work, but they were attempted.
              {/if}
              Set a progress target in Settings to track a goal.
            </p>
          {/if}

          <div class="grid grid-cols-2 gap-4 mt-4 pt-4 border-t border-line">
            <div>
              <div class="text-[10px] text-faint uppercase tracking-wider">Start</div>
              <div class="text-sm">{campaign.start_date ? formatDateOnly(campaign.start_date) : '-'}</div>
            </div>
            <div>
              <div class="text-[10px] text-faint uppercase tracking-wider">End</div>
              <div class="text-sm">{campaign.end_date ? formatDateOnly(campaign.end_date) : '-'}</div>
            </div>
          </div>

          {#if campaign.goal}
            <div class="mt-4 pt-4 border-t border-line">
              <div class="text-[10px] text-faint uppercase tracking-wider mb-1">Goal</div>
              <div class="text-sm">{campaign.goal}</div>
            </div>
          {/if}
        </div>

        <!-- Post counts by state -->
        <div class="bg-surface border border-line rounded-lg p-5">
          <h2 class="font-medium text-sm mb-4">Posts by State</h2>
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
            <a href="/posts/{post.id}" class="flex items-center gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors min-h-11">
              <span class="text-xs" style="color: {providerInkOnCard(post.integration_name?.toLowerCase()?.split(/\s+/)[0] || '')}">
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
          <label for="campaign-name" class="block text-sm text-muted mb-1">Name</label>
          <input id="campaign-name" type="text" bind:value={formName} class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
        </div>
        <div>
          <label for="campaign-description" class="block text-sm text-muted mb-1">Description</label>
          <textarea id="campaign-description" bind:value={formDescription} rows="2" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none"></textarea>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="campaign-color" class="block text-sm text-muted mb-1">Color</label>
            <div class="flex items-center gap-2">
              <input id="campaign-color" type="color" bind:value={formColor} class="min-h-11 shrink-0 w-10 h-9 p-0 rounded cursor-pointer bg-transparent border border-line" />
              <input type="text" bind:value={formColor} class="min-h-11 flex-1 px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
            </div>
          </div>
          <div>
            <label for="campaign-status" class="block text-sm text-muted mb-1">Status</label>
            <select id="campaign-status" bind:value={formStatus} class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none">
              <option value="active">Active</option>
              <option value="paused">Paused</option>
              <option value="archived">Archived</option>
              <option value="completed">Completed</option>
            </select>
          </div>
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="campaign-start" class="block text-sm text-muted mb-1">Start Date</label>
            <input id="campaign-start" type="date" bind:value={formStartDate} class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
          </div>
          <div>
            <label for="campaign-end" class="block text-sm text-muted mb-1">End Date</label>
            <input id="campaign-end" type="date" bind:value={formEndDate} class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
          </div>
        </div>
        <div>
          <label for="campaign-goal" class="block text-sm text-muted mb-1">Goal (free text)</label>
          <input id="campaign-goal" type="text" bind:value={formGoal} placeholder="e.g. 1000 new followers" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
        </div>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="campaign-metric" class="block text-sm text-muted mb-1">Progress Metric</label>
            <select id="campaign-metric" bind:value={formProgressMetric} class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none">
              <option value="">None</option>
              <option value="posts">Posts</option>
              <option value="engagement">Engagement</option>
              <option value="reach">Reach</option>
              <option value="followers">Followers</option>
              <option value="custom">Custom</option>
            </select>
          </div>
          <div>
            <label for="campaign-target" class="block text-sm text-muted mb-1">Progress Target</label>
            <input id="campaign-target" type="number" bind:value={formProgressTarget} placeholder="e.g. 20" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
          </div>
        </div>
        <!--
          The four fields the server has always persisted and no screen
          ever showed. They are JSON, so they are edited as JSON.

          A founder who set a persona through the API opened the
          campaign, saw nothing, and concluded it had never been
          saved - and "Save Changes" omitted them, so the round-trip
          was a no-op that reported success.
        -->
        <div>
          <label for="campaign-persona" class="block text-sm text-muted mb-1">Audience Persona (JSON)</label>
          <textarea id="campaign-persona" bind:value={formPersona} rows="4" placeholder="age_range: 25-44" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm font-mono focus:border-accent outline-none"></textarea>
        </div>
        <div>
          <label for="campaign-pillars" class="block text-sm text-muted mb-1">Content Pillars (JSON array)</label>
          <textarea id="campaign-pillars" bind:value={formPillars} rows="4" placeholder="title: Engineering" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm font-mono focus:border-accent outline-none"></textarea>
        </div>
        <div>
          <label for="campaign-kpis" class="block text-sm text-muted mb-1">KPI Targets (JSON)</label>
          <textarea id="campaign-kpis" bind:value={formKpiTargets} rows="3" placeholder="min_engagement_rate: 3.5" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm font-mono focus:border-accent outline-none"></textarea>
        </div>
        <div>
          <label for="campaign-budget" class="block text-sm text-muted mb-1">Budget (cents)</label>
          <input id="campaign-budget" type="number" bind:value={formBudget} placeholder="e.g. 5000" class="min-h-11 w-full px-3 py-2 bg-background-input border border-line rounded text-sm focus:border-accent outline-none" />
        </div>
        <div class="flex items-center justify-between pt-4 border-t border-line">
          <button onclick={archiveCampaign} class="text-sm text-error hover:text-error/80 transition-colors min-h-11">
            Archive Campaign
          </button>
          <button onclick={saveSettings} disabled={saving || !formName.trim()} class="px-4 py-2 text-sm bg-accent-fill hover:bg-accent-fill-hover text-accent-fg disabled:opacity-50 rounded transition-colors min-h-11">
            {saving ? 'Saving...' : 'Save Changes'}
          </button>
        </div>
      </div>
    {/if}
  {/if}
</div>
