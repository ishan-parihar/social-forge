<script lang="ts">
  import { onMount } from 'svelte';
  import { posts, integrations, type Integration } from '$lib/api';
  import { goto } from '$app/navigation';
  import { toast } from '$lib/stores/toast';

  let channels = $state<Integration[]>([]);
  let content = $state('');
  let selChannel = $state('');
  let scheduledAt = $state('');
  let busy = $state(false);

  onMount(async () => {
    const r = await integrations.list();
    if (r.data) { channels = r.data.integrations; if (channels.length) selChannel = channels[0].id; }
  });

  async function submit() {
    if (!selChannel || !content) { toast('Select a channel and enter content', 'error'); return; }
    busy = true;
    const r = await posts.create({
      integration_id: selChannel,
      content,
      scheduled_at: scheduledAt || undefined,
    });
    busy = false;
    if (r.data) { toast('Post ' + (scheduledAt ? 'scheduled' : 'saved as draft'), 'success'); goto('/posts'); }
    else toast(r.error || 'Failed', 'error');
  }
</script>

<div class="max-w-2xl space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">New Post</h2>
    <a href="/posts" class="text-xs text-indigo-400 hover:text-indigo-300">&larr; Back</a>
  </div>

  {#if channels.length === 0}
    <div class="bg-yellow-900/20 border border-yellow-800 text-yellow-300 p-4 rounded-xl text-sm">
      No channels connected. <a href="/channels" class="underline">Connect a channel</a> first.
    </div>
  {/if}

  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
    <div>
      <label class="text-xs text-[#6b7280] block mb-1.5">Channel</label>
      <select bind:value={selChannel} class="w-full bg-[#0b0e14] border border-[#1e2435] rounded-lg px-3 py-2.5 text-sm focus:border-indigo-500 outline-none">
        {#each channels as ch}<option value={ch.id}>{ch.provider_name} — {ch.profile_name}</option>{/each}
      </select>
    </div>

    <div>
      <label class="text-xs text-[#6b7280] block mb-1.5">Content</label>
      <textarea bind:value={content} rows={5} placeholder="What's happening?" class="w-full bg-[#0b0e14] border border-[#1e2435] rounded-lg px-3 py-2.5 text-sm focus:border-indigo-500 outline-none resize-vertical"></textarea>
      <p class="text-xs text-[#6b7280] mt-1">{content.length} characters</p>
    </div>

    <div>
      <label class="text-xs text-[#6b7280] block mb-1.5">Schedule (optional — leave empty for draft)</label>
      <input type="datetime-local" bind:value={scheduledAt} class="w-full bg-[#0b0e14] border border-[#1e2435] rounded-lg px-3 py-2.5 text-sm focus:border-indigo-500 outline-none" />
    </div>

    <div class="flex gap-3 pt-2">
      <button onclick={submit} disabled={busy} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg text-sm font-medium transition-colors">
        {busy ? 'Saving...' : scheduledAt ? 'Schedule Post' : 'Save Draft'}
      </button>
      <a href="/posts" class="px-4 py-2 bg-[#1a1f2e] hover:bg-[#1e2435] border border-[#1e2435] rounded-lg text-sm transition-colors">Cancel</a>
    </div>
  </div>
</div>
