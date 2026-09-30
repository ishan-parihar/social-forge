<script lang="ts">
  // AiAssistant — in-composer AI writing (v25 F2: explicit request states).
  //
  // F2's change here is honesty about what the user is waiting for. The old
  // panel had two booleans (`aiLoading` / `aiResult`), so a failed request and
  // an abandoned one looked identical, a retry looked identical to the first
  // attempt, and there was no way to cancel a call that was going to take 30
  // seconds. It is now a state machine: idle → working → done | error, with an
  // abort handle and a stage readout.
  //
  // On the stages: /api/ai/* returns a single JSON body. There is no streaming
  // endpoint on the backend and F2 does not add one (that is backend work, and
  // the brief forbids touching it). So the stage labels and the elapsed timer
  // are a PROGRESS AFFORDANCE driven by a local clock, not a report of tokens
  // arriving. They are labelled as such here because the next person to read
  // this file will otherwise assume there is a stream behind them. If the
  // backend ever grows an NDJSON or SSE response, `runTask` is the only place
  // that needs to change — the states already exist.
  import { ai } from "$lib/api/ai";
  import { profileApi, type BrandProfile } from "$lib/api/profile";
  import { onMount, onDestroy } from "svelte";

  let { content = "", onInsert }: {
    content?: string;
    onInsert?: (text: string) => void;
  } = $props();

  /** The request lifecycle, in one place. Never two booleans again. */
  type Status = "idle" | "working" | "done" | "error";
  type Task = "generate" | "improve" | "hashtags" | "tone" | "summarize";
  let status = $state<Status>("idle");
  let stage = $state(0);
  let elapsedMs = $state(0);
  let aiError = $state<string | null>(null);
  let aiResult = $state<string | null>(null);
  let selectedTask = $state<Task>("generate");
  let topic = $state("");
  let tone = $state("professional");
  let length = $state("medium");

  let abort: AbortController | null = null;
  let ticker: ReturnType<typeof setInterval> | null = null;

  // Stage labels per task. Short enough to read at a glance while they change.
  const STAGES: Record<Task, string[]> = {
    generate: ["Reading brand context", "Drafting the post", "Applying your voice", "Checking channel limits"],
    improve: ["Reading your draft", "Rewriting for clarity", "Tightening the structure"],
    hashtags: ["Reading your draft", "Scanning topic clusters", "Ranking tags"],
    tone: ["Reading your draft", "Shifting register", "Re-checking voice"],
    summarize: ["Reading your draft", "Condensing", "Fitting the character budget"],
  };
  const stages = $derived(STAGES[selectedTask]);
  let stageLabel = $derived(stages[Math.min(stage, stages.length - 1)]);
  let stagePct = $derived(stages.length > 1 ? Math.round(((Math.min(stage, stages.length - 1) + 1) / stages.length) * 100) : 100);

  onDestroy(() => {
    abort?.abort();
    if (ticker) clearInterval(ticker);
  });

  function stopTicking() {
    if (ticker) { clearInterval(ticker); ticker = null; }
  }

  function startTicking() {
    stopTicking();
    elapsedMs = 0;
    // 2.4s per stage: slow enough that a fast model does not strobe through
    // every label, fast enough that a slow one does not look hung.
    ticker = setInterval(() => {
      elapsedMs += 400;
      const next = Math.floor(elapsedMs / 2400);
      if (next !== stage) stage = Math.min(next, stages.length - 1);
    }, 400);
  }

  function reset() {
    status = "idle";
    stage = 0;
    elapsedMs = 0;
    aiError = null;
    aiResult = null;
  }

  function cancel() {
    abort?.abort();
    stopTicking();
    if (status === "working") {
      status = "idle";
      aiError = "Cancelled";
    }
  }

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
    if (status === "working") return;
    const ctx = brandContext();
    // Guard before we claim to be working — a missing input is an idle state
    // with a message, not a failed request.
    if (selectedTask === "generate" && !topic.trim()) { aiError = "Please enter a topic"; return; }
    if (selectedTask !== "generate" && !content.trim()) { aiError = "Please write some content first"; return; }

    reset();
    status = "working";
    startTicking();
    abort = new AbortController();
    try {
      let result = "";
      const sig = abort.signal;
      switch (selectedTask) {
        case "generate":
          result = await ai.generatePost(topic + ctx, tone, length, sig);
          break;
        case "improve":
          result = await ai.improveWriting(content + ctx, sig);
          break;
        case "hashtags":
          result = await ai.suggestHashtags(content, sig);
          break;
        case "tone":
          result = await ai.changeTone(content + ctx, tone, sig);
          break;
        case "summarize":
          result = await ai.summarize(content, sig);
          break;
      }
      aiResult = result;
      status = "done";
    } catch (e: unknown) {
      // An abort is a user action, not a failure — say which it was.
      if (e instanceof Error && e.name === "AbortError") {
        status = "idle";
        aiError = "Cancelled";
      } else {
        status = "error";
        aiError = (e instanceof Error ? e.message : String(e)) || "AI request failed. Check that LLM-Proxy is running on port 4488.";
      }
    } finally {
      stopTicking();
      abort = null;
    }
  }

  function handleInsert() {
    if (aiResult) {
      onInsert?.(aiResult);
      reset();
    }
  }

  function elapsedLabel(): string {
    const s = Math.round(elapsedMs / 1000);
    return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`;
  }
</script>

<div class="bg-surface border border-line rounded-xl p-4 space-y-4">
  <div class="flex items-center justify-between">
    <h3 class="text-sm font-semibold flex items-center gap-2">
      <span class="text-accent">✨</span>
      AI Assistant
    </h3>
  </div>

  <!-- Task selector. Disabled mid-request: switching task while a call is in
       flight would leave the stage readout describing a different job. -->
  <div class="flex flex-wrap gap-2">
    {#each tasks as task}
      <button
        onclick={() => { if (status !== "working") { selectedTask = task; reset(); } }}
        disabled={status === "working"}
        class="px-3 py-1.5 text-xs rounded-lg border transition-colors disabled:opacity-50
          {selectedTask === task
            ? 'bg-accent-soft text-accent border-accent/40'
            : 'text-muted border-line hover:text-content hover:border-line-hover'}"
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
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-accent outline-none"
        />
      </div>
      <div class="flex gap-3">
        <div class="flex-1">
          <label for="ai-tone" class="text-xs text-muted block mb-1">Tone</label>
          <select
            id="ai-tone"
            bind:value={tone}
            class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-accent outline-none"
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
            class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-accent outline-none"
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
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-accent outline-none"
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

  <!-- Generate / cancel. One control per state, so there is never a button
       that looks live and does nothing. -->
  {#if status === "working"}
    <div class="space-y-2">
      <div class="flex items-center justify-between text-xs">
        <span class="text-content-secondary flex items-center gap-2">
          <span class="inline-block w-3.5 h-3.5 border-2 border-line border-t-accent rounded-full animate-spin" aria-hidden="true"></span>
          {stageLabel}
        </span>
        <span class="text-faint font-mono">{elapsedLabel()}</span>
      </div>
      <!-- Progress bar. Width is the stage count, not real completion — the
           label above says which step it is on, and the elapsed clock says how
           long it has been waiting. -->
      <div class="h-1 rounded-full bg-line overflow-hidden" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={stagePct} aria-label="AI request progress">
        <div class="h-full rounded-full bg-accent transition-[width] duration-300" style="width: {stagePct}%"></div>
      </div>
      <div class="skeleton h-16 rounded-lg" aria-hidden="true"></div>
      <button onclick={cancel} class="w-full px-3 py-1.5 text-xs text-muted hover:text-content border border-line rounded-lg transition-colors">
        Cancel
      </button>
    </div>
  {:else}
    <button
      onclick={handleGenerate}
      class="w-full px-3 py-2 bg-accent-fill hover:bg-accent-fill-hover disabled:opacity-50 rounded-lg text-sm text-accent-fg transition-colors flex items-center justify-center gap-2"
    >
      {status === "done" ? "Regenerate" : status === "error" ? "Try again" : "Generate"}
    </button>
  {/if}

  <!-- Error. Cancel is informational, not a failure, so it does not use the
       error treatment. -->
  {#if aiError}
    <div
      class="text-sm rounded-lg p-3
        {aiError === 'Cancelled'
          ? 'bg-surface-hover border border-line text-muted'
          : 'bg-error/10 border border-error/30 text-error'}"
      role={aiError === 'Cancelled' ? 'status' : 'alert'}
    >
      {aiError === 'Cancelled' ? 'Cancelled — nothing was sent to the model.' : aiError}
    </div>
  {/if}

  <!-- Result -->
  {#if aiResult && status === "done"}
    <div class="space-y-2">
      <div class="bg-background-input border border-line rounded-lg p-3 text-sm text-content-secondary whitespace-pre-wrap max-h-48 overflow-y-auto">
        {aiResult}
      </div>
      <div class="flex gap-2">
        <button
          onclick={handleInsert}
          class="px-3 py-1.5 bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg text-xs transition-colors"
        >
          Insert
        </button>
        <button
          onclick={reset}
          class="px-3 py-1.5 text-xs text-muted hover:text-content border border-line rounded-lg transition-colors"
        >
          Discard
        </button>
      </div>
    </div>
  {/if}
</div>
