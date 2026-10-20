<script lang="ts">
  import { ai } from "$lib/api/ai";

  let { content = "", onAddHashtag }: {
    content?: string;
    onAddHashtag?: (tag: string) => void;
  } = $props();

  let hashtags = $state<string[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let open = $state(false);
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
      // DEDUPE.
      //
      // The backend returns the LLM's raw free-text, and the frontend
      // splits it on whitespace. The LLM repeats itself constantly -
      // "ai ai marketing" is a perfectly ordinary completion - and the
      // chips are a KEYED each (`{#each hashtags as tag (tag)}`). A
      // repeated key is a HARD THROW in Svelte (`each_key_duplicate`),
      // and this component is rendered inside the composer: the throw
      // took down the whole modal and the founder lost the text they
      // were writing.
      const tags = Array.from(
        new Set(
          result
            .split(/\s+/)
            .map((t: string) => t.replace(/^#/, "").trim())
            .filter((t: string) => t.length > 0),
        ),
      );
      hashtags = tags;
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") return;
      error = e instanceof Error ? e.message : "Failed to generate hashtags";
      hashtags = [];
    } finally {
      loading = false;
    }
  }

  /**
   * The section is CLOSED until the founder opens it.
   *
   * It used to fetch automatically, 2 seconds after the content
   * changed. On an instance with no LLM configured - which is every
   * instance whose operator has not set one up - that meant a RED
   * ERROR appeared in the composer the moment the founder typed,
   * without the founder having asked for anything. An error the user
   * did not provoke, in the middle of the panel they were working in,
   * is an error they cannot act on.
   *
   * So the suggestions are asked for, and until they are asked for
   * there is nothing on screen at all. The button says what it will
   * do.
   */
  function toggle() {
    open = !open;
    if (open && hashtags.length === 0 && !loading) {
      fetchHashtags();
    }
  }

  function handleContentChange() {
    // Only refresh what is already on screen. A closed section has
    // nothing to refresh.
    if (!open) return;
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

<div class="bg-surface border border-line rounded-xl">
  <button
    onclick={toggle}
    aria-expanded={open}
    class="w-full flex items-center justify-between gap-2 px-4 py-3 min-h-11 text-left"
  >
    <span class="text-xs font-semibold text-muted">
      Hashtag suggestions
      {#if hashtags.length > 0}
        <span class="text-content-secondary">({hashtags.length})</span>
      {/if}
    </span>
    <span class="text-muted text-xs" aria-hidden="true">{open ? '▾' : '▸'}</span>
  </button>

  {#if open}
    <div class="px-4 pb-4">
      {#if loading}
        <div class="flex items-center gap-2 text-xs text-muted">
          <span class="inline-block w-3 h-3 border-2 border-accent/30 border-t-brand-400 rounded-full animate-spin" aria-hidden="true"></span>
          Generating hashtag suggestions...
        </div>
      {:else if error}
        <p class="text-xs text-error">{error}</p>
        <button
          onclick={fetchHashtags}
          class="mt-2 text-xs text-accent hover:text-accent-strong min-h-11 px-2 -ml-2"
        >
          Try again
        </button>
      {:else if hashtags.length > 0}
        <div class="flex flex-wrap gap-2">
          {#each hashtags as tag (tag)}
            <button
              onclick={() => onAddHashtag?.(tag)}
              aria-label="Add {tag} hashtag"
              class="px-2.5 py-1 min-h-11 bg-surface-hover hover:bg-accent-fill-hover/20 text-xs text-content-secondary rounded-full transition-colors cursor-pointer"
            >
              #{tag}
            </button>
          {/each}
        </div>
      {:else}
        <p class="text-xs text-muted">
          {content.trim()
            ? 'No suggestions yet - open this and they will be generated.'
            : 'Write something first, then ask for suggestions.'}
        </p>
      {/if}
    </div>
  {/if}
</div>
