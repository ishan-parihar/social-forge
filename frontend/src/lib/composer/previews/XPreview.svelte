<script lang="ts">
  // v25 F2: X post preview.
  //
  // Light chrome on purpose: the preview shows the DESTINATION platform's card,
  // not this app's theme. Retinting it to the app's tokens would make the
  // preview a lie — the point is "this is what lands on X". That is why
  // previews/* is exempt from the check-hex-tokens gate.
  //
  // F2 corrected the prop contract: this used to take `integrationName` while
  // PlatformPreviewPane passes `authorName`/`authorHandle`, so the account name
  // silently fell back to a hardcoded 'X Account'. Every preview now speaks the
  // same prop shape.
  import { charLimitFor, countFor, plainText } from '../platforms';
  import type { MediaItem } from '$lib/api/media';

  let {
    content = '',
    authorName = 'Your Brand',
    authorHandle = 'yourbrand',
    media = [] as MediaItem[],
  }: {
    content?: string;
    authorName?: string;
    authorHandle?: string;
    media?: MediaItem[];
  } = $props();

  let text = $derived(plainText(content));
  let limit = $derived(charLimitFor('x'));
  let count = $derived(countFor('x', text));
  let isOver = $derived(count > limit);
  // X renders one 16:9 media card, or a 2-up / 3-up / 4-up grid. Video is exclusive.
  let images = $derived(media.filter(m => m.mime_type?.startsWith('image/')).slice(0, 4));
  let video = $derived(media.find(m => !m.mime_type?.startsWith('image/')));
  let gridCols = $derived(images.length === 3 ? 'grid-cols-3' : 'grid-cols-2');
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden" style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
  <div class="flex gap-3 p-4">
    <div class="w-10 h-10 rounded-full bg-gray-800 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {authorName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="flex items-center gap-1 text-sm flex-wrap">
        <span class="font-bold">{authorName}</span>
        <svg class="w-4 h-4 text-sky-500 flex-shrink-0" viewBox="0 0 24 24" fill="currentColor"><path d="M22.5 12.5c0-1.58-.875-2.95-2.148-3.6.154-.435.238-.905.238-1.4 0-2.21-1.71-3.998-3.818-3.998-.47 0-.92.084-1.336.25C14.818 2.415 13.51 1.5 12 1.5s-2.816.917-3.437 2.25c-.415-.165-.866-.25-1.336-.25-2.11 0-3.818 1.79-3.818 4 0 .494.083.964.237 1.4-1.272.65-2.147 2.018-2.147 3.6 0 1.495.782 2.798 1.942 3.486-.02.17-.032.34-.032.514 0 2.21 1.708 4 3.818 4 .47 0 .92-.086 1.335-.25.62 1.334 1.926 2.25 3.437 2.25 1.512 0 2.818-.916 3.437-2.25.415.163.865.248 1.336.248 2.11 0 3.818-1.79 3.818-4 0-.174-.012-.344-.033-.513 1.16-.687 1.943-1.99 1.943-3.484zm-6.616-3.334l-4.334 6.5c-.145.217-.382.334-.625.334-.143 0-.288-.04-.416-.126l-.115-.094-2.415-2.415c-.293-.293-.293-.768 0-1.06s.768-.294 1.06 0l1.77 1.767 3.825-5.74c.23-.345.696-.436 1.04-.207.346.23.44.696.21 1.04z"/></svg>
        <span class="text-gray-500">@{authorHandle}</span>
        <span class="text-gray-400">· 2h</span>
      </div>

      <!-- Overflow is marked, not hidden. X's publishing API rejects an
           over-length post rather than truncating it, so the user has to see
           exactly which words will cost them the slot. -->
      <p class="mt-1 text-sm whitespace-pre-wrap break-words">
        {text.slice(0, limit)}<span class="bg-red-100 text-red-600 rounded-sm">{text.slice(limit)}</span>
      </p>
      {#if isOver}
        <p class="text-[10px] text-red-600 mt-1">{count}/{limit} — will be rejected, trim {count - limit} more</p>
      {/if}

      {#if video}
        <div class="mt-2 rounded-xl overflow-hidden border border-gray-200 bg-gray-100 aspect-video flex items-center justify-center text-gray-400 text-xs">video</div>
      {:else if images.length > 0}
        <div class="mt-2 grid {gridCols} gap-0.5 rounded-xl overflow-hidden border border-gray-200">
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
