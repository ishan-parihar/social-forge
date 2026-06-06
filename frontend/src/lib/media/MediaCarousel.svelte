<script lang="ts">
  import { tick } from 'svelte';
  import { proxyMediaUrl } from '$lib/api/feed';
  import type { MediaAttachment } from '$lib/api/feed';

  let {
    items = [],
  }: {
    items?: MediaAttachment[];
  } = $props();

  let current = $state(0);
  let fullscreen = $state(false);
  let touchStartX = $state(0);
  let touchEndX = $state(0);
  let containerEl = $state<HTMLDivElement | null>(null);
  let fullscreenEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    current = 0;
  });

  function prev() {
    current = current > 0 ? current - 1 : items.length - 1;
  }

  function next() {
    current = current < items.length - 1 ? current + 1 : 0;
  }

  function goTo(index: number) {
    current = index;
  }

  function isVideo(mimeType: string | undefined, url: string): boolean {
    if (mimeType && mimeType.startsWith('video/')) return true;
    if (url.match(/\.(mp4|webm|mov|avi|mkv)(\?|$)/i)) return true;
    return false;
  }

  function isEmbed(mimeType: string | undefined): boolean {
    return mimeType === 'text/html';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft') { e.preventDefault(); prev(); }
    if (e.key === 'ArrowRight') { e.preventDefault(); next(); }
    if (e.key === 'Escape') { fullscreen = false; }
  }

  function handleTouchStart(e: TouchEvent) {
    touchStartX = e.touches[0].clientX;
  }

  function handleTouchEnd(e: TouchEvent) {
    touchEndX = e.changedTouches[0].clientX;
    const diff = touchStartX - touchEndX;
    if (Math.abs(diff) > 50) {
      if (diff > 0) next();
      else prev();
    }
  }

  async function openFullscreen() {
    fullscreen = true;
    await tick();
    fullscreenEl?.focus();
  }

  function closeFullscreen() {
    fullscreen = false;
  }
</script>

