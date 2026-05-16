<script lang="ts">
  import { ai } from "$lib/api/ai";

  let { content = "", onInsert }: {
    content?: string;
    onInsert?: (text: string) => void;
  } = $props();

  let aiError = $state<string | null>(null);
  let aiLoading = $state(false);
  let aiResult = $state<string | null>(null);
  let selectedTask = $state<"generate" | "improve" | "hashtags" | "tone" | "summarize">("generate");
  let topic = $state("");
  let tone = $state("professional");
  let length = $state("medium");

  const tasks = ["generate", "improve", "hashtags", "tone", "summarize"] as const;
  const tones = ["professional", "casual", "humorous", "inspirational"];
  const lengths = ["short", "medium", "long"];

  async function handleGenerate() {
    if (aiLoading) return;
    aiLoading = true;
    aiError = null;
    aiResult = null;
    try {
      let result = "";
      switch (selectedTask) {
        case "generate":
          if (!topic.trim()) { aiError = "Please enter a topic"; aiLoading = false; return; }
          result = await ai.generatePost(topic, tone, length);
          break;
        case "improve":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.improveWriting(content);
          break;
        case "hashtags":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.suggestHashtags(content);
          break;
        case "tone":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.changeTone(content, tone);
          break;
        case "summarize":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.summarize(content);
          break;
      }
      aiResult = result;
    } catch (e: any) {
      aiError = e.message || "AI request failed. Check that LLM-Proxy is running on port 4488.";
    } finally {
      aiLoading = false;
    }
  }

  function handleInsert() {
    if (aiResult) {
      onInsert?.(aiResult);
      aiResult = null;
    }
  }
</script>

<div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4 space-y-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold flex items-center gap-2">
      <span class="text-indigo-400">✨</span>
      AI Assistant
    </h3>
  </div>

  <!-- Task selector -->
  <div class="flex flex-wrap gap-2">
    {#each tasks as task}
      <button
        onclick={() => { selectedTask = task; aiResult = null; aiError = null; }}
        class="px-3 py-1.5 text-xs rounded-lg border transition-colors
          {selectedTask === task
            ? 'bg-indigo-600/20 text-indigo-400 border-indigo-500/30'
            : 'text-[#6b7280] border-[#1e2435] hover:text-white hover:border-[#374151]'}"
      >
        {task === "generate" ? "Generate" : task === "improve" ? "Improve" : task === "hashtags" ? "Hashtags" : task === "tone" ? "Tone" : "Summarize"}
      </button>
    {/each}
  </div>

  <!-- Conditional inputs -->
  <div class="space-y-3">
    {#if selectedTask === "generate"}
      <div>
        <label for="ai-topic" class="text-xs text-[#6b7280] block mb-1">Topic</label>
        <input
          id="ai-topic"
          type="text"
          bind:value={topic}
          placeholder="e.g. Our new product launch..."
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db] focus:border-indigo-500 outline-none"
        />
      </div>
      <div class="flex gap-3">
        <div class="flex-1">
          <label for="ai-tone" class="text-xs text-[#6b7280] block mb-1">Tone</label>
          <select
            id="ai-tone"
            bind:value={tone}
            class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db] focus:border-indigo-500 outline-none"
          >
            {#each tones as t (t)}
              <option value={t}>{t}</option>
            {/each}
          </select>
        </div>
        <div class="flex-1">
          <label for="ai-length" class="text-xs text-[#6b7280] block mb-1">Length</label>
          <select
            id="ai-length"
            bind:value={length}
            class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db] focus:border-indigo-500 outline-none"
          >
            {#each lengths as l (l)}
              <option value={l}>{l}</option>
            {/each}
          </select>
        </div>
      </div>
    {:else if selectedTask === "tone"}
      <div>
        <label for="ai-target-tone" class="text-xs text-[#6b7280] block mb-1">Target Tone</label>
        <select
          id="ai-target-tone"
          bind:value={tone}
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db] focus:border-indigo-500 outline-none"
        >
          {#each tones as t (t)}
            <option value={t}>{t}</option>
          {/each}
        </select>
      </div>
    {/if}

    {#if selectedTask !== "generate"}
      <p class="text-xs text-[#6b7280]">Using current post content as input.</p>
    {/if}
  </div>

  <!-- Generate button -->
  <button
    onclick={handleGenerate}
    disabled={aiLoading}
    class="w-full px-3 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
  >
    {#if aiLoading}
      <span class="inline-block w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin"></span>
      Generating...
    {:else}
      Generate
    {/if}
  </button>

  <!-- Error -->
  {#if aiError}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3">{aiError}</div>
  {/if}

  <!-- Result -->
  {#if aiResult}
    <div class="space-y-2">
      <div class="bg-[#0d1117] border border-[#1e2435] rounded-lg p-3 text-sm text-[#d1d5db] whitespace-pre-wrap max-h-48 overflow-y-auto">
        {aiResult}
      </div>
      <div class="flex gap-2">
        <button
          onclick={handleInsert}
          class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-xs transition-colors"
        >
          Insert
        </button>
        <button
          onclick={() => { aiResult = null; }}
          class="px-3 py-1.5 text-xs text-[#6b7280] hover:text-white border border-[#1e2435] rounded-lg transition-colors"
        >
          Discard
        </button>
      </div>
    </div>
  {/if}
</div>
