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
  import Skeleton from '$lib/ui/Skeleton.svelte';
  import ErrorState from '$lib/ui/ErrorState.svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { postsApi, type PostDetail } from '$lib/api/posts';
  import { realtime } from '$lib/stores/realtime';
  import { timezone } from '$lib/stores/timezone.svelte';
  import { composer } from '$lib/stores/composer.svelte';
  import { modals } from '$lib/stores/modals.svelte';
  import { inkOnFill } from '$lib/chip-colors';
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
    // `post_updated` covers edits made in another tab; `lagged` is the
    // server's synthetic "you missed events" signal (refetch stale view).
    const events = ['post_published', 'post_failed', 'post_deleted', 'post_updated', 'lagged'];
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
    <button onclick={() => goto('/posts')} class="text-sm text-muted hover:text-content flex items-center gap-1 min-h-11">
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="19" y1="12" x2="5" y2="12"/><polyline points="12 19 5 12 12 5"/></svg>
      Back to Posts
    </button>
    {#if post}
      <div class="flex gap-2">
        <button
          onclick={() => composer.openEdit(post.id)}
          class="px-3 py-1.5 text-sm bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg transition-colors"
        >✏️ Edit</button>
        <button
          onclick={handleDelete}
          class="px-3 py-1.5 text-sm text-error hover:text-error border border-line rounded-lg transition-colors"
        >🗑️ Delete</button>
      </div>
    {/if}
  </div>

  {#if loading}
    <Skeleton variant="text" rows={6} />
  {:else if error}
    <ErrorState message={error} actionLabel="Retry" onaction={load} />
  {:else if post}
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <!--
        The page's ONE h1. The detail view had no heading at all: the
        title was an <h2> under a page with no <h1>, so the document
        outline started at level 2 and the probe's "no <h1>" check
        failed on this route.

        The h1 IS the post title, because that is what the page is: one
        post. When the post has no title (a Bluesky or Threads post
        often does not), the first line of the body is the heading, and
        a body that is only whitespace falls back to a generic label.
      -->
      <h1 class="text-lg font-semibold">
        {post.title || post.content.trim().split('\n')[0] || 'Post'}
      </h1>

      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Badge state={post.state as "draft" | "queued" | "published" | "error"} />
          <span class="text-sm text-muted">{post.integration_name}</span>
        </div>
        {#if post.scheduled_at}
          <span class="text-xs text-muted">{timezone.formatDateTime(post.scheduled_at)}</span>
        {/if}
      </div>

      <!--
        The post body is rendered as TEXT, not as HTML.

        It used to be `{@html post.content}`, which is an XSS hole: the
        content comes from a provider, from a webhook, or from anything
        an API client posted, and rendering it as markup lets any of
        them inject script into the founder's own dashboard. The
        server stripped the tags on the way in, which is what hid the
        hole - and that stripping is also why a post that mentioned a
        tag lost the words between the angle brackets.

        The content is plain text now (the editor speaks plain text at
        its boundary, the same contract the server and the providers
        use), so rendering it as text is both safe and correct.
      -->
      <div class="text-sm text-content-secondary whitespace-pre-wrap break-words leading-relaxed">
        {post.content}
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
            <!-- A solid fill with the ink computed from it. The old form
                 painted the tag colour as TEXT on a 12%-alpha wash of
                 itself, which measures 4.01:1 for a mid-tone hue - under
                 the WCAG AA floor for small text. Black or white on a
                 solid fill always clears 4.5:1. -->
            <span class="px-2 py-0.5 rounded-full text-xs" style="background: {tag.color || 'var(--brand-soft)'}; color: {tag.color ? inkOnFill(tag.color) : 'var(--accent)'}">#{tag.name}</span>
          {/each}
        </div>
      {/if}

      {#if post.platform_post_url}
        <div class="border-t border-line pt-3">
          <a href={post.platform_post_url} target="_blank" rel="noopener" class="text-xs text-accent hover:underline">
            View original post →
          </a>
        </div>
      {/if}

      {#if post.error_message}
        <div class="border-t border-line pt-3">
          <div class="text-xs text-error bg-error/10 border border-error/30 rounded-lg p-2">
            {post.error_message}
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
