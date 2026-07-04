<script lang="ts">
  import { onMount } from "svelte";
  import { postsApi, type PostDetail } from "$lib/api/posts";
  import { tagsApi, type Tag } from "$lib/api/tags";
  import { toast } from "$lib/stores/toast";
  import { page } from "$app/stores";
  import { goto } from "$app/navigation";
  import Badge from "$lib/ui/Badge.svelte";
  import Icon from "$lib/ui/Icon.svelte";
  import RichTextEditor from "$lib/composer/RichTextEditor.svelte";

  let post = $state<PostDetail | null>(null);
  let editing = $state(false);
  let editContent = $state("");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let allTags = $state<Tag[]>([]);
  let showTagPicker = $state(false);
  let selectedTagIds = $state<string[]>([]);

  async function loadTags() {
    const r = await tagsApi.list();
    if (r.data) allTags = r.data;
  }

  async function saveTags() {
    if (!post) return;
    const r = await postsApi.setTags(post.id, selectedTagIds);
    if (r.error) {
      toast(`Failed to save tags: ${r.error}`, "error");
    } else {
      toast("Tags updated", "success");
      // Reload post to get updated tags
      const detail = await postsApi.get(post.id);
      if (detail.data) post = detail.data;
      showTagPicker = false;
    }
  }

  function openTagPicker() {
    selectedTagIds = post?.tags?.map(t => t.id) || [];
    showTagPicker = true;
  }

  function toggleTag(id: string) {
    if (selectedTagIds.includes(id)) {
      selectedTagIds = selectedTagIds.filter(t => t !== id);
    } else {
      selectedTagIds = [...selectedTagIds, id];
    }
  }

  onMount(async () => {
    const id = $page.params.id;
    if (!id) { loading = false; return; }
    loadTags();
    const r = await postsApi.get(id);
    if (r.data) { post = r.data; editContent = r.data.content; }
    else error = r.error || "Failed to load post";
    loading = false;
  });

  async function save() {
    if (!post) return;
    error = null;
    const r = await postsApi.update(post.id, { content: editContent });
    if (r.error) { error = r.error; return; }
    post.content = editContent;
    editing = false;
  }

  let showScheduleForm = $state(false);
  let schedDate = $state("");
  let schedTime = $state("09:00");

  async function schedulePost() {
    if (!post) return;
    showScheduleForm = true;
  }

  async function confirmSchedule() {
    if (!post || !schedDate) return;
    error = null;
    const iso = `${schedDate}T${schedTime}:00.000Z`;
    const r = await postsApi.schedule(post.id, iso);
    if (r.data) { post.state = "queued"; post.scheduled_at = iso; showScheduleForm = false; }
    else error = r.error || "Failed to schedule post";
  }

  async function deletePost() {
    if (!post || !confirm("Delete this post?")) return;
    error = null;
    const r = await postsApi.delete(post.id);
    if (r.error) { error = r.error; return; }
    goto("/posts");
  }
</script>