{#if items.length > 0}
  <!-- Carousel container -->
  <div
    bind:this={containerEl}
    class="group relative rounded-xl overflow-hidden bg-[#0d121e] ring-1 ring-[#1e2435]"
    role="region"
    aria-label="Media carousel"
    aria-roledescription="carousel"
    tabindex="0"
    onkeydown={handleKeydown}
    ontouchstart={handleTouchStart}
    ontouchend={handleTouchEnd}
  >
    <!-- Current media -->
    <div class="relative w-full" style="aspect-ratio: 16/9; max-height: 65vh;">
      {#each items as item, i (i)}
        <div
          class="absolute inset-0 transition-all duration-300 ease-out"
          class:opacity-100={i === current}
          class:opacity-0={i !== current}
          class:scale-100={i === current}
          class:scale-95={i !== current}
          aria-hidden={i !== current}
          role="group"
          aria-roledescription="slide"
          aria-label={`Slide ${i + 1} of ${items.length}`}
        >
          {#if isEmbed(item.mime_type)}
            <iframe
              src={item.url}
              title={item.alt ?? 'Embedded video'}
              class="w-full h-full bg-black"
              allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
              allowfullscreen
              loading="lazy"
              referrerpolicy="strict-origin-when-cross-origin"
            ></iframe>
          {:else if isVideo(item.mime_type, item.url)}
            <div class="relative w-full h-full">
              <video
                src={proxyMediaUrl(item.url)}
                controls
                preload="metadata"
                playsinline
                class="w-full h-full object-contain"
                poster={item.poster_url ? proxyMediaUrl(item.poster_url) : ''}
              >
                <a href={item.url} target="_blank" rel="noopener noreferrer"
                  class="text-xs text-indigo-400 hover:text-indigo-300 underline p-2 block">
                  Download video
                </a>
              </video>
              {#if !item.poster_url}
                <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
                  <div class="w-14 h-14 rounded-full bg-white/90 flex items-center justify-center shadow-lg">
                    <svg class="w-6 h-6 text-gray-900 ml-0.5" viewBox="0 0 24 24" fill="currentColor">
                      <path d="M8 5v14l11-7z"/>
                    </svg>
                  </div>
                </div>
              {/if}
            </div>
          {:else}
            <button
              onclick={openFullscreen}
              class="w-full h-full p-0 border-0 cursor-pointer bg-transparent block"
              aria-label="View fullscreen"
            >
              <img
                src={proxyMediaUrl(item.url)}
                alt={item.alt ?? ''}
                class="w-full h-full object-contain"
                loading="lazy"
                draggable="false"
              />
            </button>
          {/if}
        </div>
      {/each}

      <!-- Overlay gradient for nav buttons -->
      <div class="absolute inset-0 pointer-events-none" />

      <!-- Prev / Next buttons -->
      {#if items.length > 1}
        <button
          onclick={prev}
          class="absolute left-2 top-1/2 -translate-y-1/2 w-9 h-9 rounded-full
            bg-black/50 hover:bg-black/70 text-white flex items-center justify-center
            opacity-0 group-hover:opacity-100 transition-all duration-200
            backdrop-blur-sm ring-1 ring-white/10
            focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-indigo-400"
          aria-label="Previous slide"
        >
          <svg class="w-4 h-4" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10 3l-5 5 5 5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
        <button
          onclick={next}
          class="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 rounded-full
            bg-black/50 hover:bg-black/70 text-white flex items-center justify-center
            opacity-0 group-hover:opacity-100 transition-all duration-200
            backdrop-blur-sm ring-1 ring-white/10
            focus:opacity-100 focus:outline-none focus:ring-2 focus:ring-indigo-400"
          aria-label="Next slide"
        >
          <svg class="w-4 h-4" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 3l5 5-5 5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      {/if}
    </div>

    <!-- Dot indicators + counter -->
    {#if items.length > 1}
      <div class="flex items-center justify-center gap-1.5 px-4 py-2.5 bg-[#0d121e]/90 border-t border-[#1a2035]">
        <button
          onclick={prev}
          class="p-1 text-[#4a5060] hover:text-[#9ca3af] transition-colors"
          aria-label="Previous slide"
        >
          <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M10 3l-5 5 5 5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>

        <div class="flex items-center gap-1.5 mx-2">
          {#each items as _, i (i)}
            <button
              onclick={() => goTo(i)}
              class="rounded-full transition-all duration-300"
              class:w-2.5={i === current}
              class:w-2={i !== current}
              class:h-2.5={i === current}
              class:h-2={i !== current}
              class:bg-indigo-400={i === current}
              class:bg-[#2a3045]={i !== current}
              class:shadow-[0_0_6px_rgba(99,102,241,0.3)]={i === current}
              aria-label={`Go to slide ${i + 1}`}
              aria-current={i === current ? 'true' : undefined}
            />
          {/each}
        </div>

        <button
          onclick={next}
          class="p-1 text-[#4a5060] hover:text-[#9ca3af] transition-colors"
          aria-label="Next slide"
        >
          <svg class="w-3 h-3" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M6 3l5 5-5 5" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>

        <span class="ml-2 text-[10px] text-[#5a6070] font-mono tabular-nums">
          {current + 1}/{items.length}
        </span>
      </div>
    {/if}
  </div>
{/if}

<!-- Fullscreen overlay -->
{#if fullscreen}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    bind:this={fullscreenEl}
    role="dialog"
    aria-modal="true"
    aria-label="Fullscreen media viewer"
    tabindex="-1"
    class="fixed inset-0 z-50 bg-black/95 backdrop-blur-xl flex items-center justify-center"
    onkeydown={handleKeydown}
    onclick={closeFullscreen}
    ontouchstart={handleTouchStart}
    ontouchend={handleTouchEnd}
  >
    <!-- Close button -->
    <button
      onclick={closeFullscreen}
      class="absolute top-4 right-4 w-10 h-10 rounded-full bg-white/10 hover:bg-white/20
        text-white flex items-center justify-center transition-all z-10
        backdrop-blur-sm ring-1 ring-white/10"
      aria-label="Close fullscreen"
    >
      <svg class="w-5 h-5" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M5 5l10 10M15 5l-10 10" stroke-linecap="round" />
      </svg>
    </button>

    <!-- Counter -->
    <div class="absolute top-4 left-4 px-3 py-1.5 rounded-full bg-black/50 backdrop-blur-sm
      text-xs text-white/70 font-mono ring-1 ring-white/10 z-10">
      {current + 1} / {items.length}
    </div>

    <!-- Previous / Next fullscreen -->
    {#if items.length > 1}
      <button
        onclick={(e) => { e.stopPropagation(); prev(); }}
        class="absolute left-4 top-1/2 -translate-y-1/2 w-12 h-12 rounded-full
          bg-white/10 hover:bg-white/20 text-white flex items-center justify-center
          transition-all z-10 backdrop-blur-sm ring-1 ring-white/10"
        aria-label="Previous"
      >
        <svg class="w-5 h-5" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M13 4l-7 6 7 6" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
      <button
        onclick={(e) => { e.stopPropagation(); next(); }}
        class="absolute right-4 top-1/2 -translate-y-1/2 w-12 h-12 rounded-full
          bg-white/10 hover:bg-white/20 text-white flex items-center justify-center
          transition-all z-10 backdrop-blur-sm ring-1 ring-white/10"
        aria-label="Next"
      >
        <svg class="w-5 h-5" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M7 4l7 6-7 6" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    {/if}

    <!-- Current slide in fullscreen -->
    <div class="max-w-[90vw] max-h-[90vh]" onclick={(e) => e.stopPropagation()}>
      {#if isEmbed(items[current]?.mime_type ?? '')}
        <iframe
          src={items[current].url}
          title={items[current].alt ?? 'Embedded video'}
          class="w-full h-full min-h-[60vh] rounded-lg bg-black"
          allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture"
          allowfullscreen
          referrerpolicy="strict-origin-when-cross-origin"
        ></iframe>
      {:else if isVideo(items[current]?.mime_type ?? '', items[current]?.url ?? '')}
        <video
          src={proxyMediaUrl(items[current].url)}
          controls
          autoplay
          class="max-w-full max-h-[85vh] rounded-lg"
        />
      {:else}
        <img
          src={proxyMediaUrl(items[current].url)}
          alt={items[current].alt ?? ''}
          class="max-w-full max-h-[85vh] object-contain rounded-lg"
          draggable="false"
        />
      {/if}
    </div>
  </div>
{/if}
