<script lang="ts">
  // v25 F2: Bluesky post preview.
  // Light chrome = destination-platform simulation (see previews/index.ts).
  // Bluesky's 300-char grapheme limit is the second-tightest after X, so the
  // overflow mark earns its place here too.
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
  let limit = $derived(charLimitFor('bluesky'));
  let count = $derived(countFor('bluesky', text));
  let isOver = $derived(count > limit);
  // Bluesky takes up to 4 images in a 2x2 grid.
  let images = $derived(media.filter(m => m.mime_type?.startsWith('image/')).slice(0, 4));
  let handle = $derived(authorName.toLowerCase().replace(/\s+/g, '.'));
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden" style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
  <div class="flex gap-3 p-4">
    <div class="w-10 h-10 rounded-full bg-blue-500 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {authorName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-1 text-sm flex-wrap">
        <span class="font-semibold">{authorName}</span>
        <span class="text-gray-400">@{handle}.bsky.social</span>
        <span class="text-gray-400">· 2h</span>
      </div>
      <p class="mt-1 text-sm whitespace-pre-wrap break-words">
        {text.slice(0, limit)}<span class="bg-red-100 text-red-600 rounded-sm">{text.slice(limit)}</span>
      </p>
      {#if isOver}
        <p class="text-[10px] text-red-600 mt-1">{count}/{limit} — will be rejected, trim {count - limit} more</p>
      {/if}
      {#if images.length > 0}
        <div class="mt-2 grid grid-cols-2 gap-0.5 rounded-lg overflow-hidden">
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
