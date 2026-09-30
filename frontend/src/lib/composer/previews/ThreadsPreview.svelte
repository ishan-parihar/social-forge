<script lang="ts">
  // v25 F2: Threads post preview.
  // Light chrome = destination-platform simulation (see previews/index.ts).
  // F2 added the 500-char overflow mark — Threads rejects rather than crops,
  // and it is the tightest limit among the Meta properties after X.
  import { charLimitFor, countFor, plainText } from '../platforms';
  import type { MediaItem } from '$lib/api/media';

  let {
    content = '',
    authorName = 'Your Brand',
    media = [] as MediaItem[],
  }: {
    content?: string;
    authorName?: string;
    media?: MediaItem[];
  } = $props();

  let text = $derived(plainText(content));
  let limit = $derived(charLimitFor('threads'));
  let count = $derived(countFor('threads', text));
  let isOver = $derived(count > limit);
  let images = $derived(media.filter(m => m.mime_type?.startsWith('image/')).slice(0, 1));
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden" style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
  <div class="flex gap-3 p-4">
    <div class="w-9 h-9 rounded-full bg-gradient-to-br from-purple-500 to-pink-500 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {authorName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-2 text-sm mb-1">
        <span class="font-semibold">{authorName}</span>
        <span class="text-gray-400 text-xs">2h</span>
      </div>
      <p class="text-sm whitespace-pre-wrap break-words">
        {text.slice(0, limit)}<span class="bg-red-100 text-red-600 rounded-sm">{text.slice(limit)}</span>
      </p>
      {#if isOver}
        <p class="text-[10px] text-red-600 mt-1">{count}/{limit} — will be rejected, trim {count - limit} more</p>
      {/if}
      {#if images.length > 0}
        <div class="mt-2 rounded-lg overflow-hidden border border-gray-200">
          <img src={images[0].url} alt="" class="w-full max-h-96 object-cover" />
        </div>
      {/if}
      <div class="flex items-center gap-5 mt-3 text-gray-500 text-xs">
        <span>♥ 128</span>
        <span>Comments 12</span>
        <span>Reposts 5</span>
      </div>
    </div>
  </div>
</div>
