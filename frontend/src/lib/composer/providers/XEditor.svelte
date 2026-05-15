<script lang="ts">
  let { content = "", onContentChange, integrationId }: {
    content?: string; integrationId?: string;
    onContentChange?: (html: string) => void;
  } = $props();

  let text = $state(content.replace(/<[^>]*>/g, ""));
  let charCount = $derived(text.length);
  let isOverLimit = $derived(charCount > 280);

  function handleInput(e: Event) {
    const target = e.target as HTMLTextAreaElement;
    text = target.value;
    onContentChange?.(target.value);
  }
</script>

<div class="space-y-2">
  <textarea
    value={text}
    oninput={handleInput}
    maxlength="300"
    placeholder="What's happening?"
    class="w-full h-24 bg-[#0d1117] border border-[#1e2435] rounded-lg p-3 text-sm resize-none focus:border-indigo-500 outline-none"
  ></textarea>
  <div class="flex justify-between text-xs">
    <span class="text-[#6b7280]">X / Twitter</span>
    <span class={isOverLimit ? 'text-red-400' : charCount > 260 ? 'text-yellow-400' : 'text-[#6b7280]'}>
      {charCount}/280
    </span>
  </div>
</div>
