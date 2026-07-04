<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { postsApi } from "$lib/api/posts";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount } from "svelte";
  import { toast } from "$lib/stores/toast";
  import ChannelSelector from "$lib/composer/ChannelSelector.svelte";
  import RichTextEditor from "$lib/composer/RichTextEditor.svelte";
  import MediaUpload from "$lib/composer/MediaUpload.svelte";
  import SchedulePicker from "$lib/composer/SchedulePicker.svelte";
  import ProviderEditor from "$lib/composer/ProviderEditor.svelte";
  import PostPreview from "$lib/composer/PostPreview.svelte";
  import TagPicker from "$lib/composer/TagPicker.svelte";
  import PostSetModal from "$lib/composer/PostSetModal.svelte";
  import ThreadFinisher from "$lib/composer/ThreadFinisher.svelte";
  import FirstComment from "$lib/composer/FirstComment.svelte";
  import AiAssistant from "$lib/composer/AiAssistant.svelte";
  import AiHashtagSuggestions from "$lib/composer/AiHashtagSuggestions.svelte";
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
  let activeProvider = $state<string | null>(null);
  let selectedTagIds = $state<string[]>([]);
  let submitting = $state(false);
  let error = $state<string | null>(null);
  let firstComment = $state("");
  let showAi = $state(false);

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

  const DRAFT_KEY = 'social-forge-composer-draft';
  let autoSaveTimer: ReturnType<typeof setTimeout>;

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

  function handlePostSetLoad(set: { content: string; channelIds: string[]; scheduledAt: string | null }) {
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
      toast(`Failed to fetch targets for ${integrationId}: ${e instanceof Error ? (e instanceof Error ? e.message : String(e)) : "unknown"}`, "error");
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
      const r = await postsApi.create({
        integration_ids: selectedIntegrations,
        content,
        title: title || undefined,
        scheduled_at: scheduledAt || undefined,
        tag_ids: selectedTagIds,
        first_comment: firstComment || undefined,
        media: mediaItems.length > 0 ? mediaItems.map(m => ({ id: m.id, url: m.url, mime_type: m.mime_type, alt: undefined })) : undefined,
        overrides: Object.keys(overridesObj).length > 0 ? overridesObj : undefined,
        settings: Object.keys(settings).length > 0 ? settings : undefined,
      });
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
      const r = await postsApi.create({
        integration_ids: selectedIntegrations,
        content,
        title: title || undefined,
        tag_ids: selectedTagIds,
        first_comment: firstComment || undefined,
        media: mediaItems.length > 0 ? mediaItems.map(m => ({ id: m.id, url: m.url, mime_type: m.mime_type, alt: undefined })) : undefined,
        overrides: Object.keys(overridesObj).length > 0 ? overridesObj : undefined,
        settings: Object.keys(settings).length > 0 ? settings : undefined,
      });
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

<div class="page-enter page-enter max-w-4xl mx-auto space-y-6">
  {#if showRestorePrompt}
    <div class="bg-indigo-500/10 border border-indigo-500/30 rounded-lg p-3 flex items-center justify-between">
      <span class="text-sm text-indigo-300">You have an unsaved draft. Restore it?</span>
      <div class="flex gap-2">
        <button onclick={restoreDraft} class="px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-500 rounded transition-colors">Restore</button>
        <button onclick={dismissDraft} class="px-3 py-1 text-xs text-muted hover:text-white transition-colors">Dismiss</button>
      </div>
    </div>
  {/if}

  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Create Post</h2>
    <div class="flex items-center gap-3">
      {#if draftSaved}
        <span class="text-xs text-emerald-400 animate-pulse">✓ Draft saved</span>
      {/if}
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

  <!-- Main content -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Content</h3>
    <RichTextEditor {content} onUpdate={(html) => content = html} />
  </div>

  <!-- AI Assistant Panel -->
  {#if showAi}
    <AiAssistant {content} onInsert={insertAiText} />
  {/if}

  <!-- AI Hashtag Suggestions -->
  <AiHashtagSuggestions {content} onAddHashtag={addHashtag} />

  <!-- Per-provider content overrides -->
  {#if selectedIntegrations.length > 1}
    <div class="bg-surface border border-line rounded-xl p-4">
      <h3 class="text-sm font-semibold mb-3">Platform-Specific Content</h3>
      <p class="text-xs text-muted mb-3">Customize content for each platform.</p>
      <div class="page-enter space-y-2">
        {#each selectedIntegrations as intId, i}
          <div class="border border-line rounded-lg">
            <button
              onclick={() => activeProvider = activeProvider === intId ? null : intId}
              class="w-full flex items-center gap-2 px-3 py-2 text-left text-sm hover:bg-surface-hover transition-colors"
            >
              <span class="flex-1">{integrationNames.get(intId) || `Platform ${i + 1}`}</span>
              <span class="text-muted text-xs">{activeProvider === intId ? "▾" : "▸"}</span>
            </button>
            {#if activeProvider === intId}
              <div class="px-3 pb-3">
                <ProviderEditor
                  provider={integrationProviders.get(intId) || intId}
                  content={providerOverride.get(intId) || content}
                  onContentChange={(html) => providerOverride.set(intId, html)}
                  integrationId={intId}
                />
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- Media -->
  <div class="bg-surface border border-line rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Media</h3>
    <MediaUpload items={mediaItems}
      onAdd={(item) => mediaItems = [...mediaItems, item]}
      onRemove={(id) => mediaItems = mediaItems.filter(m => m.id !== id)}
    />
  </div>

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
