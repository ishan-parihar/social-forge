<script lang="ts">
  // v24-8: Bluesky post preview.
  // Matches Bluesky's clean card: avatar + handle, content, metrics.
  let { content, media = [], integrationName = 'Bluesky' }: {
    content: string;
    media?: Array<{ url: string; mime_type: string }>;
    integrationName?: string;
  } = $props();

  let plainText = $derived(content.replace(/<[^>]*>/g, ''));
  let images = $derived(media.filter(m => m.mime_type?.startsWith('image/')).slice(0, 4));
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden">
  <div class="flex gap-3 p-4">
    <div class="w-10 h-10 rounded-full bg-blue-500 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {integrationName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-1 text-sm">
        <span class="font-semibold">{integrationName}</span>
        <span class="text-gray-400">@{integrationName.toLowerCase().replace(/\s+/g, '.')}.bsky.social</span>
        <span class="text-gray-400">· 2h</span>
      </div>
      <p class="mt-1 text-sm whitespace-pre-wrap break-words">{plainText}</p>
      {#if images.length > 0}
        <div class="mt-2 grid {images.length === 1 ? 'grid-cols-1' : 'grid-cols-2'} gap-0.5 rounded-lg overflow-hidden">
          {#each images as img (img.url)}
            <img src={img.url} alt="" class="w-full h-40 object-cover" />
          {/each}
        </div>
      {/if}
      <div class="flex items-center gap-6 mt-3 text-gray-500 text-xs">
        <span class="flex items-center gap-1">
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 0 0-6 0v4H5a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-7a2 2 0 0 0-2-2h-2z"/></svg>
          32
        </span>
        <span class="flex items-center gap-1">
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg>
          8
        </span>
        <span class="flex items-center gap-1">
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 1l4 4-4 4M3 11V9a4 4 0 0 1 4-4h14M7 23l-4-4 4-4M21 13v2a4 4 0 0 1-4 4H3"/></svg>
          15
        </span>
      </div>
    </div>
  </div>
</div>
