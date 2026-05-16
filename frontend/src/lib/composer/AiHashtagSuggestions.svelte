<script lang="ts">
  import { ai } from "$lib/api/ai";

  let { content = "", onAddHashtag }: {
    content?: string;
    onAddHashtag?: (tag: string) => void;
  } = $props();

  let hashtags = $state<string[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let abortController = $state<AbortController | null>(null);

  async function fetchHashtags() {
    abortController?.abort();
    abortController = new AbortController();
    if (!content.trim()) {
      hashtags = [];
      return;
    }
    loading = true;
    error = null;
    try {
      const result = await ai.suggestHashtags(content, abortController.signal);
      const tags = result
        .split(/\s+/)
        .map((t: string) => t.replace(/^#/, "").trim())
        .filter((t: string) => t.length > 0);
      hashtags = tags;
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return;
      error = e instanceof Error ? e.message : "Failed to generate hashtags";
      hashtags = [];
    } finally {
      loading = false;
    }
  }

  function handleContentChange() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(fetchHashtags, 2000);
  }

  $effect(() => {
    content;
    handleContentChange();
    return () => {
      if (debounceTimer) clearTimeout(debounceTimer);
      abortController?.abort();
    };
  });
</script>

{#if loading}
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <div class="flex items-center gap-2 text-xs text-[#6b7280]">
      <span class="inline-block w-3 h-3 border-2 border-indigo-400/30 border-t-indigo-400 rounded-full animate-spin"></span>
      Generating hashtag suggestions...
    </div>
  </div>
{:else if error}
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <p class="text-xs text-red-400">{error}</p>
  </div>
{:else if hashtags.length > 0}
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4 space-y-2">
    <h3 class="text-xs font-semibold text-[#6b7280]">Suggested Hashtags</h3>
    <div class="flex flex-wrap gap-2">
      {#each hashtags as tag (tag)}
        <button
          onclick={() => onAddHashtag?.(tag)}
          aria-label="Add {tag} hashtag"
          class="px-2.5 py-1 bg-[#1e2435] hover:bg-indigo-600/20 text-xs text-[#d1d5db] rounded-full transition-colors cursor-pointer"
        >
          #{tag}
        </button>
      {/each}
    </div>
  </div>
{/if}
