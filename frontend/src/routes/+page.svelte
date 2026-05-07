<script lang="ts">
  import { onMount } from 'svelte';
  import { posts, type PostSummary } from '$lib/api';
  import { formatDateTime } from '$lib/calendar';
  import { goto } from '$app/navigation';

  let upcoming = $state<PostSummary[]>([]);
  let todayPosts = $state<PostSummary[]>([]);
  let stats = $state({ draft: 0, queued: 0, published: 0, error: 0 });

  onMount(async () => {
    const r = await posts.list({ limit: 100 });
    if (!r.data) return;
    const all = r.data.posts;
    upcoming = all.filter(p => p.state === 'queued').slice(0, 5);
    const t = new Date().toDateString();
    todayPosts = all.filter(p => p.scheduled_at && new Date(p.scheduled_at).toDateString() === t).slice(0, 5);
    stats = { draft: all.filter(p => p.state === 'draft').length, queued: all.filter(p => p.state === 'queued').length, published: all.filter(p => p.state === 'published').length, error: all.filter(p => p.state === 'error').length };
  });
</script>

<div class="space-y-6">
  <div>
    <h2 class="text-xl font-semibold">Dashboard</h2>
    <p class="text-sm text-[#6b7280] mt-1">Overview of your social media schedule</p>
  </div>

  <div class="grid grid-cols-4 gap-4">
    {#each [{ label: 'Drafts', value: stats.draft, color: 'text-blue-400' }, { label: 'Queued', value: stats.queued, color: 'text-yellow-400' }, { label: 'Published', value: stats.published, color: 'text-green-400' }, { label: 'Errors', value: stats.error, color: 'text-red-400' }] as s}
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
        <div class="text-2xl font-bold {s.color}">{s.value}</div>
        <div class="text-xs text-[#6b7280] mt-1">{s.label}</div>
      </div>
    {/each}
  </div>

  <div class="grid grid-cols-2 gap-4">
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
      <h3 class="font-medium text-sm mb-3">Today's Schedule</h3>
      {#if todayPosts.length === 0}
        <p class="text-sm text-[#6b7280]">No posts scheduled for today.</p>
      {:else}
        {#each todayPosts as post}
          <div class="flex items-center gap-3 py-2 border-b border-[#1e2435] last:border-0">
            <span class="text-xs text-[#6b7280] w-16">{post.scheduled_at ? formatDateTime(post.scheduled_at).slice(-5) : ''}</span>
            <span class="flex-1 text-sm truncate">{post.content}</span>
            <span class="text-xs px-2 py-0.5 rounded {post.state === 'queued' ? 'badge-queued' : 'badge-published'}">{post.integration_name}</span>
          </div>
        {/each}
      {/if}
    </div>

    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5">
      <h3 class="font-medium text-sm mb-3">Upcoming Posts</h3>
      {#if upcoming.length === 0}
        <p class="text-sm text-[#6b7280]">No upcoming posts.</p>
      {:else}
        {#each upcoming as post}
          <div class="flex items-center gap-3 py-2 border-b border-[#1e2435] last:border-0">
            <span class="text-xs text-[#6b7280] w-20">{post.scheduled_at ? formatDateTime(post.scheduled_at) : ''}</span>
            <span class="flex-1 text-sm truncate">{post.content}</span>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  <div class="flex gap-3">
    <button onclick={() => goto('/posts/new')} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm font-medium transition-colors">+ New Post</button>
    <button onclick={() => goto('/channels')} class="px-4 py-2 bg-[#1a1f2e] hover:bg-[#1e2435] border border-[#1e2435] rounded-lg text-sm transition-colors">Manage Channels</button>
  </div>
</div>
