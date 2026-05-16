<script lang="ts">
  import { goto } from "$app/navigation";
  import { postsApi } from "$lib/api/posts";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { onMount } from "svelte";
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

  // Auto-detect if X/Twitter is selected (for thread mode)
  let hasXIntegration = $derived(
    selectedIntegrations.some(id => integrationProviders.get(id) === 'x')
  );

  onMount(async () => {
    const r = await integrationsApi.list();
    if (r.data) allIntegrations = r.data.integrations.filter(i => !i.disabled);
  });

  let providerOverride = $state<Map<string, string>>(new Map());
  let showPostSets = $state(false);

  function handlePostSetLoad(set: { content: string; channelIds: string[]; scheduledAt: string | null }) {
    if (set.content) content = set.content;
    if (set.channelIds.length > 0) selectedIntegrations = set.channelIds;
    if (set.scheduledAt) scheduledAt = set.scheduledAt;
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
    } catch (e: any) {
      error = e.message || "Failed to create thread";
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
      const r = await postsApi.create({
        integration_ids: selectedIntegrations,
        content,
        title: title || undefined,
        scheduled_at: scheduledAt || undefined,
        tag_ids: selectedTagIds,
        first_comment: firstComment || undefined,
      });
      if (r.error) {
        error = r.error;
        submitting = false;
        return;
      }
      goto("/calendar");
    } catch (e: any) {
      error = e.message || "Failed to create post";
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

<div class="max-w-4xl mx-auto space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Create Post</h2>
    <div class="flex gap-2">
      <button onclick={() => (showPostSets = true)} aria-label="Post Sets" class="px-3 py-1.5 text-sm text-[#6b7280] hover:text-white border border-[#1e2435] rounded-lg transition-colors">Post Sets</button>
      <button onclick={() => showAi = !showAi} aria-label="AI Assistant"
        class="px-3 py-1.5 text-sm border border-[#1e2435] rounded-lg transition-colors
          {showAi ? 'bg-indigo-600/20 text-indigo-400 border-indigo-500/30' : 'text-[#6b7280] hover:text-white'}">
        ✨ AI
      </button>
      <button onclick={() => goto("/calendar")} class="px-3 py-1.5 text-sm text-[#6b7280] hover:text-white border border-[#1e2435] rounded-lg">Cancel</button>
      <button onclick={submit} disabled={submitting} class="px-4 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg text-sm transition-colors">
        {submitting ? "Publishing..." : "Publish"}
      </button>
    </div>
  </div>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3">{error}</div>
  {/if}

  <!-- Title -->
  <div>
    <label class="text-sm text-[#6b7280] block mb-1">Title (optional)</label>
    <input type="text" bind:value={title} placeholder="Post title..."
      class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none" />
  </div>

  <!-- Channel Selection -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
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

  <!-- Tags -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
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
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
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
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
      <h3 class="text-sm font-semibold mb-3">Platform-Specific Content</h3>
      <p class="text-xs text-[#6b7280] mb-3">Customize content for each platform.</p>
      <div class="space-y-2">
        {#each selectedIntegrations as intId, i}
          <div class="border border-[#1e2435] rounded-lg">
            <button
              onclick={() => activeProvider = activeProvider === intId ? null : intId}
              class="w-full flex items-center gap-2 px-3 py-2 text-left text-sm hover:bg-[#1a1f2e] transition-colors"
            >
              <span class="flex-1">{integrationNames.get(intId) || `Platform ${i + 1}`}</span>
              <span class="text-[#6b7280] text-xs">{activeProvider === intId ? "▾" : "▸"}</span>
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
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Media</h3>
    <MediaUpload items={mediaItems}
      onAdd={(item) => mediaItems = [...mediaItems, item]}
      onRemove={(id) => mediaItems = mediaItems.filter(m => m.id !== id)}
    />
  </div>

  <!-- Scheduling -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
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
