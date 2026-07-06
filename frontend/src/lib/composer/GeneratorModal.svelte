<script lang="ts">
  // GeneratorModal — AI bulk post generator (Phase 9, v19).
  //
  // Lets the user enter a topic + format + tone, generates multiple posts
  // via the backend LLM, and pipes each result into the ComposerModal for
  // review/scheduling.
  //
  // Inspired by postiz-app's GeneratorComponent (streaming NDJSON with
  // progress stages). We use a simpler single-request approach (no
  // streaming) since our LLM call is fast enough and streaming adds
  // complexity. The UX is similar: prompt form → loading → results →
  // "Use this" button per post.

  import { ai } from '$lib/api/ai';
  import { composer } from '$lib/stores/composer.svelte';
  import { toast } from '$lib/stores/toast';

  let { close } = $props<{ close: (confirmed?: boolean) => void }>();

  let topic = $state('');
  let format = $state<'one_short' | 'one_long' | 'thread_short' | 'thread_long'>('one_short');
  let tone = $state<'personal' | 'company'>('personal');
  let count = $state(3);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let results = $state<{ posts: string[]; suggested_dates: string[] } | null>(null);
  let abortController: AbortController | null = null;

  async function generate() {
    if (!topic.trim()) {
      error = 'Please enter a topic';
      return;
    }
    loading = true;
    error = null;
    results = null;
    abortController = new AbortController();
    try {
      const r = await ai.generateBulk(topic.trim(), format, tone, count, abortController.signal);
      results = r;
    } catch (e) {
      if (e instanceof Error && e.name === 'AbortError') return;
      error = e instanceof Error ? e.message : 'Generation failed';
    } finally {
      loading = false;
    }
  }

  function usePost(post: string, date?: string) {
    // Open the composer with the generated content prefilled.
    // Pass the suggested date as a YYYY-MM-DD string if available.
    const dateStr = date ? date.split('T')[0] : undefined;
    composer.openCreate(dateStr, undefined, post);
    close();
  }

  function useAll() {
    if (!results) return;
    // Open the composer with the first post; the user can schedule the
    // rest manually. (A "schedule all" flow would require batch creation
    // which is a future enhancement.)
    if (results.posts.length > 0) {
      const firstDate = results.suggested_dates[0]?.split('T')[0];
      composer.openCreate(firstDate, undefined, results.posts[0]);
      toast(`${results.posts.length} posts generated. First one loaded into composer — schedule the rest from the calendar.`, 'success');
    }
    close();
  }

  function cancel() {
    abortController?.abort();
    loading = false;
  }

  // Format the suggested date for display.
  function formatDate(iso: string): string {
    const d = new Date(iso);
    return d.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });
  }
</script>

<div class="space-y-5">
  {#if !results && !loading}
    <!-- Prompt form -->
    <div class="space-y-4">
      <div>
        <label class="text-sm text-muted block mb-1">Topic</label>
        <textarea
          bind:value={topic}
          placeholder="e.g., 'Tips for indie hackers launching their first product' or 'The future of AI in content creation'"
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none resize-none"
          rows="3"
        ></textarea>
      </div>

      <div class="grid grid-cols-2 gap-3">
        <div>
          <label class="text-xs text-muted block mb-1">Format</label>
          <select bind:value={format} class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none">
            <option value="one_short">Single short post</option>
            <option value="one_long">Single long post</option>
            <option value="thread_short">Short thread (3 tweets)</option>
            <option value="thread_long">Long thread (5 tweets)</option>
          </select>
        </div>
        <div>
          <label class="text-xs text-muted block mb-1">Tone</label>
          <select bind:value={tone} class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none">
            <option value="personal">Personal</option>
            <option value="company">Company</option>
          </select>
        </div>
      </div>

      <div>
        <label class="text-xs text-muted block mb-1">Number of posts: {count}</label>
        <input type="range" min="1" max="5" bind:value={count} class="w-full" />
      </div>

      {#if error}
        <div class="text-sm text-red-400 bg-red-500/10 border border-red-500/30 rounded-lg p-2">{error}</div>
      {/if}

      <button
        onclick={generate}
        disabled={!topic.trim()}
        class="w-full px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg text-sm font-medium transition-colors"
      >✨ Generate {count} Post{count > 1 ? 's' : ''}</button>
    </div>
  {:else if loading}
    <!-- Loading state -->
    <div class="text-center py-12 space-y-3">
      <div class="inline-block animate-spin text-3xl">✨</div>
      <div class="text-sm text-muted">Generating {count} {format.replace(/_/g, ' ')} post{count > 1 ? 's' : ''} about "{topic}"...</div>
      <div class="text-xs text-muted-dark">This takes 10-30 seconds depending on the LLM.</div>
      <button onclick={cancel} class="text-xs text-muted hover:text-white">Cancel</button>
    </div>
  {:else if results}
    <!-- Results -->
    <div class="space-y-4">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-semibold">{results.posts.length} post{results.posts.length > 1 ? 's' : ''} generated</h3>
        <button onclick={() => { results = null; }} class="text-xs text-muted hover:text-white">← Start over</button>
      </div>

      {#each results.posts as post, i (i)}
        <div class="bg-surface-hover border border-line rounded-lg p-3 space-y-2">
          <div class="flex items-center justify-between text-xs text-muted">
            <span>Post {i + 1}</span>
            {#if results.suggested_dates[i]}
              <span>📅 {formatDate(results.suggested_dates[i])}</span>
            {/if}
          </div>
          <div class="text-sm text-content whitespace-pre-wrap break-words">{post}</div>
          <div class="flex gap-2">
            <button
              onclick={() => usePost(post, results.suggested_dates[i])}
              class="px-3 py-1 text-xs bg-indigo-600 hover:bg-indigo-500 text-white rounded transition-colors"
            >Use this</button>
          </div>
        </div>
      {/each}

      <button
        onclick={useAll}
        class="w-full px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm font-medium transition-colors"
      >Use first post → schedule rest manually</button>
    </div>
  {/if}
</div>
