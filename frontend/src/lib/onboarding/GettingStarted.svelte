<script lang="ts">
  // Getting Started checklist widget (U-7).
  //
  // Shows a persistent 4-item checklist on the dashboard until the user
  // completes all items (or dismisses it). Progress is tracked in
  // localStorage so it survives page reloads and browser restarts.
  //
  // The checklist auto-detects completion of:
  //   1. Connect a channel  → ≥1 non-disabled integration exists
  //   2. Create a post      → ≥1 post exists (any state)
  //   3. Schedule it        → ≥1 post in 'queued' state
  //   4. View your analytics → user has visited /analytics (tracked in localStorage)
  //
  // The widget is dismissable — once dismissed, it stays hidden even if
  // not all items are complete. The user can re-show it from the
  // settings page (future enhancement).

  import { onMount } from 'svelte';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { postsApi } from '$lib/api/posts';
  import { goto } from '$app/navigation';

  const DISMISS_KEY = 'social-forge-getting-started-dismissed';
  const ANALYTICS_VISITED_KEY = 'social-forge-analytics-visited';

  let dismissed = $state(false);
  let integrations = $state<Integration[]>([]);
  let hasPosts = $state(false);
  let hasQueuedPost = $state(false);
  let analyticsVisited = $state(false);
  let loading = $state(true);

  let checklist = $derived([
    {
      label: 'Connect a channel',
      done: integrations.length > 0,
      href: '/channels',
    },
    {
      label: 'Create a post',
      done: hasPosts,
      href: '/posts/new',
    },
    {
      label: 'Schedule it on the calendar',
      done: hasQueuedPost,
      href: '/calendar',
    },
    {
      label: 'View your analytics',
      done: analyticsVisited,
      href: '/analytics',
    },
  ]);

  let completedCount = $derived(checklist.filter(c => c.done).length);
  let allDone = $derived(completedCount === checklist.length);

  onMount(async () => {
    dismissed = localStorage.getItem(DISMISS_KEY) === 'true';
    analyticsVisited = localStorage.getItem(ANALYTICS_VISITED_KEY) === 'true';

    const [integRes, postsRes] = await Promise.all([
      integrationsApi.list(),
      postsApi.list({ limit: 1 }),
    ]);

    if (integRes.data) {
      integrations = integRes.data.integrations.filter(i => !i.disabled);
    }
    if (postsRes.data) {
      hasPosts = postsRes.data.total > 0;
      // Check for any queued post — need a larger sample to be reliable
      if (postsRes.data.total > 0) {
        const allPosts = await postsApi.list({ limit: 100 });
        if (allPosts.data) {
          hasQueuedPost = allPosts.data.posts.some(p => p.state === 'queued');
        }
      }
    }
    loading = false;

    // If all items are complete, auto-dismiss after a short delay so the
    // user sees the satisfying "all done" state before it disappears.
    if (allDone && !dismissed) {
      setTimeout(() => dismiss(), 3000);
    }
  });

  function dismiss() {
    dismissed = true;
    localStorage.setItem(DISMISS_KEY, 'true');
  }

  function handleClick(href: string) {
    if (href === '/analytics') {
      localStorage.setItem(ANALYTICS_VISITED_KEY, 'true');
    }
    goto(href);
  }
</script>

{#if !loading && !dismissed && !allDone}
  <div class="bg-gradient-to-br from-brand-500/10 to-purple-500/10 border border-brand-500/30 rounded-xl p-5">
    <div class="flex items-start justify-between mb-3">
      <div>
        <h3 class="text-sm font-semibold text-brand-300">Getting Started</h3>
        <p class="text-xs text-muted mt-0.5">{completedCount} of {checklist.length} complete</p>
      </div>
      <button
        onclick={dismiss}
        class="text-muted hover:text-content text-xs"
        aria-label="Dismiss getting started checklist"
      >
        ✕
      </button>
    </div>

    <!-- Progress bar -->
    <div class="h-1.5 bg-background-input rounded-full overflow-hidden mb-4">
      <div
        class="h-full bg-brand-500 rounded-full transition-all duration-500"
        style="width: {(completedCount / checklist.length) * 100}%"
      ></div>
    </div>

    <!-- Checklist items -->
    <div class="space-y-1.5">
      {#each checklist as item, i (item.label)}
        <button
          onclick={() => handleClick(item.href)}
          class="w-full flex items-center gap-3 p-2 -mx-2 rounded-lg hover:bg-surface-hover transition-colors text-left group"
        >
          <div class="w-5 h-5 rounded-full flex items-center justify-center text-xs shrink-0
            {item.done
              ? 'bg-emerald-500/20 text-emerald-400'
              : 'bg-surface-hover text-muted border border-line'}">
            {#if item.done}
              <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                <polyline points="20 6 9 17 4 12"/>
              </svg>
            {:else}
              {i + 1}
            {/if}
          </div>
          <span class="text-sm flex-1 {item.done ? 'text-muted line-through' : 'text-content'}">
            {item.label}
          </span>
          {#if !item.done}
            <span class="text-muted text-xs opacity-0 group-hover:opacity-100 transition-opacity">→</span>
          {/if}
        </button>
      {/each}
    </div>
  </div>
{:else if !loading && !dismissed && allDone}
  <!-- All-done state: brief success banner before auto-dismiss -->
  <div class="bg-emerald-500/10 border border-emerald-500/30 rounded-xl p-4 flex items-center gap-3">
    <span class="text-xl">🎉</span>
    <div class="flex-1">
      <p class="text-sm text-emerald-300 font-medium">You're all set!</p>
      <p class="text-xs text-muted">You've completed the getting started checklist.</p>
    </div>
  </div>
{/if}
