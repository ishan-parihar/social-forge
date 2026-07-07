<script lang="ts">
  import { ai } from "$lib/api/ai";
  import { profileApi, type BrandProfile } from "$lib/api/profile";
  import { onMount } from "svelte";

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

  // v24-4: load the brand profile so AI requests include brand context.
  let brandProfile = $state<BrandProfile | null>(null);

  const tasks = ["generate", "improve", "hashtags", "tone", "summarize"] as const;
  const tones = ["professional", "casual", "humorous", "inspirational"];
  const lengths = ["short", "medium", "long"];

  onMount(async () => {
    const r = await profileApi.get();
    if (r.data) brandProfile = r.data;
  });

  // v24-4: build a context string from the brand profile to prepend to
  // AI requests. This gives the AI the brand's voice, audience, pillars,
  // keywords, and avoid-topics so generated content matches the brand.
  function brandContext(): string {
    if (!brandProfile) return "";
    const parts: string[] = [];
    if (brandProfile.brand_name) parts.push(`Brand: ${brandProfile.brand_name}`);
    if (brandProfile.description) parts.push(`Mission: ${brandProfile.description}`);
    if (brandProfile.tone_of_voice) parts.push(`Tone: ${brandProfile.tone_of_voice}`);
    if (brandProfile.audience) parts.push(`Audience: ${brandProfile.audience}`);
    if (Array.isArray(brandProfile.content_pillars) && brandProfile.content_pillars.length > 0) {
      parts.push(`Content pillars: ${brandProfile.content_pillars.map(p => p.title).join(', ')}`);
    }
    if (Array.isArray(brandProfile.keywords) && brandProfile.keywords.length > 0) {
      parts.push(`Keywords: ${brandProfile.keywords.join(', ')}`);
    }
    if (Array.isArray(brandProfile.avoid_topics) && brandProfile.avoid_topics.length > 0) {
      parts.push(`Avoid: ${brandProfile.avoid_topics.join(', ')}`);
    }
    if (parts.length === 0) return "";
    return `\n\n[Brand context: ${parts.join(' | ')}]`;
  }

  async function handleGenerate() {
    if (aiLoading) return;
    aiLoading = true;
    aiError = null;
    aiResult = null;
    try {
      let result = "";
      const ctx = brandContext();
      switch (selectedTask) {
        case "generate":
          if (!topic.trim()) { aiError = "Please enter a topic"; aiLoading = false; return; }
          result = await ai.generatePost(topic + ctx, tone, length);
          break;
        case "improve":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.improveWriting(content + ctx);
          break;
        case "hashtags":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.suggestHashtags(content);
          break;
        case "tone":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.changeTone(content + ctx, tone);
          break;
        case "summarize":
          if (!content.trim()) { aiError = "Please write some content first"; aiLoading = false; return; }
          result = await ai.summarize(content);
          break;
      }
      aiResult = result;
    } catch (e: unknown) {
      aiError = (e instanceof Error ? e.message : String(e)) || "AI request failed. Check that LLM-Proxy is running on port 4488.";
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

<div class="bg-surface border border-line rounded-xl p-4 space-y-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold flex items-center gap-2">
      <span class="text-brand-400">✨</span>
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
            ? 'bg-brand-600/20 text-brand-400 border-brand-500/30'
            : 'text-muted border-line hover:text-white hover:border-line-hover'}"
      >
        {task === "generate" ? "Generate" : task === "improve" ? "Improve" : task === "hashtags" ? "Hashtags" : task === "tone" ? "Tone" : "Summarize"}
      </button>
    {/each}
  </div>

  <!-- Conditional inputs -->
  <div class="space-y-3">
    {#if selectedTask === "generate"}
      <div>
        <label for="ai-topic" class="text-xs text-muted block mb-1">Topic</label>
        <input
          id="ai-topic"
          type="text"
          bind:value={topic}
          placeholder="e.g. Our new product launch..."
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-brand-500 outline-none"
        />
      </div>
      <div class="flex gap-3">
        <div class="flex-1">
          <label for="ai-tone" class="text-xs text-muted block mb-1">Tone</label>
          <select
            id="ai-tone"
            bind:value={tone}
            class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-brand-500 outline-none"
          >
            {#each tones as t (t)}
              <option value={t}>{t}</option>
            {/each}
          </select>
        </div>
        <div class="flex-1">
          <label for="ai-length" class="text-xs text-muted block mb-1">Length</label>
          <select
            id="ai-length"
            bind:value={length}
            class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-brand-500 outline-none"
          >
            {#each lengths as l (l)}
              <option value={l}>{l}</option>
            {/each}
          </select>
        </div>
      </div>
    {:else if selectedTask === "tone"}
      <div>
        <label for="ai-target-tone" class="text-xs text-muted block mb-1">Target Tone</label>
        <select
          id="ai-target-tone"
          bind:value={tone}
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-brand-500 outline-none"
        >
          {#each tones as t (t)}
            <option value={t}>{t}</option>
          {/each}
        </select>
      </div>
    {/if}

    {#if selectedTask !== "generate"}
      <p class="text-xs text-muted">Using current post content as input.</p>
    {/if}
  </div>

  <!-- Generate button -->
  <button
    onclick={handleGenerate}
    disabled={aiLoading}
    class="w-full px-3 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 rounded-lg text-sm transition-colors flex items-center justify-center gap-2"
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
    <div class="bg-error/10 border border-error/30 text-error text-sm rounded-lg p-3">{aiError}</div>
  {/if}

  <!-- Result -->
  {#if aiResult}
    <div class="space-y-2">
      <div class="bg-background-input border border-line rounded-lg p-3 text-sm text-content-secondary whitespace-pre-wrap max-h-48 overflow-y-auto">
        {aiResult}
      </div>
      <div class="flex gap-2">
        <button
          onclick={handleInsert}
          class="px-3 py-1.5 bg-brand-600 hover:bg-brand-500 rounded-lg text-xs transition-colors"
        >
          Insert
        </button>
        <button
          onclick={() => { aiResult = null; }}
          class="px-3 py-1.5 text-xs text-muted hover:text-white border border-line rounded-lg transition-colors"
        >
          Discard
        </button>
      </div>
    </div>
  {/if}
</div>
