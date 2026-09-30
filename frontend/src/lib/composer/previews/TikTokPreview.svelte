<script lang="ts">
  // v25 F2: TikTok preview.
  //
  // Light chrome = destination-platform simulation (see previews/index.ts).
  //
  // TikTok is a 9:16 full-bleed frame with the caption burned into the bottom
  // left and the music chip bottom right — the most distinctive of the 12
  // layouts, so it gets a real 9:16 frame rather than a card. The music chip
  // is wired to the composer's MusicPicker: if a track is attached, the chip
  // is here, because TikTok rejects an upload whose sound does not match the
  // attached track id.
  import { charLimitFor, countFor, plainText } from '../platforms';
  import type { MediaItem } from '$lib/api/media';

  let {
    content = '',
    authorName = 'Your Brand',
    authorHandle = 'yourbrand',
    media = [] as MediaItem[],
    audioTitle = '',
  }: {
    content?: string;
    authorName?: string;
    authorHandle?: string;
    media?: MediaItem[];
    audioTitle?: string;
  } = $props();

  let text = $derived(plainText(content));
  let limit = $derived(charLimitFor('tiktok'));
  let count = $derived(countFor('tiktok', text));
  let isOver = $derived(count > limit);
  let video = $derived(media.find(m => !m.mime_type?.startsWith('image/')));
</script>

<!-- The frame is 9:16, capped at 260px tall so the pane stays scrollable when
     several platforms are selected at once. -->
<div class="mx-auto w-full max-w-[220px] bg-black rounded-2xl overflow-hidden border border-gray-800" style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
  <div class="relative aspect-[9/16] bg-gray-900 flex items-center justify-center">
    {#if video}
      <div class="text-center text-gray-500 text-xs">
        <svg class="w-10 h-10 mx-auto mb-1" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        Video ready
      </div>
    {:else}
      <div class="text-center text-red-400 text-xs px-4">
        <svg class="w-10 h-10 mx-auto mb-1 text-red-500" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        A TikTok post cannot publish without a video
      </div>
    {/if}

    <!-- Right rail: the vertical action stack -->
    <div class="absolute right-1.5 bottom-16 flex flex-col items-center gap-3 text-white text-[10px]">
      <span class="flex flex-col items-center">
        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor"><path d="M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54z"/></svg>
        12.4K
      </span>
      <span class="flex flex-col items-center">
        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor"><path d="M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z"/></svg>
        318
      </span>
      <span class="flex flex-col items-center">
        <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor"><path d="M17 1l4 4-4 4 1 1-4.5 1.5L12 7 8.5 11.5 4 10l1-1-4-4 4-4 1 1L11 4.5 15 1z"/></svg>
        2.1K
      </span>
    </div>

    <!-- Caption burned into the bottom left, as TikTok renders it -->
    <div class="absolute left-2 right-2 bottom-2 text-white">
      <div class="text-xs font-semibold">@{authorHandle}</div>
      <p class="text-[11px] leading-snug mt-0.5 line-clamp-3">
        {text.slice(0, limit)}<span class="bg-red-500/40 rounded-sm">{text.slice(limit)}</span>
      </p>
      {#if isOver}
        <p class="text-[9px] text-red-300 mt-0.5">{count}/{limit} — will be rejected, trim {count - limit} more</p>
      {/if}
      <!-- Music chip: rotating-note + marquee, exactly where TikTok puts it. -->
      <div class="flex items-center gap-1 mt-1.5 text-[10px] text-white/90 overflow-hidden">
        <svg class="w-3 h-3 flex-shrink-0" viewBox="0 0 24 24" fill="currentColor"><path d="M12 3v10.55A4 4 0 1014 17V7h4V3h-6z"/></svg>
        <span class="truncate">{audioTitle || 'original sound'}</span>
      </div>
    </div>
  </div>
</div>
