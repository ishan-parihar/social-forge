<script lang="ts">
  import { feedApi, type FeedPost, type FeedAccount } from "$lib/api/feed";
  import { engagementIcon, formatMetricCount } from "$lib/calendar/engagement";

  let { post, onclose }: { post: FeedPost; onclose: () => void } = $props();

  interface Comment {
    id: string;
    author_name: string | null;
    author_avatar: string | null;
    text: string;
    created_at: string;
    like_count: number;
    replies: Comment[];
  }

  let comments = $state<Comment[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let expandedThreads = $state<Set<string>>(new Set());

  function toggleThread(id: string) {
    const next = new Set(expandedThreads);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    expandedThreads = next;
  }

  function formatTime(iso: string): string {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 1) return 'just now';
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    if (diffDay < 7) return `${diffDay}d ago`;
    return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  async function loadComments() {
    loading = true;
    error = null;
    try {
      const res = await fetch(`/api/feed/${post.id}/comments`);
      const text = await res.text();
      if (!res.ok) {
        error = `Failed to load comments (${res.status})`;
        return;
      }
      const data = JSON.parse(text);
      // Deduplicate by comment ID to prevent duplicates from appearing
      const deduped = new Map<string, any>();
      for (const c of data) {
        if (!deduped.has(c.id)) {
          deduped.set(c.id, c);
        }
      }
      comments = Array.from(deduped.values()).map((c: any) => ({
        id: c.id,
        author_name: c.author_name,
        author_avatar: c.author_avatar,
        text: c.text,
        created_at: c.created_at,
        like_count: c.like_count || 0,
        replies: (c.replies || []).reduce((acc: any[], r: any) => {
          if (!acc.some(existing => existing.id === r.id)) {
            acc.push({
              id: r.id,
              author_name: r.author_name,
              author_avatar: r.author_avatar,
              text: r.text,
              created_at: r.created_at,
              like_count: r.like_count || 0,
              replies: [],
            });
          }
          return acc;
        }, []),
      }));
    } catch (e: any) {
      error = e.message || "Failed to load comments";
    } finally {
      loading = false;
    }
  }

  // Load on mount
  $effect(() => {
    loadComments();
  });
</script>

<div class="mt-4 pt-4 border-t border-[#1a2035]">
  <!-- Header -->
  <div class="flex items-center justify-between mb-3">
    <h4 class="text-xs font-semibold text-[#9ca3af] uppercase tracking-wider flex items-center gap-2">
      <svg class="w-3.5 h-3.5 text-blue-400" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M14 8a6 6 0 01-9.3 5L2 14l1-2.7A6 6 0 1114 8z" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Comments
      {#if comments.length > 0}
        <span class="text-[10px] font-mono text-[#5a6070] bg-[#0d121e] px-1.5 py-0.5 rounded-full">
          {comments.length}
        </span>
      {/if}
    </h4>
    <button
      onclick={onclose}
      class="text-[#5a6070] hover:text-[#9ca3af] text-xs transition-colors"
    >Hide</button>
  </div>

  <!-- Loading -->
  {#if loading}
    <div class="flex items-center justify-center py-6">
      <div class="w-4 h-4 rounded-full border-2 border-indigo-400/30 border-t-indigo-400 animate-spin" />
    </div>

  <!-- Error -->
  {:else if error}
    <div class="text-xs text-red-400 text-center py-4">{error}</div>

  <!-- Empty -->
  {:else if comments.length === 0}
    <div class="text-xs text-[#5a6070] text-center py-4">No comments yet</div>

  <!-- Comments list -->
  {:else}
    <div class="space-y-3">
      {#each comments as comment (comment.id)}
        <div class="bg-[#0d121e] border border-[#1a2035] rounded-lg overflow-hidden">
          <!-- Comment header -->
          <div class="flex items-start gap-2.5 p-3">
            <!-- Avatar -->
            {#if comment.author_avatar}
              <img src={comment.author_avatar} alt="" class="w-6 h-6 rounded-full flex-shrink-0 object-cover ring-1 ring-[#1e2435]" />
            {:else}
              <span class="w-6 h-6 rounded-full flex-shrink-0 bg-[#1e2435] flex items-center justify-center">
                <svg class="w-3 h-3 text-[#5a6070]" viewBox="0 0 16 16" fill="currentColor">
                  <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6a5 5 0 0110 0H3z"/>
                </svg>
              </span>
            {/if}
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span class="text-xs font-medium text-[#cdd2dc]">{comment.author_name || 'Anonymous'}</span>
                <span class="text-[10px] text-[#5a6070]">{formatTime(comment.created_at)}</span>
              </div>
              <p class="text-xs text-[#9ca3af] mt-1 leading-relaxed whitespace-pre-wrap break-words">{comment.text}</p>
              <div class="flex items-center gap-3 mt-1.5">
                {#if comment.like_count > 0}
                  <span class="text-[10px] text-pink-400/60 flex items-center gap-1">
                    {engagementIcon('likes', post.provider)} {formatMetricCount(comment.like_count)}
                  </span>
                {/if}
                {#if comment.replies.length > 0}
                  <button
                    onclick={() => toggleThread(comment.id)}
                    class="text-[10px] text-indigo-400 hover:text-indigo-300 transition-colors"
                  >
                    {expandedThreads.has(comment.id) ? 'Hide replies' : `${comment.replies.length} ${comment.replies.length === 1 ? 'reply' : 'replies'}`}
                  </button>
                {/if}
              </div>
            </div>
          </div>

          <!-- Nested replies -->
          {#if expandedThreads.has(comment.id) && comment.replies.length > 0}
            <div class="border-t border-[#1a2035] bg-[#0a0e16]">
              {#each comment.replies as reply (reply.id)}
                <div class="flex items-start gap-2.5 p-3 pl-8 border-b border-[#1a2035] last:border-b-0">
                  <!-- Reply avatar -->
                  {#if reply.author_avatar}
                    <img src={reply.author_avatar} alt="" class="w-5 h-5 rounded-full flex-shrink-0 object-cover ring-1 ring-[#1e2435]" />
                  {:else}
                    <span class="w-5 h-5 rounded-full flex-shrink-0 bg-[#1e2435] flex items-center justify-center">
                      <svg class="w-2.5 h-2.5 text-[#5a6070]" viewBox="0 0 16 16" fill="currentColor">
                        <path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6a5 5 0 0110 0H3z"/>
                      </svg>
                    </span>
                  {/if}
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2">
                      <span class="text-xs font-medium text-[#cdd2dc]">{reply.author_name || 'Anonymous'}</span>
                      <span class="text-[10px] text-[#5a6070]">{formatTime(reply.created_at)}</span>
                    </div>
                    <p class="text-xs text-[#9ca3af] mt-0.5 leading-relaxed whitespace-pre-wrap break-words">{reply.text}</p>
                    {#if reply.like_count > 0}
                      <span class="text-[10px] text-pink-400/60 flex items-center gap-1 mt-1">
                        {engagementIcon('likes', post.provider)} {formatMetricCount(reply.like_count)}
                      </span>
                    {/if}
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  /* Thread line for replies */
  .thread-line {
    position: relative;
  }
  .thread-line::before {
    content: '';
    position: absolute;
    left: 15px;
    top: 0;
    bottom: 0;
    width: 1px;
    background: #1a2035;
  }
</style>
