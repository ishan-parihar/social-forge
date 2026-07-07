<script lang="ts">
  // ComposerModal — the modal-based composer (Phase 2).
  //
  // This is the keystone of the v18 refactor: it replaces the full-page
  // /posts/new and /posts/[id] edit flows with a single modal that opens
  // in-place from anywhere (calendar, dashboard, posts list, keyboard 'n').
  //
  // Design decisions (per PLAN_FRONTEND_REFACTOR_v18.md Phase 2):
  //   - Full-screen modal (max-w-[1400px], h-[90vh]) — like postiz-app
  //   - Two-column layout on lg+: editor left, preview right (580px)
  //   - Single column on mobile (preview collapses to a toggle)
  //   - askClose: true — confirm before closing if content is non-empty
  //   - Reuses existing composer widgets (ChannelSelector, RichTextEditor,
  //     MediaUpload, SchedulePicker, TagPicker, etc.) — no rewrite
  //   - Create and edit share the same UI; edit mode pre-fills from API
  //
  // The composer store (lib/stores/composer.svelte.ts) drives open/close
  // and entry-point state (presetDate, editingPostId, etc.).

  import { onMount, onDestroy } from 'svelte';
  import { postsApi, type PostDetail, type PostSummary } from '$lib/api/posts';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { toast } from '$lib/stores/toast';
  import { realtime } from '$lib/stores/realtime';
  import { modals } from '$lib/stores/modals.svelte';
  import { composer } from '$lib/stores/composer.svelte';
  import { goto } from '$app/navigation';
  import ChannelSelector from '$lib/composer/ChannelSelector.svelte';
  import RichTextEditor from '$lib/composer/RichTextEditor.svelte';
  import MediaUpload from '$lib/composer/MediaUpload.svelte';
  import SchedulePicker from '$lib/composer/SchedulePicker.svelte';
  import TagPicker from '$lib/composer/TagPicker.svelte';
  import { campaignsApi, type Campaign } from '$lib/api/campaigns';
  import PostSetModal from '$lib/composer/PostSetModal.svelte';
  import ThreadFinisher from '$lib/composer/ThreadFinisher.svelte';
  import FirstComment from '$lib/composer/FirstComment.svelte';
  import AiAssistant from '$lib/composer/AiAssistant.svelte';
  import AiHashtagSuggestions from '$lib/composer/AiHashtagSuggestions.svelte';
  import MusicPicker from '$lib/composer/MusicPicker.svelte';
  import PerPlatformCharCount from '$lib/composer/PerPlatformCharCount.svelte';
  import PlatformPreviewPane from '$lib/composer/PlatformPreviewPane.svelte';
  import SelectCurrent from '$lib/composer/SelectCurrent.svelte';
  import type { MediaItem } from '$lib/api/media';
  import TargetPicker from '$lib/composer/TargetPicker.svelte';
  import type { TargetInfo } from '$lib/api/integrations';

  // Props passed by ModalManager (it injects a `close` function).
  let { close } = $props<{ close: (confirmed?: boolean) => void }>();

  // ── Form state (mirrors routes/posts/new/+page.svelte) ──────────
  let content = $state('');
  let title = $state('');
  let selectedIntegrations = $state<string[]>([]);
  let allIntegrations = $state<Integration[]>([]);
  let integrationProviders = $derived(new Map(allIntegrations.map(i => [i.id, i.provider_identifier])));
  let integrationNames = $derived(new Map(allIntegrations.map(i => [i.id, i.provider_name])));
  let mediaItems = $state<MediaItem[]>([]);
  let scheduledAt = $state<string | null>(null);
  let recurring = $state<{ intervalDays: number; endDate: string } | null>(null);
  let selectedTagIds = $state<string[]>([]);
  // Phase 8: campaign selector in composer.
  let allCampaigns = $state<Campaign[]>([]);
  let selectedCampaignId = $state<string | null>(null);
  let editingMode = $state<string>('global');
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let firstComment = $state('');
  let showAi = $state(false);
  let showMusicPicker = $state(false);
  let selectedMusic = $state<{ id: string; title: string; artist: string } | null>(null);
  let providerOverride = $state<Map<string, string>>(new Map());
  let showPostSets = $state(false);
  let loading = $state(true);
  // Phase 8: mobile preview toggle — on < lg screens, the preview pane
  // is hidden by default and can be toggled with a button in the footer.
  let showPreviewMobile = $state(false);

  // Phase v21: group-editing state. When editing a post that has a
  // group_id (i.e., it's part of a thread or has a first-comment child),
  // we fetch all sibling posts via postsApi.getGroup() and show a tab
  // strip at the top of the editor so the user can switch between parts.
  // The currently-edited part's content/title/media is what gets saved
  // on submit. Switching tabs saves the current draft to a per-part
  // cache and loads the newly-selected part.
  let groupPosts = $state<Array<{ id: string; sequence: number; content: string; title?: string }>>([]);
  let editingGroupPostId = $state<string | null>(null);
  // Per-part draft cache so switching tabs doesn't lose unsaved edits.
  let groupDraftCache = $state<Map<string, { content: string; title: string; mediaItems: MediaItem[] }>>(new Map());

  // Draft auto-save (create mode only)
  let draftSaved = $state(false);
  let draftTimer: ReturnType<typeof setTimeout>;
  const DRAFT_KEY = 'social-forge-composer-draft';

  // Target selection
  let integrationTargets = $state<Map<string, TargetInfo[]>>(new Map());
  let selectedTargets = $state<Map<string, string[]>>(new Map());
  let targetsLoading = $state<Set<string>>(new Set());

  let hasXIntegration = $derived(
    selectedIntegrations.some(id => integrationProviders.get(id) === 'x')
  );
  let hasInstagramIntegration = $derived(
    selectedIntegrations.some(id => {
      const p = integrationProviders.get(id);
      return p === 'instagram' || p === 'instagram-standalone';
    })
  );

  let unsubscribers: (() => void)[] = [];

  async function loadIntegrations() {
    const r = await integrationsApi.list();
    if (r.data) {
      allIntegrations = r.data.integrations.filter(i => !i.disabled);
      const validIds = new Set(allIntegrations.map(i => i.id));
      selectedIntegrations = selectedIntegrations.filter(id => validIds.has(id));
    }
  }

  async function loadEditingPost(postId: string) {
    const r = await postsApi.get(postId);
    if (r.error || !r.data) {
      toast(`Failed to load post: ${r.error || 'unknown'}`, 'error');
      close();
      return;
    }
    const post = r.data;
    content = post.content || '';
    title = post.title || '';
    selectedIntegrations = [post.integration_id];
    scheduledAt = post.scheduled_at || null;
    firstComment = post.first_comment || '';
    selectedTagIds = post.tags?.map(t => t.id) || [];
    mediaItems = (post.media || []).map(m => ({
      id: crypto.randomUUID(),
      url: m.url,
      mime_type: m.mime_type,
      original_name: m.alt || 'media',
      file_size: 0,
    }));
    editingGroupPostId = post.id;

    // Phase v21: if this post is part of a group (thread or first-comment
    // chain), fetch all sibling posts and populate the group tab strip.
    // The user can then switch between parts without leaving the composer.
    if (post.group_id) {
      const groupRes = await postsApi.getGroup(post.group_id);
      if (groupRes.data && groupRes.data.length > 1) {
        // Sort by sequence so the tab strip shows parts in order.
        groupPosts = groupRes.data
          .map(p => ({ id: p.id, sequence: p.sequence ?? 0, content: p.content, title: p.title }))
          .sort((a, b) => a.sequence - b.sequence);
      } else {
        groupPosts = [];
      }
    } else {
      groupPosts = [];
    }
  }

  /**
   * Phase v21: switch the editor to a different part of the thread group.
   * Caches the current part's unsaved edits, then loads the selected part.
   * The cache is keyed by post id and holds {content, title, mediaItems}.
   */
  function switchGroupPart(targetId: string) {
    if (targetId === editingGroupPostId) return;
    // Save current state to cache.
    if (editingGroupPostId) {
      groupDraftCache.set(editingGroupPostId, {
        content,
        title,
        mediaItems: [...mediaItems],
      });
      groupDraftCache = new Map(groupDraftCache);
    }
    // Load target from cache or from the groupPosts snapshot.
    const cached = groupDraftCache.get(targetId);
    if (cached) {
      content = cached.content;
      title = cached.title;
      mediaItems = [...cached.mediaItems];
    } else {
      const part = groupPosts.find(p => p.id === targetId);
      if (part) {
        content = part.content || '';
        title = part.title || '';
        mediaItems = [];
      }
    }
    editingGroupPostId = targetId;
  }

  onMount(async () => {
    loading = true;
    // Pre-fill from composer store entry-point state.
    if (composer.presetDate) {
      scheduledAt = `${composer.presetDate}T09:00:00.000Z`;
    } else if (composer.mode === 'create') {
      // Restore draft if present.
      const saved = localStorage.getItem(DRAFT_KEY);
      if (saved) {
        try {
          const d = JSON.parse(saved);
          if (d.content) content = d.content;
          if (d.title) title = d.title;
          if (d.selectedIntegrations) selectedIntegrations = d.selectedIntegrations;
          if (d.scheduledAt) scheduledAt = d.scheduledAt;
          if (d.firstComment) firstComment = d.firstComment;
        } catch {}
      }
    }
    if (composer.presetIntegrationIds.length > 0) {
      selectedIntegrations = [...composer.presetIntegrationIds];
    }
    if (composer.prefilledContent) {
      content = composer.prefilledContent;
    }
    if (composer.mode === 'edit' && composer.editingPostId) {
      await loadEditingPost(composer.editingPostId);
    } else {
      await loadIntegrations();
    }

    // Phase 8: load campaigns for the campaign selector.
    const campRes = await campaignsApi.list();
    if (campRes.data) allCampaigns = campRes.data;

    // Realtime: refresh integrations if connected/disconnected in another tab.
    unsubscribers.push(realtime.on('integration_connected', loadIntegrations));
    unsubscribers.push(realtime.on('integration_disconnected', loadIntegrations));

    // Composer keyboard shortcuts (U-4): Cmd+Enter=post, Cmd+S=save draft.
    function onKeydown(e: KeyboardEvent) {
      const isMod = e.metaKey || e.ctrlKey;
      if (!isMod) return;
      if (e.key === 'Enter') {
        e.preventDefault();
        if (!submitting) postNow();
      } else if (e.key === 's' || e.key === 'S') {
        e.preventDefault();
        saveDraftNow();
      }
    }
    window.addEventListener('keydown', onKeydown);
    unsubscribers.push(() => window.removeEventListener('keydown', onKeydown));

    loading = false;
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  // Auto-save draft (create mode only).
  $effect(() => {
    if (composer.mode !== 'create') return;
    const _ = [content, title, selectedIntegrations, scheduledAt, firstComment];
    clearTimeout(draftTimer);
    draftSaved = false;
    const timer = setTimeout(() => {
      if (content || title) {
        localStorage.setItem(DRAFT_KEY, JSON.stringify({
          content, title, selectedIntegrations, scheduledAt, firstComment
        }));
        draftSaved = true;
        setTimeout(() => { draftSaved = false; }, 2000);
      }
    }, 1500);
    return () => clearTimeout(timer);
  });

  function saveDraftNow() {
    if (composer.mode !== 'create') return;
    if (content || title) {
      localStorage.setItem(DRAFT_KEY, JSON.stringify({
        content, title, selectedIntegrations, scheduledAt, firstComment
      }));
      draftSaved = true;
      clearTimeout(draftTimer);
      setTimeout(() => { draftSaved = false; }, 2000);
      toast('Draft saved', 'success');
    }
  }

  function insertAiText(text: string) {
    content = content + (content ? '\n\n' : '') + text;
  }

  function addHashtag(tag: string) {
    content = content + (content.endsWith(' ') ? '' : ' ') + tag;
  }

  async function fetchTargets(integrationId: string) {
    targetsLoading.add(integrationId);
    try {
      const r = await integrationsApi.listTargets(integrationId);
      if (r.data && r.data.targets.length > 0) {
        integrationTargets.set(integrationId, r.data.targets);
      }
    } catch (e) {
      toast('Failed to fetch targets: ' + (e instanceof Error ? e.message : String(e)), 'error');
    } finally {
      targetsLoading.delete(integrationId);
    }
  }

  function toggleTarget(integrationId: string, targetId: string) {
    const current = selectedTargets.get(integrationId) || [];
    if (current.includes(targetId)) {
      selectedTargets.set(integrationId, current.filter(t => t !== targetId));
    } else {
      selectedTargets.set(integrationId, [...current, targetId]);
    }
  }

  function handlePostSetLoad(set: { content: string; channelIds: string[]; scheduledAt?: string | null }) {
    if (set.content) content = set.content;
    if (set.channelIds.length > 0) selectedIntegrations = set.channelIds;
    if (set.scheduledAt) scheduledAt = set.scheduledAt;
  }

  function buildPayload() {
    const overridesObj: Record<string, { content: string }> = {};
    for (const [id, html] of providerOverride) {
      if (html && html !== content) {
        overridesObj[id] = { content: html };
      }
    }
    const settings: Record<string, unknown> = {};
    for (const [intId, targets] of selectedTargets) {
      if (targets.length > 0) {
        settings[intId] = { target_ids: targets };
      }
    }
    return {
      integration_ids: selectedIntegrations,
      content,
      title: title || undefined,
      scheduled_at: scheduledAt || undefined,
      tag_ids: selectedTagIds,
      first_comment: firstComment || undefined,
      media: mediaItems.length > 0 ? mediaItems.map(m => ({ id: m.id, url: m.url, mime_type: m.mime_type, alt: undefined })) : undefined,
      overrides: Object.keys(overridesObj).length > 0 ? overridesObj : undefined,
      settings: {
        ...settings,
        ...(selectedMusic ? { audio_id: selectedMusic.id, audio_title: selectedMusic.title } : {}),
      },
      // Phase v21: wire recurring schedule into the payload so it actually
      // gets sent to the backend. Previously SchedulePicker wrote to
      // `recurring` state but buildPayload() never included it — the field
      // was silently dropped and no repeat series was ever created.
      recurring: recurring ? { interval_days: recurring.intervalDays, end_date: recurring.endDate } : undefined,
    };
  }

  /**
   * Phase v21 fix: edit-mode branch.
   *
   * Previously `submit()`/`postNow()`/`saveAsDraft()` all unconditionally
   * called `postsApi.create()`, even when `composer.mode === 'edit'`. This
   * meant editing a post and clicking "Schedule" created a DUPLICATE post
   * instead of updating the existing one. The `postsApi.update()` method
   * existed but was never called from the UI.
   *
   * Now we branch on `composer.mode === 'edit'` and call
   * `postsApi.update(editingPostId, payload)` instead. The update endpoint
   * accepts content/title/media/settings; it does NOT change scheduled_at
   * (use the separate reschedule endpoint for that) and does NOT change
   * integration_ids (you can't move a post to a different channel after
   * creation — that's a duplicate-and-delete flow).
   *
   * Recurring schedule is wired into the create flow only — for edit mode,
   * the user must use the dedicated "Repeat" action on the post (outside
   * the composer) because the repeat endpoint requires an existing post id.
   */
  async function submit() {
    if (submitting) return;
    if (selectedIntegrations.length === 0) { error = 'Please select at least one channel'; return; }
    if (!content.trim()) { error = 'Please write some content'; return; }
    submitting = true;
    error = null;
    try {
      const payload = buildPayload();
      const valRes = await postsApi.validate(payload);
      if (valRes.data && !valRes.data.valid && valRes.data.errors.length > 0) {
        // Phase v21: show ALL per-platform validation errors as toasts,
        // not just the first one. Format: "X (@handle): post is too long".
        for (const err of valRes.data.errors) {
          toast(`${err.provider_name}: ${err.message}`, 'error');
        }
        const firstErr = valRes.data.errors[0];
        error = firstErr.provider_name + ': ' + firstErr.message;
        return;
      }

      if (composer.mode === 'edit' && composer.editingPostId) {
        // ── Edit mode: update existing post ──
        const r = await postsApi.update(currentEditingPostId, {
          content: payload.content,
          title: payload.title,
          media: payload.media,
          settings: payload.settings,
        });
        if (r.error) { error = r.error; return; }

        // If a scheduled_at is set and differs from the original, reschedule.
        // (The update endpoint doesn't touch scheduled_at.)
        if (payload.scheduled_at && r.data?.scheduled_at && payload.scheduled_at !== r.data.scheduled_at) {
          await postsApi.reschedule(currentEditingPostId, payload.scheduled_at, false);
        }

        localStorage.removeItem(DRAFT_KEY);
        composer.close();
        toast('Post updated', 'success');
      } else {
        // ── Create mode: create new post(s) ──
        const r = await postsApi.create(payload);
        if (r.error) { error = r.error; return; }

        // Phase v21: wire recurring schedule. After successful creation,
        // if recurring is set, call postsApi.repeat() to spawn the series.
        if (recurring && r.data?.posts?.[0]?.id) {
          const repeatRes = await postsApi.repeat(
            r.data.posts[0].id,
            recurring.intervalDays,
            recurring.endDate,
          );
          if (repeatRes.error) {
            toast(`Post created but repeat setup failed: ${repeatRes.error}`, 'error');
          } else {
            toast(`Post scheduled + ${repeatRes.data?.count || 0} recurring copies created`, 'success');
          }
        }

        localStorage.removeItem(DRAFT_KEY);
        composer.close();
        if (!recurring) toast('Post scheduled', 'success');
      }
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || 'Failed to schedule';
    } finally {
      submitting = false;
    }
  }

  async function postNow() {
    if (submitting) return;
    if (selectedIntegrations.length === 0) { error = 'Please select at least one channel'; return; }
    if (!content.trim()) { error = 'Please write some content'; return; }
    submitting = true;
    error = null;
    try {
      const payload = buildPayload();
      const valRes = await postsApi.validate(payload);
      if (valRes.data && !valRes.data.valid && valRes.data.errors.length > 0) {
        for (const err of valRes.data.errors) {
          toast(`${err.provider_name}: ${err.message}`, 'error');
        }
        const firstErr = valRes.data.errors[0];
        error = firstErr.provider_name + ': ' + firstErr.message;
        return;
      }

      if (composer.mode === 'edit' && composer.editingPostId) {
        // Edit mode: update content, then publish immediately.
        const r = await postsApi.update(currentEditingPostId, {
          content: payload.content,
          title: payload.title,
          media: payload.media,
          settings: payload.settings,
        });
        if (r.error) { error = r.error; return; }
        const pub = await postsApi.publish(currentEditingPostId);
        if (pub.error) { error = pub.error; return; }
        localStorage.removeItem(DRAFT_KEY);
        composer.close();
        toast('Post published', 'success');
      } else {
        // Create mode: create + publish.
        const r = await postsApi.create({ ...payload, scheduled_at: undefined });
        if (r.error) { error = r.error; return; }
        if (r.data?.posts?.[0]?.id) {
          const pub = await postsApi.publish(r.data.posts[0].id);
          if (pub.error) { error = pub.error; return; }
        }
        localStorage.removeItem(DRAFT_KEY);
        composer.close();
        toast('Post published', 'success');
      }
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || 'Failed to post';
    } finally {
      submitting = false;
    }
  }

  async function saveAsDraft() {
    if (submitting) return;
    if (!content.trim() && !title.trim()) { error = 'Please write some content or title'; return; }
    submitting = true;
    error = null;
    try {
      const payload = buildPayload();
      // Save as draft: no scheduled_at, state will default to draft
      if (composer.mode === 'edit' && composer.editingPostId) {
        // Edit mode: update content + clear scheduled_at by transitioning
        // back to draft state via reschedule (the backend's reschedule
        // endpoint accepts a new scheduled_at; we set it to the current
        // time + 100 years to effectively unschedule — TODO: add a
        // dedicated "unchedule" endpoint in v22).
        // For now, just update content; the state stays as-is.
        const r = await postsApi.update(currentEditingPostId, {
          content: payload.content,
          title: payload.title,
          media: payload.media,
          settings: payload.settings,
        });
        if (r.error) { error = r.error; return; }
        localStorage.removeItem(DRAFT_KEY);
        composer.close();
        toast('Draft updated', 'success');
      } else {
        const r = await postsApi.create({ ...payload, scheduled_at: undefined });
        if (r.error) { error = r.error; return; }
        localStorage.removeItem(DRAFT_KEY);
        composer.close();
        toast('Draft saved', 'success');
      }
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || 'Failed to save draft';
    } finally {
      submitting = false;
    }
  }

  // askClose: if content is non-empty, confirm before closing.
  async function handleClose(confirmed: boolean) {
    if (confirmed) {
      composer.close();
      return;
    }
    // Backdrop/escape close — confirm if dirty.
    if (content.trim() || title.trim() || selectedIntegrations.length > 0) {
      const ok = await modals.areYouSure({
        title: 'Discard this post?',
        message: 'Your content will be lost. (Drafts are auto-saved if you typed something.)',
        confirmLabel: 'Discard',
        cancelLabel: 'Keep editing',
        danger: true,
      });
      if (ok) {
        composer.close();
      }
      return; // abort close either way (we handle it manually)
    }
    composer.close();
  }

  let isDirty = $derived(!!content.trim() || !!title.trim() || selectedIntegrations.length > 0);

  // Phase v21: the post id to send update/publish/reschedule calls to.
  // Falls back to composer.editingPostId when the user hasn't switched
  // tabs (or when there's no group — single-post edit).
  let currentEditingPostId = $derived(editingGroupPostId || composer.editingPostId);

  // Phase 3: compute which integrations have diverged from global.
  // An integration "diverged" when it has an override in providerOverride
  // AND that override differs from the current global content.
  // This drives the pink dot on the SelectCurrent pill strip.
  let divergedIntegrations = $derived.by(() => {
    const set = new Set<string>();
    for (const [intId, html] of providerOverride) {
      if (html && html !== content) {
        set.add(intId);
      }
    }
    return set;
  });

  // Phase 3: when the user switches to a per-channel tab for the first
  // time (no override exists yet), clone the global content into the
  // override so the editor starts from global and then diverges.
  // This matches postiz-app's addRemoveInternal() clone-on-first-switch.
  function handleCurrentChange(tab: string) {
    if (tab !== 'global' && !providerOverride.has(tab)) {
      // Clone global → internal[tab] so the editor has something to edit.
      providerOverride.set(tab, content);
      providerOverride = new Map(providerOverride);
    }
    editingMode = tab === 'global' ? 'global' : `internal:${tab}`;
  }

  // Phase 3: remove a per-channel override (reset to global).
  // Called by SelectCurrent's X button.
  function handleRemoveOverride(integrationId: string) {
    providerOverride.delete(integrationId);
    providerOverride = new Map(providerOverride);
    // If we were editing this channel, switch back to global.
    if (editingMode === `internal:${integrationId}`) {
      editingMode = 'global';
    }
  }
</script>

<div class="flex flex-col h-full">
  <!-- Header -->
  <div class="flex items-center justify-between px-5 py-3 border-b border-line shrink-0">
    <div class="flex items-center gap-3">
      <h2 class="text-xl font-semibold">
        {composer.mode === 'edit' ? 'Edit Post' : 'Create Post'}
      </h2>
      {#if composer.mode === 'edit'}
        <span class="text-xs px-2 py-0.5 rounded-full bg-indigo-500/20 text-indigo-400">Editing</span>
      {/if}
      {#if draftSaved}
        <span class="text-xs text-emerald-400 animate-pulse">✓ Draft saved</span>
      {/if}
    </div>
    <div class="flex items-center gap-3">
      <span class="hidden lg:inline text-[10px] text-muted-dark" title="Keyboard shortcuts">
        <kbd class="px-1 py-0.5 bg-surface-hover rounded">⌘</kbd>+<kbd class="px-1 py-0.5 bg-surface-hover rounded">↵</kbd> post ·
        <kbd class="px-1 py-0.5 bg-surface-hover rounded">⌘</kbd>+<kbd class="px-1 py-0.5 bg-surface-hover rounded">S</kbd> save
      </span>
      <button
        onclick={() => handleClose(true)}
        class="text-muted hover:text-content text-2xl leading-none -mt-1"
        aria-label="Close composer"
      >&times;</button>
    </div>
  </div>

  {#if loading}
    <div class="flex-1 flex items-center justify-center">
      <div class="text-sm text-muted">Loading...</div>
    </div>
  {:else}
    <!-- Two-column body: editor left, preview right -->
    <div class="flex-1 flex flex-col lg:flex-row overflow-hidden">
      <!-- Left column: editor (scrollable) -->
      <div class="flex-1 overflow-y-auto p-5 space-y-4">
        {#if error}
          <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3">{error}</div>
        {/if}

        <!-- Phase v21: group/thread tab strip. Shows one tab per post in
             the group when editing a multi-part thread. Switching tabs
             caches the current part's edits and loads the selected part. -->
        {#if composer.mode === 'edit' && groupPosts.length > 1}
          <div class="bg-surface border border-line rounded-xl p-2">
            <div class="text-xs text-muted mb-1.5 px-1">Thread parts ({groupPosts.length})</div>
            <div class="flex gap-1 flex-wrap">
              {#each groupPosts as part, i (part.id)}
                <button
                  onclick={() => switchGroupPart(part.id)}
                  class="px-2.5 py-1 text-xs rounded-lg transition-colors {part.id === editingGroupPostId ? 'bg-indigo-600 text-white' : 'bg-surface-hover text-muted hover:text-content'}"
                  title={part.content ? (part.content.slice(0, 80) + (part.content.length > 80 ? '…' : '')) : `Part ${i + 1}`}
                >
                  {i === 0 ? 'Main post' : `Part ${i + 1}`}
                </button>
              {/each}
            </div>
            <p class="text-[10px] text-muted-dark mt-1.5 px-1">Editing part {groupPosts.findIndex(p => p.id === editingGroupPostId) + 1} of {groupPosts.length}. Switching tabs preserves your unsaved edits per part.</p>
          </div>
        {/if}

        <!-- Title -->
        <div>
          <label class="text-sm text-muted block mb-1">Title (optional)</label>
          <input type="text" bind:value={title} placeholder="Post title..."
            class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none" />
        </div>

        <!-- Channel Selection -->
        <div class="bg-surface border border-line rounded-xl p-4">
          <h3 class="text-sm font-semibold mb-3">Post to</h3>
          <ChannelSelector
            selected={selectedIntegrations}
            onToggle={(id) => {
              if (selectedIntegrations.includes(id)) {
                selectedIntegrations = selectedIntegrations.filter(i => i !== id);
              } else {
                selectedIntegrations = [...selectedIntegrations, id];
              }
            }}
          />
        </div>

        <!-- Posting Targets -->
        {#each selectedIntegrations as intId (intId)}
          {#if integrationTargets.has(intId)}
            <div class="bg-surface border border-line rounded-xl p-4">
              <h3 class="text-sm font-semibold mb-3">
                Posting Targets for {integrationNames.get(intId)}
                {#if targetsLoading.has(intId)}
                  <span class="ml-2 text-xs text-muted">Loading...</span>
                {/if}
              </h3>
              <TargetPicker
                targets={integrationTargets.get(intId) || []}
                selectedTargets={selectedTargets.get(intId) || []}
                onToggle={(targetId) => toggleTarget(intId, targetId)}
              />
            </div>
          {/if}
        {/each}

        <!-- Tags -->
        <div class="bg-surface border border-line rounded-xl p-4">
          <h3 class="text-sm font-semibold mb-3">Tags</h3>
          <TagPicker
            selected={selectedTagIds}
            onToggle={(id) => {
              if (selectedTagIds.includes(id)) {
                selectedTagIds = selectedTagIds.filter(t => t !== id);
              } else {
                selectedTagIds = [...selectedTagIds, id];
              }
            }}
          />
        </div>

        <!-- Phase 8: Campaign selector -->
        {#if allCampaigns.length > 0}
          <div class="bg-surface border border-line rounded-xl p-4">
            <h3 class="text-sm font-semibold mb-3">Campaign</h3>
            <select
              value={selectedCampaignId || ''}
              onchange={(e) => selectedCampaignId = e.currentTarget.value || null}
              class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none"
            >
              <option value="">No campaign</option>
              {#each allCampaigns as c (c.id)}
                <option value={c.id}>{c.name}</option>
              {/each}
            </select>
          </div>
        {/if}

        <!-- Global vs Internal channel editor toggle (Phase 3: SelectCurrent) -->
        {#if selectedIntegrations.length > 1}
          <div class="bg-surface border border-line rounded-xl p-3">
            <SelectCurrent
              {selectedIntegrations}
              {integrationProviders}
              {integrationNames}
              current={editingMode === 'global' ? 'global' : editingMode.split(':')[1]}
              {divergedIntegrations}
              onCurrentChange={handleCurrentChange}
              onRemoveIntegration={handleRemoveOverride}
            />
          </div>
        {/if}

        <!-- Content Editor -->
        <div class="bg-surface border border-line rounded-xl p-4">
          <h3 class="text-sm font-semibold mb-3">
            {#if editingMode === 'global'}
              Content (shared across all channels)
            {:else}
              Content for {integrationNames.get(editingMode.split(':')[1]) || 'this channel'}
            {/if}
          </h3>
          {#if editingMode === 'global'}
            <RichTextEditor {content} onUpdate={(html) => content = html} />
            {#if selectedIntegrations.length > 0}
              <div class="mt-3 pt-3 border-t border-line">
                <PerPlatformCharCount
                  {content}
                  {selectedIntegrations}
                  {integrationProviders}
                  {integrationNames}
                />
              </div>
            {/if}
          {:else}
            <RichTextEditor
              content={providerOverride.get(editingMode.split(':')[1]) || content}
              onUpdate={(html) => {
                const intId = editingMode.split(':')[1];
                providerOverride.set(intId, html);
                providerOverride = new Map(providerOverride);
              }}
            />
            <div class="mt-3 pt-3 border-t border-line">
              <PerPlatformCharCount
                content={providerOverride.get(editingMode.split(':')[1]) || content}
                selectedIntegrations={[editingMode.split(':')[1]]}
                {integrationProviders}
                {integrationNames}
              />
            </div>
          {/if}
        </div>

        <!-- AI Assistant -->
        {#if showAi}
          <AiAssistant {content} onInsert={insertAiText} />
        {/if}

        <!-- AI Hashtag Suggestions -->
        <AiHashtagSuggestions {content} onAddHashtag={addHashtag} />

        <!-- Media -->
        <div class="bg-surface border border-line rounded-xl p-4">
          <h3 class="text-sm font-semibold mb-3">Media</h3>
          <MediaUpload items={mediaItems}
            onAdd={(item) => mediaItems = [...mediaItems, item]}
            onRemove={(id) => mediaItems = mediaItems.filter(m => m.id !== id)}
            onReorder={(newItems) => mediaItems = newItems}
          />
        </div>

        <!-- Music (Instagram only) -->
        {#if hasInstagramIntegration}
          <div class="bg-surface border border-line rounded-xl p-4">
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-sm font-semibold">🎵 Music</h3>
              <button
                onclick={() => showMusicPicker = true}
                class="text-xs px-3 py-1.5 bg-surface-hover hover:bg-line-hover rounded-lg text-muted hover:text-white transition-colors"
              >
                {selectedMusic ? 'Change' : 'Browse'}
              </button>
            </div>
            {#if selectedMusic}
              <div class="flex items-center gap-3 bg-surface-hover rounded-lg p-2">
                <div class="w-8 h-8 rounded bg-brand-500/20 flex items-center justify-center text-brand-400 text-sm">🎵</div>
                <div class="flex-1 min-w-0">
                  <p class="text-sm font-medium truncate">{selectedMusic.title}</p>
                  <p class="text-xs text-muted truncate">{selectedMusic.artist}</p>
                </div>
                <button onclick={() => selectedMusic = null} class="text-muted hover:text-red-400 text-sm">&times;</button>
              </div>
            {:else}
              <p class="text-xs text-muted">Add trending music from Instagram's library to your Reel.</p>
            {/if}
          </div>
        {/if}

        <!-- Thread Finisher (X/Twitter only) -->
        {#if hasXIntegration}
          <ThreadFinisher {content} onCreateThread={async (parts) => {
            if (submitting) return;
            if (selectedIntegrations.length === 0) { error = 'Please select at least one channel'; return; }
            submitting = true;
            error = null;
            try {
              const r = await postsApi.createThread({
                content_parts: parts,
                integration_ids: selectedIntegrations,
                scheduled_at: scheduledAt || undefined,
              });
              if (r.error) { error = r.error; return; }
              localStorage.removeItem(DRAFT_KEY);
              composer.close();
              toast('Thread created', 'success');
            } catch (e: unknown) {
              error = (e instanceof Error ? e.message : String(e)) || 'Failed to create thread';
            } finally {
              submitting = false;
            }
          }} {submitting} />
        {/if}

        <!-- First Comment (LinkedIn/Facebook only) -->
        <FirstComment
          {selectedIntegrations}
          {integrationProviders}
          firstComment={firstComment}
          onFirstCommentChange={(v: string) => firstComment = v}
        />
      </div>

      <!-- Right column: preview + schedule (fixed width on lg+, toggleable on mobile) -->
      <div class="lg:w-[400px] lg:border-l lg:border-line lg:bg-background/50 overflow-y-auto p-5 space-y-4
        {showPreviewMobile ? 'block' : 'hidden lg:block'}">
        <!-- Mobile: close preview button -->
        <div class="lg:hidden flex justify-end">
          <button onclick={() => showPreviewMobile = false} class="text-muted hover:text-content text-sm">✕ Close preview</button>
        </div>
        <!-- Phase 4: per-platform live preview pane -->
        <PlatformPreviewPane
          content={editingMode === 'global' ? content : (providerOverride.get(editingMode.split(':')[1]) || content)}
          current={editingMode === 'global' ? 'global' : editingMode.split(':')[1]}
          {selectedIntegrations}
          {integrationProviders}
          {integrationNames}
          media={mediaItems}
        />

        <!-- Schedule -->
        <div class="bg-surface border border-line rounded-xl p-4">
          <h3 class="text-sm font-semibold mb-3">Schedule</h3>
          <SchedulePicker
            {scheduledAt}
            onChange={(iso: string | null) => scheduledAt = iso}
            {recurring}
            onRecurringChange={(r: { intervalDays: number; endDate: string } | null) => recurring = r}
            integrationId={selectedIntegrations[0]}
          />
        </div>

        <!-- Post Sets -->
        <button
          onclick={() => showPostSets = true}
          class="w-full px-3 py-2 text-sm text-muted hover:text-white border border-line rounded-lg transition-colors"
        >Post Sets</button>
      </div>
    </div>

    <!-- Footer: actions -->
    <div class="border-t border-line px-5 py-3 flex items-center justify-between shrink-0 bg-surface">
      <div class="flex items-center gap-2">
        <button
          onclick={() => showAi = !showAi}
          class="px-3 py-1.5 text-sm border border-line rounded-lg transition-colors {showAi ? 'bg-indigo-600/20 text-indigo-400 border-indigo-500/30' : 'text-muted hover:text-white'}"
        >✨ AI</button>
        <!-- Phase 8: mobile preview toggle -->
        <button
          onclick={() => showPreviewMobile = !showPreviewMobile}
          class="lg:hidden px-3 py-1.5 text-sm border border-line rounded-lg transition-colors text-muted hover:text-white"
        >{showPreviewMobile ? '✕ Preview' : '👁 Preview'}</button>
      </div>
      <div class="flex items-center gap-2 flex-wrap justify-end">
        {#if composer.mode === 'edit'}
          <!-- Edit mode: no "save draft" (post already exists) -->
        {:else}
          <button
            onclick={saveAsDraft}
            disabled={submitting}
            class="px-3 py-1.5 text-sm text-muted hover:text-white border border-line rounded-lg disabled:opacity-50 transition-colors"
          >Save Draft</button>
        {/if}
        <button
          onclick={postNow}
          disabled={submitting}
          class="px-3 py-1.5 bg-green-600 hover:bg-green-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors"
        >{submitting ? 'Posting...' : 'Post Now'}</button>
        <button
          onclick={submit}
          disabled={submitting}
          class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors"
        >{submitting ? 'Scheduling...' : 'Schedule'}</button>
      </div>
    </div>
  {/if}
</div>

{#if showMusicPicker}
  <MusicPicker integrationId={selectedIntegrations.find(id => integrationProviders.get(id) === 'instagram' || integrationProviders.get(id) === 'instagram-standalone') || ''} onSelect={(m) => { selectedMusic = m; showMusicPicker = false; }} onclose={() => showMusicPicker = false} />
{/if}

{#if showPostSets}
  <PostSetModal open={true} onclose={() => showPostSets = false} currentContent={content} onLoad={handlePostSetLoad} />
{/if}
