<script lang="ts">
  // v25 F2: YouTube preview.
  //
  // Light chrome = destination-platform simulation (see previews/index.ts).
  //
  // A YouTube post is a VIDEO UPLOAD, not a text post: the Content Posting API
  // takes a title (required), a description (the body), and the media file.
  // So the frame is a channel-row video card — 16:9 thumbnail, title above the
  // fold, description folded under "show more" exactly as YouTube renders it.
  // The "Title is required" placeholder is deliberate: an untitled YouTube
  // upload cannot publish, and the preview is where that should become obvious.
  import { charLimitFor, countFor, plainText } from '../platforms';
  import type { MediaItem } from '$lib/api/media';

  let {
    content = '',
    title = '',
    authorName = 'Your Brand',
    media = [] as MediaItem[],
  }: {
    content?: string;
    title?: string;
    authorName?: string;
    media?: MediaItem[];
  } = $props();

  let text = $derived(plainText(content));
  let limit = $derived(charLimitFor('youtube'));
  let count = $derived(countFor('youtube', text));
  let isOver = $derived(count > limit);
  let video = $derived(media.find(m => !m.mime_type?.startsWith('image/')));

  // YouTube truncates the description past ~160 chars behind "more"; the
  // preview shows the first two lines, which is what the user actually sees.
  let folded = $derived(text.length > 160 ? text.slice(0, 160) : text);
</script>

<div class="bg-white text-gray-900 rounded-xl border border-gray-200 overflow-hidden max-w-[500px] mx-auto" style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;">
  <div class="flex gap-3 p-3">
    <div class="w-9 h-9 rounded-full bg-red-600 flex-shrink-0 flex items-center justify-center text-white text-sm font-bold">
      {authorName.charAt(0).toUpperCase()}
    </div>
    <div class="flex-1 min-w-0">
      <div class="text-sm font-semibold">{authorName}</div>
      <div class="text-xs text-gray-500">1.2K subscribers · Just now</div>
    </div>
    <span class="text-gray-400 text-lg">•••</span>
  </div>

  <!-- 16:9 thumbnail -->
  <div class="w-full aspect-video bg-gray-100 flex items-center justify-center overflow-hidden">
    {#if video}
      <!-- ponytail: a <video> element with a poster would be more accurate but
           costs a decode per keystroke-triggered re-render. The frame the
           platform will show is the same box; the poster arrives with the
           real thumbnail once the upload finishes. -->
      <div class="text-center text-gray-400 text-xs">
        <svg class="w-10 h-10 mx-auto mb-1" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        Video ready
      </div>
    {:else}
      <div class="text-center text-red-500 text-xs px-4">
        <svg class="w-10 h-10 mx-auto mb-1 text-red-400" viewBox="0 0 24 24" fill="currentColor"><path d="M8 5v14l11-7z"/></svg>
        A YouTube post cannot publish without a video
      </div>
    {/if}
  </div>

  <div class="p-3">
    <h3 class="font-semibold text-[15px] leading-snug mb-1">
      {#if title}
        {title}
      {:else}
        <span class="text-red-500 italic">Untitled — YouTube requires a title</span>
      {/if}
    </h3>
    <p class="text-xs text-gray-600 whitespace-pre-wrap break-words">
      {folded}{#if text.length > 160}<span class="text-gray-500"> ...more</span>{/if}
    </p>
    {#if isOver}
      <p class="text-[10px] text-red-600 mt-1">{count}/{limit} — will be rejected, trim {count - limit} more</p>
    {/if}
  </div>

  <div class="flex items-center gap-4 px-3 py-2 border-t border-gray-200 text-xs text-gray-500">
    <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="currentColor"><path d="M1 21V3.5C1 2.7 1.7 2 2.5 2H19v20H2.5C1.7 22 1 21.3 1 20.5v-8C1 11.3 1.3 10.7 2 10.7h15V4H3v17z"/></svg> Approve</span>
    <span class="flex items-center gap-1"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/></svg> Comment</span>
    <span class="ml-auto">Public</span>
  </div>
</div>
