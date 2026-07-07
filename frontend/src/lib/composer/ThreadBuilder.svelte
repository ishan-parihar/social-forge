<script lang="ts">
  // v26-4: ThreadBuilder — a generic thread builder with per-row delay.
  //
  // Replaces the X-only ThreadFinisher. Works for ANY provider (X,
  // Bluesky, Mastodon, Threads, Reddit, etc. — any that supports
  // multi-post threads). Each part is editable, has a per-row char count
  // against the provider's char limit, and the delay between parts can
  // be set in minutes (the backend v24-5 supports delay_minutes).
  //
  // The "Split from content" button uses the same paragraph-aware
  // splitting logic as the old ThreadFinisher, but the user can then
  // edit each part independently before posting.

  import { providerCharLimit } from '$lib/providers';

  let {
    content = '',
    provider = 'x',
    onCreateThread,
    submitting = false,
  }: {
    content?: string;
    /** Provider identifier (e.g. 'x', 'bluesky', 'reddit') — used for char limit. */
    provider?: string;
    onCreateThread?: (parts: string[], delayMinutes?: number) => void;
    submitting?: boolean;
  } = $props();

  let expanded = $state(false);
  let parts = $state<string[]>([]);
  let delayMinutes = $state(0);
  const MAX_PARTS = 25;

  let charLimit = $derived(providerCharLimit(provider));

  // Split content into thread parts using paragraph-aware logic.
  // Falls back to word-boundary splitting for long paragraphs.
  function splitIntoThread(text: string, maxLen: number): string[] {
    const paragraphs = text.split(/\n\s*\n/).filter(p => p.trim());
    const result: string[] = [];
    for (const para of paragraphs) {
      if (para.length <= maxLen) {
        result.push(para.trim());
      } else {
        let remaining = para.trim();
        while (remaining.length > maxLen) {
          let splitAt = remaining.lastIndexOf(' ', maxLen);
          if (splitAt === -1) splitAt = maxLen;
          result.push(remaining.slice(0, splitAt).trim());
          remaining = remaining.slice(splitAt).trim();
        }
        if (remaining) result.push(remaining);
      }
    }
    return result;
  }

  let totalChars = $derived(parts.reduce((sum, p) => sum + p.length, 0));
  let exceedsMax = $derived(parts.length > MAX_PARTS);
  let validParts = $derived(parts.filter(p => p.trim().length > 0));

  function toggle() { expanded = !expanded; }

  function splitFromContent() {
    parts = splitIntoThread(content, charLimit).slice(0, MAX_PARTS);
    if (parts.length === 0) {
      // No content to split — start with one empty part.
      parts = [''];
    }
  }

  function addPart() {
    if (parts.length >= MAX_PARTS) return;
    parts = [...parts, ''];
  }

  function removePart(index: number) {
    parts = parts.filter((_, i) => i !== index);
  }

  function movePart(index: number, direction: 'up' | 'down') {
    if (direction === 'up' && index === 0) return;
    if (direction === 'down' && index === parts.length - 1) return;
    const newIndex = direction === 'up' ? index - 1 : index + 1;
    const newParts = [...parts];
    [newParts[index], newParts[newIndex]] = [newParts[newIndex], newParts[index]];
    parts = newParts;
  }

  function updatePart(index: number, value: string) {
    parts = parts.map((p, i) => i === index ? value : p);
  }

  function handlePostThread() {
    if (validParts.length === 0) return;
    onCreateThread?.(validParts, delayMinutes > 0 ? delayMinutes : undefined);
  }

  // Auto-split when expanding for the first time if parts is empty.
  $effect(() => {
    if (expanded && parts.length === 0 && content.trim()) {
      splitFromContent();
    }
  });
</script>

