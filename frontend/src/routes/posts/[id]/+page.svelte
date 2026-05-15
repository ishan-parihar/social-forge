<script lang="ts">
  import { onMount } from 'svelte';
  import { postsApi, type PostDetail } from '$lib/api';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { formatDateTime } from '$lib/calendar';
  import { toast } from '$lib/stores/toast';

  let post = $state<PostDetail | null>(null);
  let loading = $state(true);

  onMount(async () => {
    const id = $page.params.id;
    if (!id) { loading = false; return; }
    const r = await postsApi.get(id);
    if (r.data) post = r.data;
    loading = false;
  });

  async function schedule() {
    if (!post) return;
    const dt = prompt('ISO8601 date:', new Date(Date.now() + 3600000).toISOString().slice(0, 16));
    if (!dt) return;
    const r = await postsApi.schedule(post.id, new Date(dt).toISOString());
    if (r.data) { post = r.data; toast('Post scheduled', 'success'); }
  }

  async function remove() {
    if (!post || !confirm('Delete?')) return;
    await postsApi.delete(post.id);
    toast('Post deleted', 'success');
    goto('/posts');
  }

  function badge(s: string) {
    return s === 'draft' ? 'badge-draft' : s === 'queued' ? 'badge-queued' : s === 'published' ? 'badge-published' : 'badge-error';
  }
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Post Details</h2>
    <a href="/posts" class="text-xs text-indigo-400 hover:text-indigo-300">&larr; Back to Posts</a>
  </div>

  {#if loading}
    <div class="text-center py-12 text-sm text-[#6b7280]">Loading...</div>
  {:else if !post}
    <div class="bg-red-900/20 border border-red-800 text-red-300 rounded-xl p-4 text-sm">Post not found.</div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-3">
        <div class="flex items-center justify-between">
          <span class="text-xl">{post.content.length > 50 ? post.content.slice(0, 50) + '...' : 'Post'}</span>
          <span class="text-xs px-2 py-0.5 rounded {badge(post.state)}">{post.state}</span>
        </div>
        <div class="text-sm whitespace-pre-wrap bg-[#0b0e14] rounded-lg p-3 border border-[#1e2435]">{post.content}</div>
        <div class="space-y-1.5 text-xs text-[#6b7280]">
          {#if post.scheduled_at}<div><span class="text-indigo-400">Scheduled:</span> {formatDateTime(post.scheduled_at)}</div>{/if}
          {#if post.published_at}<div><span class="text-green-400">Published:</span> {formatDateTime(post.published_at)}</div>{/if}
          {#if post.platform_post_url}<div><span class="text-indigo-400">URL:</span> <a href={post.platform_post_url} target="_blank" class="underline">{post.platform_post_url}</a></div>{/if}
          {#if post.error_message}<div><span class="text-red-400">Error:</span> {post.error_message}</div>{/if}
        </div>
        <div class="flex gap-2 pt-2">
          {#if post.state === 'draft'}<button onclick={schedule} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-xs transition-colors">Schedule</button>{/if}
          <button onclick={remove} class="px-3 py-1.5 bg-red-700 hover:bg-red-600 rounded-lg text-xs transition-colors">Delete</button>
        </div>
      </div>
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
        <h3 class="text-sm font-medium mb-3">Preview</h3>
        <div class="bg-white text-black rounded-xl p-4 max-w-sm">
          <div class="flex items-center gap-2 mb-3">
            <div class="w-8 h-8 rounded-full bg-indigo-100 flex items-center justify-center text-xs font-bold text-indigo-600">U</div>
            <div class="text-xs"><div class="font-semibold">Preview</div><div class="text-gray-500">@username</div></div>
          </div>
          <p class="text-sm leading-relaxed">{post.content}</p>
        </div>
      </div>
    </div>
  {/if}
</div>
