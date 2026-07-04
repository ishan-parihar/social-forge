<script lang="ts">
  import type { EngagementMetrics } from "$lib/api/feed";

  let { engagement, provider, compact = false }: { engagement: EngagementMetrics; provider: string; compact?: boolean } = $props();

  // Format large numbers (e.g., 1234 -> "1.2k")
  function fmt(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1).replace(/\.0$/, '') + 'M';
    if (n >= 1_000) return (n / 1_000).toFixed(1).replace(/\.0$/, '') + 'k';
    return String(n);
  }

  // Always show all relevant metrics — display 0 values instead of hiding them
  // Uses platform-aware slots to avoid redundant metrics:
  //   - Positive feedback: likes/upvotes/reactions unified in one slot
  //   - Shares: shares/reposts/quotes unified in one slot
  //   - Comments: comments/replies unified
  const metrics = $derived.by(() => {
    const items: { icon: string; label: string; value: number }[] = [];

    // Positive feedback slot: likes + upvotes + reactions unified
    const likesVal = Math.max(engagement.likes ?? 0, engagement.upvotes ?? 0);
    let likesLabel = 'Likes';
    let likesIcon = 'heart';
    if (provider === 'reddit' || provider === 'lemmy') {
      likesLabel = 'Upvotes';
      likesIcon = 'upvote';
    } else if (engagement.reactions) {
      likesLabel = 'Reactions';
      likesIcon = 'heart';
    }
    items.push({ icon: likesIcon, label: likesLabel, value: likesVal });

    // Comments/replies unified
    const commentsVal = Math.max(engagement.comments ?? 0, engagement.replies ?? 0);
    let commentsLabel = 'Comments';
    if (provider === 'x' || provider === 'bluesky' || provider === 'threads') {
      commentsLabel = 'Replies';
    }
    items.push({ icon: 'comment', label: commentsLabel, value: commentsVal });

    // Shares/reposts/quotes unified
    const sharesVal = Math.max(engagement.shares ?? 0, engagement.reposts ?? 0, engagement.quotes ?? 0);
    let sharesLabel = 'Shares';
    if (provider === 'x' || provider === 'bluesky' || provider === 'threads') {
      sharesLabel = 'Reposts';
    }
    items.push({ icon: 'share', label: sharesLabel, value: sharesVal });

    // Views (impressions)
    items.push({ icon: 'eye', label: 'Views', value: engagement.views ?? 0 });

    // Saves/bookmarks
    items.push({ icon: 'bookmark', label: 'Saves', value: engagement.saves ?? 0 });

    // Awards (Reddit)
    if (engagement.awards && engagement.awards > 0) {
      items.push({ icon: 'award', label: 'Awards', value: engagement.awards });
    }

    return items;
  });

  // Reaction breakdown (Facebook) — always show if reactions object exists
  const reactions = $derived.by(() => {
    if (!engagement.reactions) return null;
    const r = engagement.reactions as Record<string, number>;
    const total = Object.values(r).reduce((a, b) => a + b, 0);
    return { total, breakdown: r };
  });

  const reactionEmojis: Record<string, string> = {
    like: '👍',
    love: '❤️',
    haha: '😂',
    wow: '😮',
    sad: '😢',
    angry: '😡',
    care: '💚',
  };
</script>

<div class="flex flex-wrap items-center gap-2 {compact ? '' : 'mt-3 pt-3 border-t border-[#1a2035]'}">
    <!-- Core metrics (show only top 3 in compact mode) -->
    {#each (compact ? metrics.slice(0, 3) : metrics) as m}
      <span
        class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium
          bg-[#0d121e] text-[#9ca3af] border border-[#1a2035]
          group-hover:border-[#222a45] transition-colors"
        title={m.label}
      >
        <!-- Icon -->
        {#if m.icon === 'heart'}
          <svg class="w-3 h-3 text-red-400" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 3.5C6.5 1.5 3.5 1.5 2 3.5s-1 5 2 8l4 2.5 4-2.5c3-3 3.5-6 2-8s-4.5-2-6 0z"/>
          </svg>
        {:else if m.icon === 'comment'}
          <svg class="w-3 h-3 text-blue-400" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M14 8a6 6 0 01-9.3 5L2 14l1-2.7A6 6 0 1114 8z" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else if m.icon === 'share'}
          <svg class="w-3 h-3 text-emerald-400" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M6 8L2 5l4-3M14 5l-4 3 4 3" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M2 5h8a4 4 0 014 4v2" stroke-linecap="round"/>
          </svg>
        {:else if m.icon === 'eye'}
          <svg class="w-3 h-3 text-violet-400" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M8 3C4.5 3 1.5 8 1.5 8s3 5 6.5 5 6.5-5 6.5-5-3-5-6.5-5z" stroke-linecap="round"/>
            <circle cx="8" cy="8" r="2" stroke-linecap="round"/>
          </svg>
        {:else if m.icon === 'bookmark'}
          <svg class="w-3 h-3 text-amber-400" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M4 2v12l4-3 4 3V2a1 1 0 00-1-1H5a1 1 0 00-1 1z" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        {:else if m.icon === 'quote'}
          <svg class="w-3 h-3 text-cyan-400" viewBox="0 0 16 16" fill="currentColor">
            <path d="M3.5 4.5A1.5 1.5 0 005 3h2v2c0 2.5-1 4-2.5 5l-1.4-.8C4.7 8.2 5 7 5 6H3.5V4.5zM9.5 4.5A1.5 1.5 0 0011 3h2v2c0 2.5-1 4-2.5 5l-1.4-.8C10.7 8.2 11 7 11 6H9.5V4.5z"/>
          </svg>
        {:else if m.icon === 'upvote'}
          <svg class="w-3 h-3 text-orange-400" viewBox="0 0 16 16" fill="currentColor">
            <path d="M8 1l6 6h-4v6H6V7H2l6-6z"/>
          </svg>
        {/if}
        <span>{fmt(m.value)}</span>
      </span>
    {/each}

    <!-- Reaction breakdown (Facebook) -->
    {#if reactions}
      <span
        class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-medium
          bg-[#0d121e] text-[#9ca3af] border border-[#1a2035]
          group-hover:border-[#222a45] transition-colors"
        title="Reactions"
      >
        {#each Object.entries(reactions.breakdown) as [type, count]}
          <span class="inline-flex items-center gap-0.5">
            <span>{reactionEmojis[type] ?? '👍'}</span>
            <span class="text-[10px]">{count}</span>
          </span>
        {/each}
        <span class="text-[#5a6070] ml-0.5">{fmt(reactions.total)}</span>
      </span>
    {/if}
  </div>

