<script lang="ts">
  let { content = "", onContentChange, integrationId, platformLabel, charLimit, placeholder }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
    platformLabel: string;
    charLimit: number;
    placeholder?: string;
  } = $props();

  let text = $state(content.replace(/<[^>]*>/g, ""));
  let charCount = $derived(text.length);
  let isOverLimit = $derived(charCount > charLimit);
  let isWarning = $derived(charCount > charLimit * 0.9 && !isOverLimit);

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    onContentChange?.(`<p>${target.value}</p>`);
  }
</script>

<div class="space-y-2">
  <textarea
    value={text}
    oninput={handleInput}
    placeholder={placeholder || `Write your ${platformLabel} post...`}
    aria-label="Post content"
    class="w-full h-24 bg-[#0d1117] border border-[#1e2435] rounded-lg p-3 text-sm resize-none focus:border-indigo-500 outline-none"
  ></textarea>
  <div class="flex justify-between text-xs">
    <span class="text-[#6b7280]">{platformLabel}</span>
    <span class={isOverLimit ? 'text-red-400' : isWarning ? 'text-yellow-400' : 'text-[#6b7280]'}>
      {charCount}/{charLimit}
    </span>
  </div>
</div>
