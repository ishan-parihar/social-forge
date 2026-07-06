<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { commentsApi, type Comment } from "$lib/api/comments";
  import { integrationsApi, type Integration } from "$lib/api/integrations";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";

  let comments = $state<Comment[]>([]);
  let connectedIntegrations = $state<Integration[]>([]);
  let filterPlatform = $state("all");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let replyModal = $state<{ comment: Comment; text: string } | null>(null);
  let sending = $state(false);

  const statuses = ["all", "new", "resolved"];

  // Build platform list from connected integrations
  let platforms = $derived.by(() => {
    const connected = connectedIntegrations
      .filter(i => !i.disabled)
      .map(i => i.provider_identifier);
    const unique = [...new Set(connected)];
    return ["all", ...unique];
  });

  async function load() {
    loading = true;
    error = null;
    const r = await commentsApi.list({
      ...(filterPlatform !== "all" && { platform: filterPlatform }),
    });
    if (r.data) {
      comments = r.data.comments;
    } else {
      error = r.error || "Failed to load comments";
    }
    loading = false;
  }

  async function resolveComment(id: string) {
    const r = await commentsApi.resolve(id);
    if (r.error) {
      toast("Failed to resolve: " + r.error, "error");
    } else {
      toast("Comment resolved", "success");
      load();
    }
  }

  async function sendReply() {
    if (!replyModal || !replyModal.text.trim()) return;
    sending = true;
    const r = await commentsApi.reply(replyModal.comment.id, replyModal.text);
    if (r.error) {
      toast("Reply failed: " + r.error, "error");
    } else {
      toast("Reply sent", "success");
      replyModal = null;
    }
    sending = false;
  }

  function platformIcon(p: string): string {
    const icons: Record<string, string> = { x: "X", reddit: "R", linkedin: "in", facebook: "f", instagram: "IG" };
    return icons[p] || p.slice(0, 2).toUpperCase();
  }

  let commentsUnsubscribers: (() => void)[] = [];

  onMount(async () => {
    const integRes = await integrationsApi.list();
    if (integRes.data) connectedIntegrations = integRes.data.integrations;
    load();
    // Refresh when a new comment arrives (realtime SSE event) — this is the
    // event the backend actually broadcasts. The previous subscription to
    // `post_published`/`post_created` never fired for comment activity.
    for (const evt of ['comment_received', 'post_published']) {
      commentsUnsubscribers.push(realtime.on(evt, () => load()));
    }
  });

  onDestroy(() => {
    commentsUnsubscribers.forEach(fn => fn());
  });
</script>

<div class="page-enter space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Comments</h2>
    <button onclick={load} class="px-3 py-1.5 text-sm text-muted hover:text-white border border-line rounded-lg transition-colors">Refresh</button>
  </div>

  <!-- Filters -->
  <div class="flex gap-2 flex-wrap">
    {#each platforms as p}
      <button
        onclick={() => { filterPlatform = p; load(); }}
        class="px-3 py-1.5 text-xs capitalize rounded-lg transition-colors {filterPlatform === p ? 'bg-indigo-600 text-white' : 'bg-surface text-muted hover:text-white border border-line'}"
      >{p}</button>
    {/each}
  </div>

  <!-- Content -->
  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-muted">Loading comments...</div>
  {:else if comments.length === 0}
    <div class="text-center py-12">
      <p class="text-sm text-muted mb-2">No comments found</p>
      <p class="text-xs text-muted-dark">Comments are fetched from connected platforms that support the comments API (Instagram, Facebook, LinkedIn).</p>
    </div>
  {:else}
    <div class="bg-surface border border-line rounded-xl overflow-hidden">
      <div class="grid grid-cols-[40px_1fr_1.5fr_100px_100px_90px] gap-3 px-4 py-2 border-b border-line bg-background-input text-xs text-muted">
        <span></span><span>Post</span><span>Comment</span><span>Author</span><span>Date</span><span>Status</span>
      </div>
      {#each comments as c (c.id)}
        <div class="grid grid-cols-[40px_1fr_1.5fr_100px_100px_90px] gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors items-center">
          <span class="text-sm text-indigo-400">{platformIcon(c.platform)}</span>
          <span class="text-sm truncate text-muted" title={c.post_content}>{c.post_content?.slice(0, 50) || c.post_id}</span>
          <span class="text-sm text-content-secondary truncate">{c.content}</span>
          <span class="text-xs text-muted truncate">{c.author || 'Unknown'}</span>
          <span class="text-xs text-muted">{new Date(c.created_at).toLocaleDateString()}</span>
          <div class="flex items-center gap-2">
            {#if c.status !== 'resolved'}
              <span class="px-2 py-0.5 text-xs rounded bg-yellow-500/20 text-yellow-400">New</span>
              <button onclick={() => resolveComment(c.id)} class="text-xs text-muted hover:text-green-400" title="Resolve">✓</button>
              <button onclick={() => replyModal = { comment: c, text: "" }} class="text-xs text-muted hover:text-indigo-400" title="Reply">↩</button>
            {:else}
              <span class="px-2 py-0.5 text-xs rounded bg-green-500/20 text-green-400">Resolved</span>
            {/if}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Reply Modal -->
{#if replyModal}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-md">
      <h3 class="text-lg font-semibold mb-2">Reply to {replyModal.comment.author || 'Unknown'}</h3>
      <p class="text-sm text-muted mb-4 truncate">{replyModal.comment.content}</p>
      <textarea
        bind:value={replyModal.text}
        placeholder="Write your reply..."
        rows="4"
        class="w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm mb-4"
      ></textarea>
      <div class="flex gap-3 justify-end">
        <button onclick={() => replyModal = null} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={sendReply} disabled={sending || !replyModal.text.trim()} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">
          {sending ? "Sending..." : "Reply"}
        </button>
      </div>
    </div>
  </div>
{/if}
