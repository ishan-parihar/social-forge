<script lang="ts">
  // Phase 8: /posts/[id] is a read-only detail view. The Edit button
  // opens the composer modal (composer.openEdit(id)). The full edit
  // logic lives in lib/composer/ComposerModal.svelte.
  //
  // This route is kept for direct-link compatibility (e.g., bookmarks,
  // the browser address bar, the "View original" link from other pages).
  // The primary edit flow is the modal opened via composer.openEdit()
  // from anywhere.

  import { onMount, onDestroy } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { postsApi, type PostDetail } from '$lib/api/posts';
  import { realtime } from '$lib/stores/realtime';
  import { timezone } from '$lib/stores/timezone.svelte';
  import { composer } from '$lib/stores/composer.svelte';
  import { modals } from '$lib/stores/modals.svelte';
  import Badge from '$lib/ui/Badge.svelte';
  import Icon from '$lib/ui/Icon.svelte';

  let post = $state<PostDetail | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let unsubscribers: (() => void)[] = [];

  let postId = $derived($page.params.id);

  async function load() {
    loading = true;
    error = null;
    const r = await postsApi.get(postId);
    if (r.data) {
      post = r.data;
    } else {
      error = r.error || 'Post not found';
    }
    loading = false;
  }

  onMount(() => {
    load();
    const events = ['post_published', 'post_failed', 'post_deleted'];
    for (const evt of events) {
      unsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });

  async function handleDelete() {
    if (!post) return;
    // Phase v21: replace native confirm() with modals.areYouSure for
    // consistent UX with the calendar + posts list.
    const ok = await modals.areYouSure({
      title: 'Delete this post?',
      message: 'The post will be soft-deleted. It will be hidden from the calendar and posts list, but can be recovered from the Trash (coming in v22).',
      confirmLabel: 'Delete',
      cancelLabel: 'Cancel',
      danger: true,
    });
    if (!ok) return;
    const r = await postsApi.delete(post.id);
    if (r.error) {
      error = r.error;
    } else {
      goto('/posts');
    }
  }
</script>

<div class="page-enter max-w-2xl mx-auto space-y-6">
  <!-- Back + actions -->
  <div class="flex items-center justify-between">
    <button onclick={() => goto('/posts')} class="text-sm text-muted hover:text-content flex items-center gap-1">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
      Back to Posts
    </button>
    {#if post}
      <div class="flex gap-2">
        <button
          onclick={() => composer.openEdit(post.id)}
          class="px-3 py-1.5 text-sm bg-indigo-600 hover:bg-indigo-500 rounded-lg transition-colors"
        >✏️ Edit</button>
        <button
          onclick={handleDelete}
          class="px-3 py-1.5 text-sm text-red-400 hover:text-red-300 border border-line rounded-lg transition-colors"
        >🗑️ Delete</button>
      </div>
    {/if}
  </div>

  {#if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if post}
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
          <span class="text-sm text-muted">{post.integration_name}</span>
        </div>
        {#if post.scheduled_at}
          <span class="text-xs text-muted">{timezone.formatDateTime(post.scheduled_at)}</span>
        {/if}
      </div>

      {#if post.title}
        <h2 class="text-lg font-semibold">{post.title}</h2>
      {/if}

      <div class="text-sm text-content-secondary whitespace-pre-wrap break-words leading-relaxed">
        {@html post.content}
      </div>

      {#if post.first_comment}
        <div class="border-t border-line pt-3">
          <div class="text-xs text-muted mb-1">First comment:</div>
          <div class="text-sm text-content-secondary">{post.first_comment}</div>
        </div>
      {/if}

      {#if post.tags && post.tags.length > 0}
        <div class="flex gap-1 flex-wrap border-t border-line pt-3">
          {#each post.tags as tag (tag.id)}
            <span class="px-2 py-0.5 rounded-full text-xs" style="background: {tag.color || '#4f46e5'}20; color: {tag.color || '#4f46e5'}">#{tag.name}</span>
          {/each}
        </div>
      {/if}

      {#if post.platform_post_url}
        <div class="border-t border-line pt-3">
          <a href={post.platform_post_url} target="_blank" rel="noopener" class="text-xs text-indigo-400 hover:underline">
            View original post →
          </a>
        </div>
      {/if}

      {#if post.error_message}
        <div class="border-t border-line pt-3">
          <div class="text-xs text-red-400 bg-red-500/10 border border-red-500/30 rounded-lg p-2">
            {post.error_message}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