<div class="bg-surface border border-line rounded-xl p-4 space-y-3">
  <button
    onclick={toggle}
    class="flex items-center justify-between w-full text-left"
  >
    <h3 class="text-sm font-semibold flex items-center gap-2">
      🧵 Thread Builder
    </h3>
    <span class="text-xs text-muted">{expanded ? '▾' : '▸'}</span>
  </button>

  {#if expanded}
    {#if parts.length === 0}
      <div class="text-center py-4">
        <p class="text-xs text-muted mb-3">Split your content into a multi-part thread, or add parts manually.</p>
        <button
          onclick={splitFromContent}
          class="px-3 py-1.5 bg-brand-500 hover:bg-brand-600 rounded-lg text-xs text-white font-medium transition-colors"
        >
          ✂️ Split from content
        </button>
        <button
          onclick={addPart}
          class="ml-2 px-3 py-1.5 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-xs text-muted transition-colors"
        >
          + Add empty part
        </button>
      </div>
    {:else}
      <!-- Parts list -->
      <div class="space-y-2 max-h-80 overflow-y-auto pr-1">
        {#each parts as part, i (i)}
          <div class="border border-line rounded-lg p-3 space-y-2">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="text-xs font-medium text-brand-400">Part {i + 1}</span>
                <div class="flex items-center gap-0.5">
                  <button
                    onclick={() => movePart(i, 'up')}
                    disabled={i === 0}
                    class="w-5 h-5 flex items-center justify-center rounded text-muted hover:text-content hover:bg-surface-hover disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    aria-label="Move up"
                  >↑</button>
                  <button
                    onclick={() => movePart(i, 'down')}
                    disabled={i === parts.length - 1}
                    class="w-5 h-5 flex items-center justify-center rounded text-muted hover:text-content hover:bg-surface-hover disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                    aria-label="Move down"
                  >↓</button>
                </div>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-xs {part.length > charLimit ? 'text-error' : 'text-muted'}">
                  {part.length} / {charLimit}
                </span>
                <button
                  onclick={() => removePart(i)}
                  class="w-5 h-5 flex items-center justify-center rounded text-muted hover:text-error hover:bg-error/10 transition-colors"
                  aria-label="Remove part {i + 1}"
                >✕</button>
              </div>
            </div>
            <textarea
              bind:value={parts[i]}
              oninput={(e) => updatePart(i, e.currentTarget.value)}
              rows="2"
              class="w-full px-2 py-1.5 bg-background-input border border-line rounded text-sm text-content resize-y focus:border-brand-500 outline-none"
              placeholder="Thread part {i + 1} content..."
            ></textarea>
          </div>
        {/each}
      </div>

      {#if exceedsMax}
        <p class="text-xs text-warning">Thread exceeds max of {MAX_PARTS} parts. Only first {MAX_PARTS} will be posted.</p>
      {/if}

      <!-- Controls row -->
      <div class="flex items-center justify-between gap-2 pt-2 border-t border-line">
        <button
          onclick={addPart}
          disabled={parts.length >= MAX_PARTS}
          class="px-2 py-1 text-xs text-brand-400 hover:text-brand-300 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >+ Add part</button>
        <div class="flex items-center gap-2">
          <label class="text-xs text-muted">Delay between parts:</label>
          <input
            type="number"
            bind:value={delayMinutes}
            min="0"
            max="1440"
            class="w-16 px-2 py-1 bg-background-input border border-line rounded text-xs text-content-secondary text-center"
          />
          <span class="text-xs text-muted">min</span>
        </div>
      </div>

      {#if delayMinutes > 0 && validParts.length > 1}
        <p class="text-[10px] text-muted-dark">
          Part 1 posts at the scheduled time. Each subsequent part posts {delayMinutes} min after the previous.
        </p>
      {/if}

      <!-- Summary + post button -->
      <div class="flex items-center justify-between pt-2">
        <span class="text-xs text-muted">{validParts.length} parts · {totalChars} chars total</span>
        <button
          onclick={handlePostThread}
          disabled={submitting || validParts.length === 0}
          class="px-4 py-2 bg-brand-500 hover:bg-brand-600 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm text-white font-medium transition-colors"
        >
          {submitting ? 'Posting...' : `Post Thread (${validParts.length})`}
        </button>
      </div>
    {/if}
  {/if}
</div>
