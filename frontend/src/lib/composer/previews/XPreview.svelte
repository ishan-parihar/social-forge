<script lang="ts">
  // v24-8: X (Twitter) post preview.
  // Matches X's post card layout: avatar + name + handle, content, metrics.
  let { content, media = [], integrationName = 'X Account' }: {
    content: string;
    media?: Array<{ url: string; mime_type: string }>;
    integrationName?: string;
  } = $props();

  let plainText = $derived(content.replace(/<[^>]*>/g, ''));
  let images = $derived(media.filter(m => m.mime_type?.startsWith('image/')).slice(0, 4));
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden">
  <div class="flex gap-3 p-4">
    <div class="w-10 h-10 rounded-full bg-gray-800 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {integrationName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-1 text-sm">
        <span class="font-bold">{integrationName}</span>
        <svg class="w-4 h-4 text-blue-500" viewBox="0 0 24 24" fill="currentColor"><path d="M22.5 12.5c0-1.58-.875-2.95-2.148-3.6.154-.435.238-.905.238-1.4 0-2.21-1.71-3.998-3.818-3.998-.47 0-.92.084-1.336.25C14.818 2.415 13.51 1.5 12 1.5s-2.816.917-3.437 2.25c-.415-.165-.866-.25-1.336-.25-2.11 0-3.818 1.79-3.818 4 0 .494.083.964.237 1.4-1.272.65-2.147 2.02-2.147 3.6 0 1.495.788 2.8 1.95 3.485-.07.336-.108.686-.108 1.045 0 2.21 1.71 3.998 3.818 3.998.47 0 .92-.085 1.336-.25.62 1.334 1.927 2.25 3.437 2.25s2.816-.917 3.437-2.25c.415.165.866.25 1.336.25 2.11 0 3.818-1.79 3.818-4 0-.358-.038-.708-.108-1.045 1.162-.685 1.95-1.99 1.95-3.485z"/></svg>
        <span class="text-gray-500">@{integrationName.toLowerCase().replace(/\s+/g, '_')}</span>
        <span class="text-gray-400">· 2h</span>
      </div>
      <p class="mt-1 text-sm whitespace-pre-wrap break-words">{plainText}</p>
      {#if images.length > 0}
        <div class="mt-2 grid {images.length === 1 ? 'grid-cols-1' : 'grid-cols-2'} gap-0.5 rounded-xl overflow-hidden border border-gray-200">
          {#each images as img (img.url)}
            <img src={img.url} alt="" class="w-full h-48 object-cover" />
          {/each}
        </div>
      {/if}
      <div class="flex items-center gap-6 mt-3 text-gray-500 text-xs">
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 9V5a3 3 0 0 0-6 0v4H5a2 2 0 0 0-2 2v7a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2v-7a2 2 0 0 0-2-2h-2z"/></svg> 24</span>
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/></svg> 5</span>
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 1l4 4-4 4M3 11V9a4 4 0 0 1 4-4h14M7 23l-4-4 4-4M21 13v2a4 4 0 0 1-4 4H3"/></svg> 12</span>
        <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20.84 4.61a5.5 5.5 0 0 0-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 0 0-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 0 0 0-7.78z"/></svg> 48</span>
      </div>
    </div>
  </div>
</div>
