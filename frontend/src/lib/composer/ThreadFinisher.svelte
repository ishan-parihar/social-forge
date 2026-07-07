<script lang="ts">
  let { content = "", onCreateThread, submitting = false }: {
    content?: string;
    onCreateThread?: (parts: string[]) => void;
    submitting?: boolean;
  } = $props();

  let expanded = $state(false);
  let maxTweets = 25;

  function splitIntoThread(text: string, maxLen = 280): string[] {
    const paragraphs = text.split(/\n\s*\n/).filter(p => p.trim());
    const tweets: string[] = [];
    for (const para of paragraphs) {
      if (para.length <= maxLen) {
        tweets.push(para.trim());
      } else {
        let remaining = para.trim();
        while (remaining.length > maxLen) {
          let splitAt = remaining.lastIndexOf(' ', maxLen);
          if (splitAt === -1) splitAt = maxLen;
          tweets.push(remaining.slice(0, splitAt).trim());
          remaining = remaining.slice(splitAt).trim();
        }
        if (remaining) tweets.push(remaining);
      }
    }
    return tweets;
  }

  let allTweets = $derived(expanded ? splitIntoThread(content) : []);
  let tweets = $derived(allTweets.slice(0, maxTweets));
  let tweetCount = $derived(allTweets.length);
  let exceedsMax = $derived(tweetCount > maxTweets);

  function toggle() {
    expanded = !expanded;
  }

  function handlePostThread() {
    onCreateThread?.(tweets);
  }
</script>

<div class="bg-surface border border-line rounded-xl p-4 space-y-3">
  <button
    onclick={toggle}
    class="flex items-center justify-between w-full text-left"
  >
    <h3 class="text-sm font-semibold flex items-center gap-2">
      Thread Finisher
    </h3>
    <span class="text-xs text-muted">{expanded ? '▾' : '▸'}</span>
  </button>

  {#if expanded}
    {#if tweetCount === 0}
      <p class="text-xs text-muted">Add content with paragraph breaks to create a thread.</p>
    {:else if exceedsMax}
      <p class="text-xs text-warning">Thread exceeds max of {maxTweets} tweets ({tweetCount} parts). Only first {maxTweets} will be posted.</p>
    {/if}

    {#if tweetCount > 0}
      <div class="space-y-2 max-h-64 overflow-y-auto pr-1">
        {#each tweets as tweet, i}
          <div class="border border-line rounded-lg p-3 text-sm space-y-1">
            <div class="flex items-center justify-between">
              <span class="text-xs font-medium text-brand-400">Tweet {i + 1}</span>
              <span class="text-xs {tweet.length > 280 ? 'text-error' : 'text-muted'}">
                {tweet.length} / 280
              </span>
            </div>
            <p class="text-content-secondary text-sm leading-relaxed">{tweet}</p>
          </div>
        {/each}
      </div>

      <button
        onclick={handlePostThread}
        disabled={submitting}
        class="w-full px-3 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 rounded-lg text-sm transition-colors"
      >
        {submitting ? "Posting..." : `Post Thread (${tweetCount} tweets)`}
      </button>
    {/if}
  {/if}
</div>