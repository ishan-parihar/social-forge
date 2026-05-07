<script lang="ts">
  import { onMount } from 'svelte';
  import { posts as postsApi, type PostSummary } from '$lib/api';
  import { formatDateTime } from '$lib/calendar';
  import { goto } from '$app/navigation';

  let all = $state<PostSummary[]>([]);
  let filter = $state('');

  onMount(async () => { const r = await postsApi.list({ limit: 100 }); if (r.data) all = r.data.posts; });
  let filtered = $derived(filter ? all.filter(p => p.state === filter) : all);

  function del(id: string) { if (!confirm('Delete?')) return; postsApi.delete(id); all = all.filter(p => p.id !== id); }
  function stateCls(s: string) { return s === 'draft' ? 'badge-draft' : s === 'queued' ? 'badge-queued' : s === 'published' ? 'badge-published' : 'badge-error'; }

  const tabs = ['', 'draft', 'queued', 'published', 'error'];
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Posts</h2>
    <button onclick={() => goto('/posts/new')} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Post</button>
  </div>

  <div class="flex gap-2">
    {#each tabs as st}
      <button onclick={() => filter = st} class="px-3 py-1.5 rounded-lg text-xs transition-colors {filter === st ? 'bg-indigo-600 text-white' : 'bg-[#1a1f2e] text-[#6b7280] hover:text-white'}">{st || 'All'}</button>
    {/each}
  </div>

  <div class="space-y-2">
    {#each filtered as post (post.id)}
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4 flex items-center gap-4">
        <div class="flex-1 min-w-0">
          <p class="text-sm truncate">{post.content}</p>
          <div class="flex gap-3 mt-1.5 text-xs text-[#6b7280]">
            <span>{post.integration_name}</span>
            {#if post.scheduled_at}<span>{formatDateTime(post.scheduled_at)}</span>{/if}
            {#if post.error_message}<span class="text-red-400">{post.error_message}</span>{/if}
          </div>
        </div>
        <span class="text-xs px-2 py-0.5 rounded {stateCls(post.state)}">{post.state}</span>
        <button onclick={() => goto(`/posts/${post.id}`)} class="text-xs text-indigo-400 hover:text-indigo-300">View</button>
        <button onclick={() => del(post.id)} class="text-xs text-red-400 hover:text-red-300">Del</button>
      </div>
    {/each}
    {#if filtered.length === 0}
      <div class="text-center py-12 text-sm text-[#6b7280]">No posts found. <button onclick={() => goto('/posts/new')} class="text-indigo-400 hover:underline">Create one</button></div>
    {/if}
  </div>
</div>
