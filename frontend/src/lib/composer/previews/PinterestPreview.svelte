<script lang="ts">
  // v25 F2: Pinterest preview.
  //
  // Light chrome = destination-platform simulation (see previews/index.ts).
  //
  // A pin is a 2:3 portrait tile with a required title and a required board.
  // Both are real API requirements, so both are surfaced as live placeholders
  // here rather than as an error toast after the user thinks they are done —
  // the board chip is the one place the TargetPicker's board selection is
  // actually visible in a preview.
  import { charLimitFor, countFor, plainText } from '../platforms';
  import type { MediaItem } from '$lib/api/media';

  let {
    content = '',
    title = '',
    authorName = 'Your Brand',
    media = [] as MediaItem[],
    targetLabel = '',
  }: {
    content?: string;
    title?: string;
    authorName?: string;
    media?: MediaItem[];
    targetLabel?: string;
  } = $props();

  let text = $derived(plainText(content));
  let limit = $derived(charLimitFor('pinterest'));
  let count = $derived(countFor('pinterest', text));
  let isOver = $derived(count > limit);
  let image = $derived(media.find(m => m.mime_type?.startsWith('image/')));
  // Pinterest's own description fold is ~100 chars behind "more".
  let folded = $derived(text.length > 100 ? text.slice(0, 100) : text);
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden max-w-[280px] mx-auto" style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
  <!-- 2:3 portrait tile -->
  <div class="relative aspect-[2/3] bg-gray-100 flex items-center justify-center overflow-hidden">
    {#if image}
      <img src={image.url} alt="" class="w-full h-full object-cover" />
    {:else}
      <div class="text-center text-red-500 text-xs px-4">
        <svg class="w-8 h-8 mx-auto mb-1 text-red-400" viewBox="0 0 24 24" fill="currentColor"><path d="M12 2C6.48 2 2 6.48 2 12c0 4.24 2.64 7.86 6.36 9.31-.09-.79-.17-2 .03-2.86.18-.78 1.19-4.96 1.19-4.96s-.3-.6-.3-1.49c0-1.4.81-2.44 1.82-2.44.86 0 1.27.64 1.27 1.42 0 .86-.55 2.15-.83 3.35-.24 1 .5 1.81 1.48 1.81 1.78 0 3.05-2.26 3.05-4.92 0-2.04-1.42-3.58-3.99-3.58-2.9 0-4.75 2.16-4.75 4.55 0 .83.25 1.79.58 2.35.06.1.07.19.05.29-.06.25-.2 1.13-.23 1.29-.04.22-.17.26-.4.16-1.49-.6-2.19-2.2-2.19-4.02 0-2.98 2.51-6.5 7.48-6.5 4.04 0 6.68 2.95 6.68 6.1 0 4.19-2.32 6.85-5.74 6.85-1.15 0-2.23-.62-2.6-1.35l-.7 2.86c-.26 1.02-.94 2.2-1.42 2.98C17.34 21.29 20 17 20 12c0-5.52-3.58-10-8-10z"/></svg>
        A pin needs an image
      </div>
    {/if}
    <!-- Save button overlays the bottom-right of the tile, as on the site. -->
    <div class="absolute right-2 bottom-2 bg-red-600 text-white text-xs font-semibold px-3 py-1.5 rounded-full">
      Save
    </div>
  </div>

  <div class="p-3">
    <h3 class="font-semibold text-sm leading-snug">
      {#if title}
        {title}
      {:else}
        <span class="text-red-500 italic">Untitled — a pin requires a title</span>
      {/if}
    </h3>
    <p class="text-[11px] text-gray-700 mt-1 whitespace-pre-wrap break-words">
      {folded}{#if text.length > 100}<span class="text-gray-500"> ...more</span>{/if}
    </p>
    {#if isOver}
      <p class="text-[10px] text-red-600 mt-1">{count}/{limit} — will be rejected, trim {count - limit} more</p>
    {/if}
    <div class="flex items-center gap-1.5 mt-2 pt-2 border-t border-gray-100">
      <div class="w-5 h-5 rounded-full bg-gray-700 flex items-center justify-center text-white text-[9px] font-bold">
        {authorName.charAt(0).toUpperCase()}
      </div>
      <span class="text-[11px] text-gray-600 truncate">{authorName}</span>
      <!-- Board chip: mirrors the TargetPicker selection so the user can see
           which board this pin will land on. -->
      <span class="ml-auto text-[10px] bg-gray-100 text-gray-700 px-2 py-0.5 rounded-full truncate max-w-[120px]">
        {targetLabel || 'No board selected'}
      </span>
    </div>
  </div>
</div>
