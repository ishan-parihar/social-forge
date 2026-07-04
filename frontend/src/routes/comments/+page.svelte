<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { commentsApi, type Comment } from "$lib/api/comments";
  import { toast } from "$lib/stores/toast";
  import { realtime } from "$lib/stores/realtime";

  let comments = $state<Comment[]>([]);
  let filterPlatform = $state("all");
  let filterStatus = $state("all");
  let loading = $state(true);
  let error = $state<string | null>(null);
  let replyModal = $state<{ comment: Comment; text: string } | null>(null);
  let sending = $state(false);

  const platforms = ["all", "x", "reddit", "linkedin", "facebook", "instagram"];
  const statuses = ["all", "new", "resolved"];

  async function load() {
    loading = true;
    error = null;
    const r = await commentsApi.list({
      ...(filterStatus !== "all" && { resolved: filterStatus === "resolved" }),
    });
    if (r.data) {
      let filtered = r.data.comments;
      if (filterPlatform !== "all") {
        filtered = filtered.filter(c => c.platform === filterPlatform);
      }
      comments = filtered;
    } else {
      error = r.error || "Failed to load comments";
    }
    loading = false;
  }

  async function resolveComment(id: string) {
    const r = await commentsApi.resolve(id);
    if (r.error) {
      toast(`Failed to resolve: ${r.error}`, "error");
    } else {
      comments = comments.map(c => c.id === id ? { ...c, is_resolved: true } : c);
    }
  }

  async function sendReply() {
    if (!replyModal || !replyModal.text.trim()) return;
    sending = true;
    const r = await commentsApi.reply(replyModal.comment.id, replyModal.text);
    if (r.error) {
      toast(`Reply failed: ${r.error}`, "error");
    } else {
      toast("Reply sent", "success");
      replyModal = null;
    }
    sending = false;
  }

  function platformIcon(p: string): string {
    const icons: Record<string, string> = { x: "𝕏", reddit: "𝗥", linkedin: "in", facebook: "f", instagram: "📷" };
    return icons[p] || "•";
  }

  let commentsUnsubscribers: (() => void)[] = [];

  onMount(() => {
    load();
    // Refresh when new comments arrive or posts change
    for (const evt of ['post_published', 'post_created']) {
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
    <button onclick={load} class="px-3 py-1.5 text-sm text-muted hover:text-white border border-line rounded-lg transition-colors">↻ Refresh</button>
  </div>

  <!-- Filters -->
  <div class="flex gap-4">
    <div class="flex gap-1 bg-surface border border-line rounded-lg p-1">
      {#each platforms as p}
        <button
          onclick={() => { filterPlatform = p; load(); }}
          class="px-3 py-1.5 text-xs capitalize rounded-md transition-colors {filterPlatform === p ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover'}"
        >{p}</button>
      {/each}
    </div>
    <div class="flex gap-1 bg-surface border border-line rounded-lg p-1">
      {#each statuses as s}
        <button
          onclick={() => { filterStatus = s; load(); }}
          class="px-3 py-1.5 text-xs capitalize rounded-md transition-colors {filterStatus === s ? 'bg-indigo-600 text-white' : 'text-muted hover:bg-surface-hover'}"
        >{s}</button>
      {/each}
    </div>
  </div>

  <!-- Content -->
  {#if error}
    <div class="text-center py-12 text-sm text-red-400">{error}</div>
  {:else if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if comments.length === 0}
    <div class="text-center py-12 text-sm text-muted">No comments found</div>
  {:else}
    <div class="bg-surface border border-line rounded-xl overflow-hidden">
      <div class="grid grid-cols-[40px_1fr_1.5fr_100px_100px_90px] gap-3 px-4 py-2 border-b border-line bg-background-input text-xs text-muted">
        <span></span><span>Post</span><span>Comment</span><span>Author</span><span>Date</span><span>Status</span>
      </div>
      {#each comments as c (c.id)}
        <div class="grid grid-cols-[40px_1fr_1.5fr_100px_100px_90px] gap-3 px-4 py-3 border-b border-line last:border-0 hover:bg-surface-hover transition-colors items-center">
          <span class="text-sm text-indigo-400">{platformIcon(c.platform)}</span>
          <span class="text-sm truncate">{c.post_id}</span>
          <span class="text-sm text-content-secondary truncate">{c.text}</span>
          <span class="text-xs text-muted truncate">{c.author_name || 'Unknown'}</span>
          <span class="text-xs text-muted">{new Date(c.created_at).toLocaleDateString()}</span>
          <div class="flex items-center gap-2">
            {#if !c.is_resolved}
              <span class="px-2 py-0.5 text-xs rounded bg-yellow-500/20 text-yellow-400">New</span>
              <button onclick={() => resolveComment(c.id)} class="text-xs text-muted hover:text-green-400">✓</button>
              <button onclick={() => replyModal = { comment: c, text: "" }} class="text-xs text-muted hover:text-indigo-400">↩</button>
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
      <h3 class="text-lg font-semibold mb-2">Reply to {replyModal.comment.author_name || 'Unknown'}</h3>
      <p class="text-sm text-muted mb-4 truncate">{replyModal.comment.text}</p>
      <textarea
        bind:value={replyModal.text}
        placeholder="Write your reply..."
        rows="4"
        class="w-full px-3 py-2 bg-[#161b22] border border-[#30363d] rounded text-sm mb-4"
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
