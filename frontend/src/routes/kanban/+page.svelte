<script lang="ts">
  // Kanban board — content pipeline view (Phase 7, v20).
  //
  // Shows posts grouped by state (Ideas → Drafts → Scheduled → Published)
  // with drag-and-drop between columns. Optionally filter by campaign.
  //
  // Inspired by postiz-app's calendar list view but reimagined as a
  // kanban board for content ideation and pipeline management.

  import { onMount, onDestroy } from 'svelte';
  import { postsApi, type PostSummary } from '$lib/api/posts';
  import { campaignsApi, type Campaign } from '$lib/api/campaigns';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { composer } from '$lib/stores/composer.svelte';
  import { modals } from '$lib/stores/modals.svelte';
  import { toast } from '$lib/stores/toast';
  import { realtime } from '$lib/stores/realtime';
  import { providerIcon, providerColor } from '$lib/providers';
  import Badge from '$lib/ui/Badge.svelte';
  import { goto } from '$app/navigation';

  let posts = $state<PostSummary[]>([]);
  let campaigns = $state<Campaign[]>([]);
  // v22 Phase 6: load integrations so quick-add can require a channel
  // (previously created posts with integration_ids: [] which the
  // backend rejects).
  let integrations = $state<Integration[]>([]);
  let selectedCampaign = $state<string | null>(null);
  let loading = $state(true);
  let draggingId = $state<string | null>(null);

  // Phase v21: campaign-create modal state. Previously used native
  // prompt() which is jarring and can't be styled. Now we render a
  // small inline modal with a text input + Create/Cancel buttons.
  let createCampaignModalOpen = $state(false);
  let newCampaignName = $state('');

  // Kanban columns — map post_state to display config.
  const columns = [
    { state: 'idea', label: '💡 Ideas', color: 'border-t-purple-500', emptyMsg: 'No ideas yet. Quick-add one below!' },
    { state: 'draft', label: '📝 Drafts', color: 'border-t-blue-500', emptyMsg: 'No drafts. Create a post to start.' },
    { state: 'queued', label: '📅 Scheduled', color: 'border-t-indigo-500', emptyMsg: 'No scheduled posts.' },
    { state: 'published', label: '✅ Published', color: 'border-t-green-500', emptyMsg: 'No published posts yet.' },
  ];

  let unsubscribers: (() => void)[] = [];

  async function load() {
    loading = true;
    const [postsRes, campRes, integRes] = await Promise.all([
      postsApi.list({ limit: 200 }),
      campaignsApi.list(),
      integrationsApi.list(),
    ]);
    if (postsRes.data) posts = postsRes.data.posts;
    if (campRes.data) campaigns = campRes.data;
    if (integRes.data) integrations = integRes.data.integrations.filter(i => !i.disabled);
    loading = false;
  }

  onMount(() => {
    load();
    // v22 Phase 1: subscribe to post lifecycle events + the new
    // `post_stage_changed` (kanban drag in another tab) + `lagged`
    // (SSE missed events → refetch).
    // v22 Phase 6: also subscribe to campaign_created/updated/deleted.
    const events = [
      'post_created', 'post_scheduled', 'post_published', 'post_failed',
      'post_deleted', 'post_stage_changed', 'lagged',
      'campaign_created', 'campaign_updated', 'campaign_deleted',
    ];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  // v22 Phase 6: fixed campaign filter. Previously accessed
  // `(p as any).campaign_id` because PostSummary didn't have the field.
  // Now PostSummary has campaign_id as a proper optional field, so the
  // filter works correctly. The old `p.group_id === selectedCampaign`
  // fallback (which matched on thread/group ID, not campaign ID) is
  // removed — it was a bug that caused false positives.
  let filteredPosts = $derived(
    selectedCampaign
      ? posts.filter(p => p.campaign_id === selectedCampaign)
      : posts
  );

  let postsByState = $derived.by(() => {
    const map: Record<string, PostSummary[]> = {};
    for (const col of columns) {
      // v25-3: sort by kanban_sort_order (ascending, stable on created_at
      // as tiebreaker) so drag-to-reorder (v25-4) has a deterministic
      // order to mutate. Falls back to created_at desc when sort_order
      // is 0 for all posts (the default until the user reorders).
      const colPosts = filteredPosts.filter(p => p.state === col.state);
      colPosts.sort((a, b) => {
        const so = (a.kanban_sort_order ?? 0) - (b.kanban_sort_order ?? 0);
        if (so !== 0) return so;
        return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
      });
      map[col.state] = colPosts;
    }
    // Also include 'error' posts in the draft column.
    const errPosts = filteredPosts.filter(p => p.state === 'error');
    errPosts.sort((a, b) => {
      const so = (a.kanban_sort_order ?? 0) - (b.kanban_sort_order ?? 0);
      if (so !== 0) return so;
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    });
    map['draft'] = [...map['draft'], ...errPosts];
    return map;
  });

  // Quick-add idea: create a post with state='idea' and minimal content.
  // v22 Phase 6: fixed the integration_ids: [] bug. Previously quick-add
  // created posts with no integration, which the backend rejects (the
  // composer requires at least one integration). Now we require a
  // channel to be selected via the quick-add-integration dropdown. If
  // the user has no connected channels, we show a helpful message
  // directing them to the Channels page.
  let quickIdeaText = $state('');
  let quickIdeaIntegrationId = $state<string>('');
  let showQuickAddIntegSelect = $state(false);

  // Auto-select the first integration when the dropdown opens.
  $effect(() => {
    if (showQuickAddIntegSelect && !quickIdeaIntegrationId && integrations.length > 0) {
      quickIdeaIntegrationId = integrations[0].id;
    }
  });

  async function quickAddIdea() {
    if (!quickIdeaText.trim()) return;
    if (!quickIdeaIntegrationId) {
      toast('Please select a channel for this idea, or connect one in the Channels page first.', 'error');
      showQuickAddIntegSelect = true;
      return;
    }
    // Create a post with the selected integration.
    const r = await postsApi.create({
      integration_ids: [quickIdeaIntegrationId],
      content: quickIdeaText.trim(),
      title: undefined,
    });
    if (r.error) {
      toast(`Failed to save idea: ${r.error}`, 'error');
      return;
    }
    // If created successfully, update its stage to 'idea'.
    if (r.data?.posts?.[0]?.id) {
      await campaignsApi.updateStage(r.data.posts[0].id, 'idea');
    }
    quickIdeaText = '';
    showQuickAddIntegSelect = false;
    toast('Idea saved', 'success');
    load();
  }

  // Drag-and-drop: move post to a new column.
  function onDragStart(e: DragEvent, postId: string) {
    draggingId = postId;
    e.dataTransfer?.setData('text/plain', `kanban:${postId}`);
    e.dataTransfer!.effectAllowed = 'move';
  }

  async function onDrop(e: DragEvent, newState: string) {
    e.preventDefault();
    const postId = draggingId;
    if (!postId) return;
    draggingId = null;

    // Optimistic update: move the post to the new column immediately.
    const post = posts.find(p => p.id === postId);
    if (post) {
      post.state = newState;
      posts = [...posts];
    }

    // Persist the stage change.
    const r = await campaignsApi.updateStage(postId, newState, selectedCampaign || undefined);
    if (r.error) {
      toast(`Failed to move: ${r.error}`, 'error');
      load(); // revert
    } else {
      toast(`Moved to ${newState}`, 'success');
    }
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function onDragEnd() {
    draggingId = null;
  }

  // Create a new campaign.
  // Phase v21: replaced native prompt() with an inline modal
  // (createCampaignModalOpen + newCampaignName state). The modal markup
  // is at the bottom of the file. confirmCreateCampaign() does the
  // actual API call after the user types a name and clicks Create.
  function openCreateCampaignModal() {
    newCampaignName = '';
    createCampaignModalOpen = true;
  }

  async function confirmCreateCampaign() {
    const name = newCampaignName.trim();
    if (!name) return;
    createCampaignModalOpen = false;
    const r = await campaignsApi.create({ name, color: '#6366f1' });
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Campaign created', 'success');
      load();
    }
  }

  // Delete a campaign.
  // Phase v21: replaced native confirm() with modals.areYouSure.
  async function deleteCampaign(id: string) {
    const ok = await modals.areYouSure({
      title: 'Delete this campaign?',
      message: 'Posts in this campaign will be unassigned (not deleted). You can re-assign them to another campaign later.',
      confirmLabel: 'Delete',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    const r = await campaignsApi.delete(id);
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      selectedCampaign = null;
      toast('Campaign deleted', 'success');
      load();
    }
  }

  // Format date for display.
  function formatDate(iso?: string): string {
    if (!iso) return '';
    const d = new Date(iso);
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  // v25-3: kanban card metadata helpers. These render priority / due date /
  // substate / tags on each card using semantic tokens so they retheme
  // correctly in dark + light mode.

  // Resolve the provider_identifier for a post — needed because
  // providerColor/providerIcon key on the provider id (e.g. "x"), not the
  // user-set account name. The dashboard had this same bug (fixed in v22
  // Phase 5); the kanban had it too. Now both use the same lookup pattern.
  function providerIdFor(post: PostSummary): string {
    const match = integrations.find(i => i.id === post.integration_id);
    return match?.provider_identifier || post.integration_name?.toLowerCase()?.split(/\s+/)[0] || '';
  }

  // Priority → display label + Tailwind classes. "medium" is the default
  // and is NOT rendered as a chip (only deviations from default get visual
  // emphasis, to keep the card scannable).
  function priorityLabel(p: string): string {
    return ({ low: 'Low', medium: 'Medium', high: 'High', urgent: 'Urgent' } as Record<string, string>)[p] || p;
  }
  function priorityChipClass(p: string): string {
    return ({
      low: 'text-muted bg-line/40',
      medium: 'text-muted bg-line/40',
      high: 'text-error bg-error/10',
      urgent: 'text-error bg-error/20 font-bold',
    } as Record<string, string>)[p] || 'text-muted bg-line/40';
  }
  // Left-border accent for the card itself — draws the eye to high/urgent.
  function priorityBorderClass(p: string): string {
    return ({
      low: 'border-l-2 border-l-info/40',
      medium: '',
      high: 'border-l-2 border-l-error/70',
      urgent: 'border-l-2 border-l-error',
    } as Record<string, string>)[p] || '';
  }

  // Due date → color based on how far out it is. Overdue = red, today =
  // yellow, soon (≤3d) = muted, future = muted-dark. Returns just the
  // text color class; the caller wraps the icon + date.
  function dueDateClass(iso: string): string {
    const d = new Date(iso);
    if (isNaN(d.getTime())) return 'text-muted';
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const dueDay = new Date(d.getFullYear(), d.getMonth(), d.getDate());
    const diffDays = Math.round((dueDay.getTime() - today.getTime()) / 86_400_000);
    if (diffDays < 0) return 'text-error font-medium';
    if (diffDays === 0) return 'text-warning font-medium';
    if (diffDays <= 3) return 'text-muted';
    return 'text-muted-dark';
  }

  // Substate → label + text color + dot color.
  function substateLabel(s: string): string {
    return ({
      ready_to_publish: 'Ready',
      in_review: 'In review',
      blocked: 'Blocked',
    } as Record<string, string>)[s] || s;
  }
  function substateTextClass(s: string): string {
    return ({
      ready_to_publish: 'text-success',
      in_review: 'text-warning',
      blocked: 'text-error',
    } as Record<string, string>)[s] || 'text-muted';
  }
  function substateDotClass(s: string): string {
    return ({
      ready_to_publish: 'bg-success',
      in_review: 'bg-warning',
      blocked: 'bg-error',
    } as Record<string, string>)[s] || 'bg-muted';
  }
</script>

<div class="page-enter space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between">
    <div>
      <h2 class="text-xl font-semibold">Content Pipeline</h2>
      <p class="text-sm text-muted mt-1">Drag posts between columns to move them through your content pipeline.</p>
    </div>
    <div class="flex gap-2">
      <button onclick={openCreateCampaignModal} class="px-3 py-1.5 text-sm border border-line rounded-lg text-muted hover:text-white hover:bg-surface-hover transition-colors">
        + Campaign
      </button>
    </div>
  </div>

  <!-- Campaign filter -->
  {#if campaigns.length > 0}
    <div class="flex items-center gap-2 flex-wrap">
      <span class="text-xs text-muted">Campaign:</span>
      <button
        onclick={() => selectedCampaign = null}
        class="px-3 py-1 text-xs rounded-lg transition-colors {!selectedCampaign ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover border border-line'}"
      >All</button>
      {#each campaigns as c (c.id)}
        <div class="flex items-center gap-1">
          <button
            onclick={() => selectedCampaign = selectedCampaign === c.id ? null : c.id}
            class="px-3 py-1 text-xs rounded-lg transition-colors flex items-center gap-1.5 {selectedCampaign === c.id ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover border border-line'}"
          >
            <span class="w-2 h-2 rounded-full" style="background: {c.color}"></span>
            {c.name}
            {#if c.post_count}
              <span class="text-[10px] opacity-60">({c.post_count})</span>
            {/if}
          </button>
          {#if selectedCampaign === c.id}
            <button onclick={() => deleteCampaign(c.id)} class="text-muted hover:text-red-400 text-xs" title="Delete campaign">✕</button>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  <!-- Kanban board -->
  {#if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {#each columns as col (col.state)}
        <div
          class="bg-surface border border-line rounded-xl overflow-hidden {col.color} border-t-4"
          ondragover={onDragOver}
          ondrop={(e) => onDrop(e, col.state)}
          role="region"
          aria-label="{col.label} column"
        >
          <!-- Column header -->
          <div class="px-4 py-3 border-b border-line flex items-center justify-between">
            <span class="text-sm font-semibold">{col.label}</span>
            <span class="text-xs text-muted">{postsByState[col.state]?.length || 0}</span>
          </div>

          <!-- Cards -->
          <div class="p-2 space-y-2 min-h-[200px] max-h-[600px] overflow-y-auto">
            {#each postsByState[col.state] || [] as post (post.id)}
              <article
                class="bg-background-input border border-line rounded-lg cursor-grab active:cursor-grabbing hover:border-brand-500/50 transition-colors overflow-hidden {draggingId === post.id ? 'opacity-50' : ''} {priorityBorderClass(post.priority || 'medium')}"
                draggable={true}
                ondragstart={(e) => onDragStart(e, post.id)}
                ondragend={onDragEnd}
                onclick={() => composer.openEdit(post.id)}
                role="button"
                tabindex="0"
                onkeydown={(e) => { if (e.key === 'Enter') composer.openEdit(post.id); }}
                aria-label="Post: {(post.content || post.title || '(no content)').slice(0, 80)}"
              >
                <div class="p-3">
                  <!-- Content -->
                  <div class="text-sm text-content line-clamp-2 mb-2">{post.content || post.title || '(no content)'}</div>

                  <!-- Tags (max 3 visible, +N for the rest) -->
                  {#if post.tags && post.tags.length > 0}
                    <div class="flex flex-wrap gap-1 mb-2">
                      {#each post.tags.slice(0, 3) as tag (tag.id)}
                        <span
                          class="px-1.5 py-0.5 rounded text-[10px] font-medium leading-none"
                          style="background-color: {tag.color}22; color: {tag.color}"
                          title={tag.name}
                        >{tag.name}</span>
                      {/each}
                      {#if post.tags.length > 3}
                        <span class="text-[10px] text-muted leading-none py-0.5" title="{post.tags.length - 3} more tags">+{post.tags.length - 3}</span>
                      {/if}
                    </div>
                  {/if}

                  <!-- Metadata row: priority + due date + substate -->
                  {#if (post.priority && post.priority !== 'medium') || post.due_date || post.kanban_substate}
                    <div class="flex flex-wrap items-center gap-1.5 mb-2 text-[10px] leading-none">
                      {#if post.priority && post.priority !== 'medium'}
                        <span class="px-1.5 py-0.5 rounded font-medium {priorityChipClass(post.priority)}">
                          {priorityLabel(post.priority)}
                        </span>
                      {/if}
                      {#if post.due_date}
                        <span class="flex items-center gap-0.5 {dueDateClass(post.due_date)}" title="Due {new Date(post.due_date).toLocaleString()}">
                          📅 {formatDate(post.due_date)}
                        </span>
                      {/if}
                      {#if post.kanban_substate}
                        <span class="flex items-center gap-0.5 {substateTextClass(post.kanban_substate)}">
                          <span class="w-1.5 h-1.5 rounded-full {substateDotClass(post.kanban_substate)}"></span>
                          {substateLabel(post.kanban_substate)}
                        </span>
                      {/if}
                    </div>
                  {/if}

                  <!-- Footer: provider + scheduled date -->
                  <div class="flex items-center justify-between text-xs text-muted">
                    <span class="flex items-center gap-1 min-w-0">
                      {#if post.integration_name}
                        <span style="color: {providerColor(providerIdFor(post))}" class="shrink-0">{providerIcon(providerIdFor(post))}</span>
                        <span class="truncate max-w-[80px]">{post.integration_name}</span>
                      {/if}
                    </span>
                    {#if post.scheduled_at}
                      <span class="shrink-0 ml-2">{formatDate(post.scheduled_at)}</span>
                    {/if}
                  </div>

                  {#if post.error_message}
                    <div class="mt-1 text-[10px] text-error truncate" title={post.error_message}>⚠ {post.error_message}</div>
                  {/if}
                </div>
              </article>
            {/each}

            <!-- Empty state -->
            {#if !postsByState[col.state] || postsByState[col.state].length === 0}
              <div class="text-center py-8 text-xs text-muted">{col.emptyMsg}</div>
            {/if}
          </div>

          <!-- Quick-add idea (only in Ideas column) -->
          {#if col.state === 'idea'}
            <div class="p-2 border-t border-line space-y-1">
              <div class="flex gap-1">
                <input
                  type="text"
                  bind:value={quickIdeaText}
                  onkeydown={(e) => { if (e.key === 'Enter') quickAddIdea(); }}
                  onfocus={() => { if (!showQuickAddIntegSelect) showQuickAddIntegSelect = true; }}
                  placeholder="Quick add idea..."
                  class="flex-1 px-2 py-1 text-xs bg-background-input border border-line rounded focus:border-brand-500 outline-none"
                />
                <button
                  onclick={quickAddIdea}
                  disabled={!quickIdeaText.trim()}
                  class="px-2 py-1 text-xs bg-brand-500 hover:bg-brand-600 disabled:opacity-50 text-white rounded transition-colors"
                >+</button>
              </div>
              <!-- v22 Phase 6: channel selector for quick-add. Previously
                   quick-add created posts with integration_ids: [] which
                   the backend rejects. Now the user must pick a channel. -->
              {#if showQuickAddIntegSelect}
                {#if integrations.length === 0}
                  <p class="text-[10px] text-warning px-1">
                    No channels connected. <a href="/channels" class="underline">Connect one</a> to add ideas.
                  </p>
                {:else}
                  <select
                    bind:value={quickIdeaIntegrationId}
                    class="w-full px-2 py-1 text-xs bg-background-input border border-line rounded focus:border-brand-500 outline-none"
                  >
                    {#each integrations as int (int.id)}
                      <option value={int.id}>{int.provider_name || int.provider_identifier}</option>
                    {/each}
                  </select>
                {/if}
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Phase v21: campaign-create modal — replaces native prompt(). -->
{#if createCampaignModalOpen}
  <div
    class="fixed inset-0 z-[200] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
    onclick={() => (createCampaignModalOpen = false)}
    role="dialog"
    aria-modal="true"
    aria-labelledby="create-campaign-title"
  >
    <div
      class="bg-surface border border-line rounded-xl shadow-2xl w-full max-w-md p-5"
      onclick={(e) => e.stopPropagation()}
    >
      <h3 id="create-campaign-title" class="text-lg font-semibold mb-1">New campaign</h3>
      <p class="text-xs text-muted mb-4">Campaigns group related posts together on the calendar.</p>
      <label class="text-sm text-muted block mb-1.5">Name</label>
      <input
        type="text"
        bind:value={newCampaignName}
        placeholder="e.g. Product launch Q3"
        autofocus
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none mb-4"
        onkeydown={(e) => { if (e.key === 'Enter') confirmCreateCampaign(); if (e.key === 'Escape') createCampaignModalOpen = false; }}
      />
      <div class="flex items-center justify-end gap-2">
        <button
          onclick={() => (createCampaignModalOpen = false)}
          class="px-3 py-1.5 text-sm text-muted hover:text-content border border-line rounded-lg transition-colors"
        >Cancel</button>
        <button
          onclick={confirmCreateCampaign}
          disabled={!newCampaignName.trim()}
          class="px-3 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg transition-colors"
        >Create</button>
      </div>
    </div>
  </div>
{/if}
