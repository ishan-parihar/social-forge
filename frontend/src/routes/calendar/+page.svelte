<script lang="ts">
  import { onMount } from 'svelte';
  import { calendar as calApi, type PostSummary } from '$lib/api';
  import { getMonthDays, isToday, isCurrentMonth, months, days, formatDateTime } from '$lib/calendar';
  import { goto } from '$app/navigation';

  let now = $state(new Date());
  let year = $state(now.getFullYear());
  let month = $state(now.getMonth());
  let calDays = $state<Date[]>([]);
  let postsByDate = $state<Map<string, PostSummary[]>>(new Map());
  let selectedDay = $state<string | null>(null);
  let dayPosts = $state<PostSummary[]>([]);

  function getPosts(date: Date): PostSummary[] {
    return postsByDate.get(date.toISOString().split('T')[0]) || [];
  }

  async function refresh() {
    calDays = getMonthDays(year, month);
    const start = year + '-' + String(month + 1).padStart(2, '0') + '-01';
    const end = new Date(year, month + 1, 0).toISOString().split('T')[0];
    const r = await calApi.get(start, end);
    if (r.data) {
      const m = new Map();
      r.data.days.forEach(d => m.set(d.date, d.posts));
      postsByDate = m;
    }
  }

  onMount(() => refresh());
  function prev() { if (month === 0) { year--; month = 11; } else month--; refresh(); }
  function next() { if (month === 11) { year++; month = 0; } else month++; refresh(); }
  function select(date: Date) {
    selectedDay = date.toISOString().split('T')[0];
    dayPosts = getPosts(date);
  }

  function badge(s: string) {
    return s === 'draft' ? 'badge-draft' : s === 'queued' ? 'badge-queued' : s === 'published' ? 'badge-published' : 'badge-error';
  }
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Content Calendar</h2>
    <button onclick={() => goto('/posts/new')} class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">+ New Post</button>
  </div>

  <div class="flex items-center justify-between bg-[#131720] border border-[#1e2435] rounded-xl px-4 py-3">
    <button onclick={prev} class="text-sm text-[#6b7280] hover:text-white">&larr;</button>
    <span class="font-medium">{months[month]} {year}</span>
    <button onclick={next} class="text-sm text-[#6b7280] hover:text-white">&rarr;</button>
  </div>

  <div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
    <div class="grid grid-cols-7 text-center text-xs text-[#6b7280] py-2.5 border-b border-[#1e2435]">
      {#each days as d}<span>{d}</span>{/each}
    </div>
    <div class="grid grid-cols-7">
      {#each calDays as date}
        {@const p = getPosts(date)}
        <button onclick={() => select(date)}
          class="min-h-24 p-2 text-left border-b border-r border-[#1e2435] transition-colors hover:bg-[#1a1f2e]"
          style="opacity: {isCurrentMonth(date, year, month) ? 1 : 0.3}; background: {selectedDay === date.toISOString().split('T')[0] ? 'rgba(79,70,229,0.15)' : ''}"
        >
          <span class="text-xs w-6 h-6 flex items-center justify-center rounded-full"
            style="background: {isToday(date) ? '#6366f1' : 'transparent'}; color: {isToday(date) ? 'white' : '#6b7280'}"
          >{date.getDate()}</span>
          {#each p.slice(0, 2) as post}
            <div class="text-[10px] truncate mt-1 px-1 py-0.5 rounded {badge(post.state)}">{post.content}</div>
          {/each}
          {#if p.length > 2}
            <div class="text-[10px] text-[#6b7280] mt-1 px-1">+{p.length - 2} more</div>
          {/if}
        </button>
      {/each}
    </div>
  </div>

  {#if selectedDay && dayPosts.length > 0}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
      <h3 class="text-sm font-medium mb-3">{selectedDay}</h3>
      {#each dayPosts as post}
        <div class="flex items-center gap-3 py-2 border-b border-[#1e2435] last:border-0">
          <span class="text-xs text-[#6b7280] w-24">{formatDateTime(post.scheduled_at || post.created_at)}</span>
          <span class="flex-1 text-sm truncate">{post.content}</span>
          <span class="text-xs px-2 py-0.5 rounded {badge(post.state)}">{post.state}</span>
        </div>
      {/each}
    </div>
  {/if}
</div>