<div class="page-enter page-enter max-w-2xl mx-auto space-y-6">
  <button onclick={() => goto("/posts")} class="text-sm text-muted hover:text-white">&larr; Back to posts</button>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3 flex items-center justify-between">
      <span>{error}</span>
      <button onclick={() => error = null} class="text-red-400/70 hover:text-red-400">&times;</button>
    </div>
  {/if}

  {#if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if post}
    <div class="bg-surface border border-line rounded-xl p-6 space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
          <span class="text-sm text-muted">{post.integration_id}</span>
        </div>
        <div class="flex gap-2">
          {#if !editing}
            <button onclick={() => editing = true} class="text-xs text-indigo-400 hover:underline">Edit</button>
            {#if post.state === "draft" || post.state === "queued"}
              <button onclick={schedulePost} class="text-xs text-indigo-400 hover:underline">{post.state === "queued" ? "Reschedule" : "Schedule"}</button>
            {/if}
            <button onclick={deletePost} class="text-xs text-red-400 hover:underline">Delete</button>
          {:else}
            <button onclick={save} class="text-xs text-green-400 hover:underline">Save</button>
            <button onclick={() => editing = false} class="text-xs text-muted hover:underline">Cancel</button>
          {/if}
        </div>
      </div>

      {#if post.tags && post.tags.length > 0}
        <div class="flex flex-wrap gap-1.5 items-center">
          {#each post.tags as tag (tag.id)}
            <span
              class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs"
              style="background: {tag.color}22; color: {tag.color}; border: 1px solid {tag.color}44"
            >
              <span class="w-1.5 h-1.5 rounded-full" style="background: {tag.color}"></span>
              {tag.name}
            </span>
          {/each}
          <button onclick={openTagPicker} class="text-xs text-muted hover:text-indigo-400 transition-colors flex items-center gap-1">
            <Icon name="tag" class="w-3 h-3" />
            Edit
          </button>
        </div>
      {:else}
        <button onclick={openTagPicker} class="text-xs text-muted hover:text-indigo-400 transition-colors flex items-center gap-1">
          <Icon name="tag" class="w-3 h-3" />
          Add Tags
        </button>
      {/if}

      {#if post.scheduled_at}
        <div class="text-sm text-muted">
          Scheduled: {new Date(post.scheduled_at).toLocaleString()}
        </div>
      {/if}

      {#if showScheduleForm}
        <div class="flex items-center gap-2 bg-background-input border border-line rounded-lg p-3">
          <input type="date" bind:value={schedDate} class="px-2 py-1 bg-surface border border-line rounded text-sm text-content-secondary" />
          <input type="time" bind:value={schedTime} class="px-2 py-1 bg-surface border border-line rounded text-sm text-content-secondary" />
          <button onclick={confirmSchedule} class="px-3 py-1 bg-indigo-600 hover:bg-indigo-500 rounded text-xs">Confirm</button>
          <button onclick={() => showScheduleForm = false} class="px-3 py-1 text-muted hover:text-white text-xs">Cancel</button>
        </div>
      {/if}

      {#if editing}
        <RichTextEditor content={editContent} onUpdate={(html: string) => editContent = html} />
      {:else}
        <div class="prose prose-invert max-w-none text-sm">{@html post.content}</div>
      {/if}

      {#if post.first_comment}
        <div class="border-t border-line pt-3 mt-3">
          <div class="text-xs text-muted mb-1">First Comment</div>
          <div class="text-sm text-content-secondary">{post.first_comment}</div>
        </div>
      {/if}

      {#if post.group_id}
        <div class="border-t border-line pt-3 mt-3">
          <div class="text-xs text-indigo-400 mb-1">Thread post (sequence {post.sequence ?? 0})</div>
          <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-xs bg-indigo-500/20 text-indigo-400">
            Group: {post.group_id.slice(0, 8)}...
          </span>
        </div>
      {/if}

      {#if post.platform_post_url}
        <a href={post.platform_post_url} target="_blank" class="text-sm text-indigo-400 hover:underline inline-block">
          View on platform &rarr;
        </a>
      {/if}

      {#if post.error_message}
        <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3">
          Error: {post.error_message}
        </div>
      {/if}
    </div>
  {:else}
    <div class="text-center py-12 text-sm text-muted">Post not found</div>
  {/if}
</div>

<!-- Tag Picker Modal -->
{#if showTagPicker}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background border border-line rounded-xl p-6 w-full max-w-md">
      <h3 class="text-lg font-semibold mb-4">Manage Tags</h3>
      {#if allTags.length === 0}
        <p class="text-sm text-muted py-4 text-center">No tags available. Create tags in the Tags page first.</p>
      {:else}
        <div class="space-y-2 max-h-60 overflow-y-auto">
          {#each allTags as tag (tag.id)}
            <label class="flex items-center gap-2 text-sm cursor-pointer p-2 rounded-lg hover:bg-surface-hover transition-colors">
              <input
                type="checkbox"
                checked={selectedTagIds.includes(tag.id)}
                onchange={() => toggleTag(tag.id)}
                class="rounded"
              />
              <span class="w-2 h-2 rounded-full" style="background: {tag.color}"></span>
              <span>{tag.name}</span>
            </label>
          {/each}
        </div>
      {/if}
      <div class="flex gap-3 justify-end mt-4">
        <button onclick={() => showTagPicker = false} class="px-4 py-2 text-sm text-muted hover:text-content">Cancel</button>
        <button onclick={saveTags} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded-lg transition-colors">Save Tags</button>
      </div>
    </div>
  </div>
{/if}
