<script lang="ts">
  let { content = "", onContentChange, integrationId, platformLabel, charLimit, placeholder }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
    platformLabel: string;
    charLimit: number;
    placeholder?: string;
  } = $props();

  // Re-derive plain text from the content prop whenever it changes.
  // Previously `text` was initialized once from `content` and never
  // synced, so switching between global and per-channel mode left the
  // editor showing stale text from the previous mode.
  let text = $derived(content.replace(/<[^>]*>/g, ""));
  let charCount = $derived(text.length);
  let isOverLimit = $derived(charCount > charLimit);
  let isWarning = $derived(charCount > charLimit * 0.9 && !isOverLimit);

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    onContentChange?.(`<p>${target.value}</p>`);
  }
</script>

<div class="space-y-2">
  <textarea
    value={text}
    oninput={handleInput}
    placeholder={placeholder || `Write your ${platformLabel} post...`}
    aria-label="Post content"
    class="w-full h-24 bg-background-input border border-line rounded-lg p-3 text-sm resize-none focus:border-indigo-500 outline-none"
  ></textarea>
  <div class="flex justify-between text-xs">
    <span class="text-muted">{platformLabel}</span>
    <span class={isOverLimit ? 'text-red-400' : isWarning ? 'text-yellow-400' : 'text-muted'}>
      {charCount}/{charLimit}
    </span>
  </div>
</div>
