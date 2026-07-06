<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { postsApi } from "$lib/api/posts";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount, onDestroy } from "svelte";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";
  import ChannelSelector from "$lib/composer/ChannelSelector.svelte";
  import RichTextEditor from "$lib/composer/RichTextEditor.svelte";
  import MediaUpload from "$lib/composer/MediaUpload.svelte";
  import SchedulePicker from "$lib/composer/SchedulePicker.svelte";
  import PostPreview from "$lib/composer/PostPreview.svelte";
  import TagPicker from "$lib/composer/TagPicker.svelte";
  import PostSetModal from "$lib/composer/PostSetModal.svelte";
  import ThreadFinisher from "$lib/composer/ThreadFinisher.svelte";
  import FirstComment from "$lib/composer/FirstComment.svelte";
  import AiAssistant from "$lib/composer/AiAssistant.svelte";
  import AiHashtagSuggestions from "$lib/composer/AiHashtagSuggestions.svelte";
  import MusicPicker from "$lib/composer/MusicPicker.svelte";
  import PerPlatformCharCount from "$lib/composer/PerPlatformCharCount.svelte";
  import type { MediaItem } from "$lib/api/media";
  import TargetPicker from "$lib/composer/TargetPicker.svelte";
  import type { TargetInfo } from "$lib/api/integrations";

  let content = $state("");
  let title = $state("");
  let selectedIntegrations = $state<string[]>([]);
  let allIntegrations = $state<Integration[]>([]);
  let integrationProviders = $derived(new Map(allIntegrations.map(i => [i.id, i.provider_identifier])));
  let integrationNames = $derived(new Map(allIntegrations.map(i => [i.id, i.provider_name])));
  let mediaItems = $state<MediaItem[]>([]);
  let scheduledAt = $state<string | null>(null);
  let recurring = $state<{ intervalDays: number; endDate: string } | null>(null);
  let selectedTagIds = $state<string[]>([]);
  // "global" = shared content for all channels
  // "internal:{integrationId}" = per-channel override
  let editingMode = $state<string>("global");
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let firstComment = $state("");
  let showAi = $state(false);
  let showMusicPicker = $state(false);
  let selectedMusic = $state<{ id: string; title: string; artist: string } | null>(null);

  // Draft auto-save state
  let draftSaved = $state(false);
  let draftTimer: ReturnType<typeof setTimeout>;
  let showRestorePrompt = $state(false);

  // Target selection state
  let integrationTargets = $state<Map<string, TargetInfo[]>>(new Map());
  let selectedTargets = $state<Map<string, string[]>>(new Map());
  let targetsLoading = $state<Set<string>>(new Set());

  // Auto-detect if X/Twitter is selected (for thread mode)
  let hasXIntegration = $derived(
    selectedIntegrations.some(id => integrationProviders.get(id) === 'x')
  );
  let hasInstagramIntegration = $derived(
    selectedIntegrations.some(id => {
      const p = integrationProviders.get(id);
      return p === 'instagram' || p === 'instagram-standalone';
    })
  );
  let instagramIntegrationId = $derived(
    selectedIntegrations.find(id => {
      const p = integrationProviders.get(id);
      return p === 'instagram' || p === 'instagram-standalone';
    }) || ''
  );

  const DRAFT_KEY = 'social-forge-composer-draft';
  let autoSaveTimer: ReturnType<typeof setTimeout>;

  let unsubscribers: (() => void)[] = [];

  async function refreshIntegrations() {
    const r = await integrationsApi.list();
    if (r.data) {
      allIntegrations = r.data.integrations.filter(i => !i.disabled);
      // Prune any selected integrations that no longer exist (e.g.
      // user disconnected an account in another tab while composing).
      const validIds = new Set(allIntegrations.map(i => i.id));
      selectedIntegrations = selectedIntegrations.filter(id => validIds.has(id));
    }
  }

  onMount(async () => {
    const dateParam = new URL(window.location.href).searchParams.get('date');
    if (dateParam) {
      scheduledAt = `${dateParam}T09:00:00.000Z`;
    } else {
      // Check for saved draft and prompt to restore
      const saved = localStorage.getItem(DRAFT_KEY);
      if (saved) {
        try {
          const d = JSON.parse(saved);
          if (d.content || d.selectedIntegrations?.length) {
            showRestorePrompt = true;
          }
        } catch {}
      }
    }
    const r = await integrationsApi.list();
    if (r.data) allIntegrations = r.data.integrations.filter(i => !i.disabled);

    // Composer-specific keyboard shortcuts (U-4):
    //   Cmd/Ctrl + Enter  → publish now (postNow)
    //   Cmd/Ctrl + S      → save draft to localStorage (preventDefault
    //                       so the browser's "Save Page" dialog doesn't fire)
    // These are scoped to the composer page only and don't conflict with
    // the global shortcuts (n, /, g c, etc.) which are single-key.
    function onKeydown(e: KeyboardEvent) {
      const isMod = e.metaKey || e.ctrlKey;
      if (!isMod) return;
      if (e.key === 'Enter') {
        e.preventDefault();
        // Don't trigger if already submitting — postNow() guards too,
        // but preventing the toast spam is nicer UX.
        if (!submitting) postNow();
      } else if (e.key === 's' || e.key === 'S') {
        e.preventDefault();
        // Force-save the draft immediately (bypass the 1500ms debounce).
        if (content || title) {
          localStorage.setItem(DRAFT_KEY, JSON.stringify({
            content, title, selectedIntegrations, scheduledAt, firstComment
          }));
          draftSaved = true;
          clearTimeout(draftTimer);
          draftTimer = setTimeout(() => { draftSaved = false; }, 2000);
          toast('Draft saved', 'success');
        }
      }
    }
    window.addEventListener('keydown', onKeydown);
    unsubscribers.push(() => window.removeEventListener('keydown', onKeydown));

    // Realtime: if the user connects/disconnects an account in
    // another tab (or via the onboarding flow), refresh the channel
    // selector so newly-connected accounts appear immediately and
    // disconnected ones are pruned from the selection.
    unsubscribers.push(realtime.on('integration_connected', () => refreshIntegrations()));
    unsubscribers.push(realtime.on('integration_disconnected', () => refreshIntegrations()));
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  function restoreDraft() {
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
    showRestorePrompt = false;
  }

  function dismissDraft() {
    localStorage.removeItem(DRAFT_KEY);
    showRestorePrompt = false;
  }

  // Auto-save draft to localStorage with visual feedback
  $effect(() => {
    const _ = [content, title, selectedIntegrations, scheduledAt, firstComment];
    clearTimeout(autoSaveTimer);
    clearTimeout(draftTimer);
    draftSaved = false;
    autoSaveTimer = setTimeout(() => {
      if (content || title) {
        localStorage.setItem(DRAFT_KEY, JSON.stringify({
          content, title, selectedIntegrations, scheduledAt, firstComment
        }));
        draftSaved = true;
        draftTimer = setTimeout(() => { draftSaved = false; }, 2000);
      }
    }, 1500);
  });

  // Fetch targets for newly selected integrations
  $effect(() => {
    const _ = [selectedIntegrations];
    for (const intId of selectedIntegrations) {
      if (!integrationTargets.has(intId)) {
        fetchTargets(intId);
      }
    }
    // Remove targets for deselected integrations
    for (const key of integrationTargets.keys()) {
      if (!selectedIntegrations.includes(key)) {
        integrationTargets.delete(key);
        selectedTargets.delete(key);
      }
    }
  });

  let providerOverride = $state<Map<string, string>>(new Map());
  let showPostSets = $state(false);

  function handlePostSetLoad(set: { content: string; channelIds: string[]; scheduledAt?: string | null }) {
    if (set.content) content = set.content;
    if (set.channelIds.length > 0) selectedIntegrations = set.channelIds;
    if (set.scheduledAt) scheduledAt = set.scheduledAt;
  }

  async function fetchTargets(integrationId: string) {
    targetsLoading.add(integrationId);
    try {
      const r = await integrationsApi.listTargets(integrationId);
      if (r.data && r.data.targets.length > 0) {
        integrationTargets.set(integrationId, r.data.targets);
      }
    } catch (e) {
      toast("Failed to fetch targets for " + integrationId + ": " + (e instanceof Error ? e.message : String(e)), "error");
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

  async function handleCreateThread(parts: string[]) {
    if (submitting) return;
    if (selectedIntegrations.length === 0) {
      error = "Please select at least one channel";
      return;
    }
    submitting = true;
    error = null;
    try {
      const r = await postsApi.createThread({
        content_parts: parts,
        integration_ids: selectedIntegrations,
        scheduled_at: scheduledAt || undefined,
      });
      if (r.error) {
        error = r.error;
        submitting = false;
        return;
      }
      goto("/calendar");
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || "Failed to create thread";
    } finally {
      submitting = false;
    }
  }

  async function submit() {
    if (submitting) return;
    if (selectedIntegrations.length === 0) {
      error = "Please select at least one channel";
      return;
    }
    if (!content.trim()) {
      error = "Please write some content";
      return;
    }
    submitting = true;
    error = null;
    try {
      const overridesObj: Record<string, { content: string }> = {};
      for (const [id, html] of providerOverride) {
        if (html && html !== content) {
          overridesObj[id] = { content: html };
        }
      }
      // Build settings with target info
      const settings: Record<string, unknown> = {};
      for (const [intId, targets] of selectedTargets) {
        if (targets.length > 0) {
          settings[intId] = { target_ids: targets };
        }
      }

      const payload = {
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
      };

      // ── Pre-submit validation ──────────────────────────────
      // Call /api/posts/validate to check per-provider limits before
      // creating posts. If validation fails, show the first error.
      const valRes = await postsApi.validate(payload);
      if (valRes.data && !valRes.data.valid && valRes.data.errors.length > 0) {
        const firstErr = valRes.data.errors[0];
        error = firstErr.provider_name + ": " + firstErr.message;
        submitting = false;
        return;
      }

      const r = await postsApi.create(payload);
      if (r.error) {
        error = r.error;
        submitting = false;
        return;
      }
      // Set up recurring if configured
      if (recurring && r.data?.posts?.[0]?.id) {
        await postsApi.repeat(r.data.posts[0].id, recurring.intervalDays, recurring.endDate);
      }
      localStorage.removeItem(DRAFT_KEY);
      goto("/calendar");
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || "Failed to create post";
    } finally {
      submitting = false;
    }
  }

  async function postNow() {
    if (submitting) return;
    if (selectedIntegrations.length === 0) { error = "Please select at least one channel"; return; }
    if (!content.trim()) { error = "Please write some content"; return; }
    submitting = true;
    error = null;
    try {
      // Build overrides (only includes content that differs from main)
      const overridesObj: Record<string, { content: string }> = {};
      for (const [id, html] of providerOverride) {
        if (html && html !== content) {
          overridesObj[id] = { content: html };
        }
      }
      // Build settings with target info
      const settings: Record<string, unknown> = {};
      for (const [intId, targets] of selectedTargets) {
        if (targets.length > 0) {
          settings[intId] = { target_ids: targets };
        }
      }
      const payload = {
        integration_ids: selectedIntegrations,
        content,
        title: title || undefined,
        tag_ids: selectedTagIds,
        first_comment: firstComment || undefined,
        media: mediaItems.length > 0 ? mediaItems.map(m => ({ id: m.id, url: m.url, mime_type: m.mime_type, alt: undefined })) : undefined,
        overrides: Object.keys(overridesObj).length > 0 ? overridesObj : undefined,
        settings: {
          ...settings,
          ...(selectedMusic ? { audio_id: selectedMusic.id, audio_title: selectedMusic.title } : {}),
        },
      };

      // Pre-submit validation
      const valRes = await postsApi.validate(payload);
      if (valRes.data && !valRes.data.valid && valRes.data.errors.length > 0) {
        const firstErr = valRes.data.errors[0];
        error = firstErr.provider_name + ": " + firstErr.message;
        submitting = false;
        return;
      }

      const r = await postsApi.create(payload);
      if (r.error) { error = r.error; return; }
      if (r.data?.posts?.[0]?.id) {
        const pub = await postsApi.publish(r.data.posts[0].id);
        if (pub.error) { error = pub.error; return; }
      }
      localStorage.removeItem(DRAFT_KEY);
      goto("/calendar");
    } catch (e: unknown) {
      error = (e instanceof Error ? e.message : String(e)) || "Failed to post";
    } finally {
      submitting = false;
    }
  }

  function insertAiText(text: string) {
    content = content + (content ? "\n\n" : "") + text;
  }

  function addHashtag(tag: string) {
    content = content + (content.endsWith(" ") ? "" : " ") + "#" + tag;
  }
</script>

<div class="page-enter max-w-4xl mx-auto space-y-6">
  {#if showRestorePrompt}
    <div class="bg-indigo-500/10 border border-indigo-500/30 rounded-lg p-3 flex items-center justify-between">
      <span class="text-sm text-indigo-300">You have an unsaved draft. Restore it?</span>
      <div class="flex gap-2">
        <button onclick={restoreDraft} class="px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-500 rounded transition-colors">Restore</button>
        <button onclick={dismissDraft} class="px-3 py-1 text-xs text-muted hover:text-white transition-colors">Dismiss</button>
      </div>
    </div>
  {/if}

  <div class="sticky top-0 z-20 bg-background -mx-6 px-6 py-3 border-b border-line flex items-center justify-between">
    <h2 class="text-xl font-semibold">Create Post</h2>
    <div class="flex items-center gap-3">
      {#if draftSaved}
        <span class="text-xs text-emerald-400 animate-pulse">✓ Draft saved</span>
      {/if}
      <span class="hidden lg:inline text-[10px] text-muted-dark" title="Keyboard shortcuts">
        <kbd class="px-1 py-0.5 bg-surface-hover rounded">⌘</kbd>+<kbd class="px-1 py-0.5 bg-surface-hover rounded">↵</kbd> post ·
        <kbd class="px-1 py-0.5 bg-surface-hover rounded">⌘</kbd>+<kbd class="px-1 py-0.5 bg-surface-hover rounded">S</kbd> save
      </span>
      <div class="flex gap-2">
      <button onclick={() => (showPostSets = true)} aria-label="Post Sets" class="px-3 py-1.5 text-sm text-muted hover:text-white border border-line rounded-lg transition-colors">Post Sets</button>
      <button onclick={() => showAi = !showAi} aria-label="AI Assistant"
        class="px-3 py-1.5 text-sm border border-line rounded-lg transition-colors
          {showAi ? 'bg-indigo-600/20 text-indigo-400 border-indigo-500/30' : 'text-muted hover:text-white'}">
        ✨ AI
      </button>
      <button onclick={() => goto("/calendar")} class="px-3 py-1.5 text-sm text-muted hover:text-white border border-line rounded-lg">Cancel</button>
      <button onclick={postNow} disabled={submitting} class="px-4 py-1.5 bg-green-600 hover:bg-green-500 disabled:opacity-50 rounded-lg text-sm transition-colors">
        {submitting ? "Posting..." : "Post Now"}
      </button>
      <button onclick={submit} disabled={submitting} class="px-4 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg text-sm transition-colors">
        {submitting ? "Scheduling..." : "Schedule"}
      </button>
    </div>
    </div>
  </div>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3">{error}</div>
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

  <!-- Posting Targets (shown when selected integrations have discoverable targets) -->
  {#each selectedIntegrations as intId}
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

  <!-- Global vs Internal channel editor toggle -->
  {#if selectedIntegrations.length > 1}
    <div class="bg-surface border border-line rounded-xl p-3">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="text-xs text-muted mr-2">Editing:</span>
        <button
          onclick={() => editingMode = "global"}
          class="px-3 py-1.5 text-xs rounded-lg transition-colors {editingMode === 'global' ? 'bg-brand-500 text-white' : 'bg-surface-hover text-muted hover:text-white'}"
        >
          🌐 Global (all channels)
        </button>
        {#each selectedIntegrations as intId, i}
          <button
            onclick={() => editingMode = "internal:" + intId}
            class="px-3 py-1.5 text-xs rounded-lg transition-colors flex items-center gap-1 {editingMode === 'internal:' + intId ? 'bg-brand-500 text-white' : 'bg-surface-hover text-muted hover:text-white'}"
          >
            {#if providerOverride.has(intId)}
              <span class="w-1.5 h-1.5 rounded-full bg-pink-400"></span>
            {/if}
            {integrationNames.get(intId) || "Channel " + (i + 1)}
          </button>
        {/each}
      </div>
      {#if editingMode !== "global"}
        <div class="mt-2 flex items-center justify-between">
          <p class="text-xs text-pink-400">
            ✏️ Editing custom content for {integrationNames.get(editingMode.split(":")[1]) || "this channel"}
          </p>
          <button
            onclick={() => {
              const intId = editingMode.split(":")[1];
              providerOverride.delete(intId);
              editingMode = "global";
            }}
            class="text-xs text-muted hover:text-white underline"
          >
            Reset to global
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Main content editor — switches between global and internal mode -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">
      {#if editingMode === "global"}
        Content (shared across all channels)
      {:else}
        Content for {integrationNames.get(editingMode.split(":")[1]) || "this channel"}
      {/if}
    </h3>
    {#if editingMode === "global"}
      <RichTextEditor {content} onUpdate={(html) => content = html} />
      <!-- Per-platform char counters: shows X=280, Threads=500, etc. for
           each selected channel so the user knows when shared content
           will exceed any platform's limit. -->
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
        content={providerOverride.get(editingMode.split(":")[1]) || content}
        onUpdate={(html) => {
          const intId = editingMode.split(":")[1];
          providerOverride.set(intId, html);
          providerOverride = new Map(providerOverride); // trigger reactivity
        }}
      />
      <!-- Per-platform char counter for the override content too -->
      <div class="mt-3 pt-3 border-t border-line">
        <PerPlatformCharCount
          content={providerOverride.get(editingMode.split(":")[1]) || content}
          selectedIntegrations={[editingMode.split(":")[1]]}
          {integrationProviders}
          {integrationNames}
        />
      </div>
    {/if}
  </div>

  <!-- AI Assistant Panel -->
  {#if showAi}
    <AiAssistant {content} onInsert={insertAiText} />
  {/if}

  <!-- AI Hashtag Suggestions -->
  <AiHashtagSuggestions {content} onAddHashtag={addHashtag} />

  <!-- Per-provider content overrides — replaced by Global/Internal toggle above -->

  <!-- Media -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Media</h3>
    <MediaUpload items={mediaItems}
      onAdd={(item) => mediaItems = [...mediaItems, item]}
      onRemove={(id) => mediaItems = mediaItems.filter(m => m.id !== id)}
    />
  </div>

  <!-- Music (Instagram only — uses IG Audio API) -->
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
          <button
            onclick={() => selectedMusic = null}
            class="text-muted hover:text-red-400 text-sm"
          >
            &times;
          </button>
        </div>
      {:else}
        <p class="text-xs text-muted">Add trending music from Instagram's library to your Reel.</p>
      {/if}
    </div>
  {/if}

  <!-- Scheduling -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Schedule</h3>
    <SchedulePicker {scheduledAt} onChange={(iso: string | null) => scheduledAt = iso} {recurring} onRecurringChange={(r: { intervalDays: number; endDate: string } | null) => recurring = r} integrationId={selectedIntegrations[0]} />
  </div>

  <!-- Thread Finisher (visible when X/Twitter is selected) -->
  {#if hasXIntegration}
    <ThreadFinisher {content} onCreateThread={handleCreateThread} {submitting} />
  {/if}

  <!-- First Comment (visible when LinkedIn/Facebook is selected) -->
  <FirstComment
    {selectedIntegrations}
    {integrationProviders}
    firstComment={firstComment}
    onFirstCommentChange={(text) => firstComment = text}
  />

  <!-- Preview -->
  {#if selectedIntegrations.length > 0}
    <PostPreview {content} selectedIntegrations={selectedIntegrations} {integrationProviders} />
  {/if}
</div>

<PostSetModal
  open={showPostSets}
  onclose={() => (showPostSets = false)}
  currentContent={content}
  currentChannelIds={selectedIntegrations}
  currentScheduleAt={scheduledAt}
  onLoad={handlePostSetLoad}
/>

{#if showMusicPicker}
  <MusicPicker
    integrationId={instagramIntegrationId}
    onSelect={(track) => selectedMusic = track}
    onclose={() => showMusicPicker = false}
  />
{/if}
