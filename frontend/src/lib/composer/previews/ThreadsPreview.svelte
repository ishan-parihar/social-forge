<script lang="ts">
  // v24-8: Threads post preview.
  // Matches Threads' minimalist card: avatar + name, content, metrics.
  let { content, media = [], integrationName = 'Threads' }: {
    content: string;
    media?: Array<{ url: string; mime_type: string }>;
    integrationName?: string;
  } = $props();

  let plainText = $derived(content.replace(/<[^>]*>/g, ''));
  let images = $derived(media.filter(m => m.mime_type?.startsWith('image/')).slice(0, 1));
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden">
  <div class="flex gap-3 p-4">
    <div class="w-9 h-9 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {integrationName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2 text-sm mb-1">
        <span class="font-semibold">{integrationName}</span>
        <span class="text-gray-400 text-xs">2h</span>
      </div>
      <p class="text-sm whitespace-pre-wrap break-words">{plainText}</p>
      {#if images.length > 0}
        <div class="mt-2 rounded-lg overflow-hidden border border-gray-200">
          <img src={images[0].url} alt="" class="w-full max-h-96 object-cover" />
        </div>
      {/if}
      <div class="flex items-center gap-5 mt-3 text-gray-500 text-xs">
        <span>♥ 128</span>
        <span>💬 12</span>
        <span>↗ 5</span>
      </div>
    </div>
  </div>
</div>
