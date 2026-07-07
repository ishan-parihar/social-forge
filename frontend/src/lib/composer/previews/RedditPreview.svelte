<script lang="ts">
  // v24-8: Reddit post preview.
  // Matches Reddit's post card: subreddit, title, content, vote buttons.
  let { content, title, integrationName = 'Reddit' }: {
    content: string;
    title?: string;
    integrationName?: string;
  } = $props();

  let plainText = $derived(content.replace(/<[^>]*>/g, ''));
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden">
  <div class="flex">
    <!-- Vote column -->
    <div class="bg-gray-100 flex flex-col items-center py-2 px-2 gap-1">
      <svg class="w-5 h-5 text-gray-400 hover:text-orange-500" viewBox="0 0 24 24" fill="currentColor"><path d="M12 4l8 8h-5v8h-6v-8H4z"/></svg>
      <span class="text-xs font-bold text-orange-600">1.2k</span>
      <svg class="w-5 h-5 text-gray-400 hover:text-blue-500" viewBox="0 0 24 24" fill="currentColor"><path d="M12 20l-8-8h5V4h6v8h5z"/></svg>
    </div>
    <!-- Content -->
    <div class="flex-1 p-3">
      <div class="flex items-center gap-1 text-xs text-gray-500 mb-1">
        <span class="w-5 h-5 rounded-full bg-orange-500 flex items-center justify-center text-white text-[8px] font-bold">r/</span>
        <span class="font-medium text-gray-700">r/socialforge</span>
        <span>· 2h ·</span>
        <span>Posted by u/{integrationName}</span>
      </div>
      {#if title}
        <h3 class="font-semibold text-base mb-1">{title}</h3>
      {/if}
      <p class="text-sm text-gray-700 whitespace-pre-wrap break-words">{plainText}</p>
      <div class="flex items-center gap-4 mt-3 text-xs text-gray-500">
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg> 42 Comments</span>
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 12v8a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg> Share</span>
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"/></svg> Save</span>
      </div>
    </div>
  </div>
</div>
