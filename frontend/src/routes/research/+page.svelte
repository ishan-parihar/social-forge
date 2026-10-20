<script lang="ts">
  // Research — "what is working in my niche, and what should I post next".
  //
  // Four read-only sections, all derived from data Social Forge already
  // stores: own posts ranked by engagement, recent competitor posts ranked
  // by engagement, when the founder's own posts performed best, and topics
  // the niche uses that they have not. Plus what their connected feeds are
  // publishing.
  //
  // There are no writes and no network calls anywhere on this page. Each
  // section owns its own EmptyState that names the missing source and how
  // to fill it (Settings → Channels to connect, Feed to import) rather than
  // blanking the whole dashboard the way a single page-level check does.
    import Icon from "$lib/ui/Icon.svelte";
  import Skeleton from "$lib/ui/Skeleton.svelte";
  import EmptyState from "$lib/ui/EmptyState.svelte";
  import { researchApi, type ResearchResponse } from "$lib/api/research";
  import { providerLabel } from "$lib/providers";
  import { timezone } from "$lib/stores/timezone.svelte";

  let data = $state<ResearchResponse | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function fetchData(signal?: AbortSignal) {
    loading = true;
    error = null;
    const r = await researchApi.get(undefined, signal);
    if (signal?.aborted) return;
    if (r.error) {
      error = r.error;
      data = null;
    } else if (r.data) {
      data = r.data;
    }
    loading = false;
  }

  $effect(() => {
    const controller = new AbortController();
    fetchData(controller.signal);
    return () => controller.abort();
  });

  function num(n: number): string {
    return n.toLocaleString();
  }

  /**
   * A link URL that is safe to put in an `href`.
   *
   * The URL comes from an RSS feed the user subscribed to, so it is not
   * under the app's control. `javascript:` in a feed's `<link>` would
   * become a clickable script URL on this page. Only `http` and `https`
   * are allowed; anything else renders as plain text.
   */
  function safeUrl(url: string | null | undefined): string | null {
    if (!url) return null;
    try {
      const parsed = new URL(url, window.location.origin);
      if (parsed.protocol === "http:" || parsed.protocol === "https:") {
        return parsed.toString();
      }
      return null;
    } catch {
      return null;
    }
  }

  /**
   * The best engagement per post across the slots, used to scale the
   * bars. Without it every bar was drawn at 100% width, so the chart
   * compared nothing.
   */
  let maxSlotEngagement = $derived(
    data?.timing?.best_slots?.length
      ? Math.max(...data.timing.best_slots.map((s) => s.engagement_per_post))
      : 0,
  );

  /**
   * A posting slot, read in the APP's timezone.
   *
   * The server groups the posts by their UTC hour and labels the slot
   * "Wednesday 14:00-15:00 UTC". The founder schedules from the app's
   * timezone, so a table labelled in UTC is a table they have to
   * convert in their head before they can use it - and the conversion
   * is where the mistake happens.
   *
   * So the LABEL is built here, in the app's timezone, from the slot's
   * UTC hour. A UTC hour is an instant on the day the slot was
   * measured, so it is converted against a reference date in the app's
   * zone; the weekday follows from the same conversion, so the two can
   * never disagree.
   */
  function slotLabel(slot: { weekday: string; hour: number }): { weekday: string; hour: string } {
    try {
      // The slot is ONE UTC hour. The founder schedules in the app's
      // timezone, so the slot has to be rendered in that zone.
      //
      // The conversion is ONE instant, and the weekday and the hour
      // both come from it. They used to be derived separately - the
      // weekday from a reference instant and the hour from another -
      // so they could disagree about which day the slot falls on.
      //
      // The reference day is a WEDNESDAY (2026-01-07). The weekday
      // name is the SERVER's grouping key, computed in UTC, so it
      // must stay the UTC weekday - what the server counted is what
      // the table shows. Only the CLOCK is converted, because a
      // posting time is a wall-clock time in the founder's zone.
      const anchor = Date.UTC(2026, 0, 7, slot.hour, 0, 0);
      const d = new Date(anchor);
      const hour = d.toLocaleTimeString("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
        timeZone: timezone.value,
      });
      // The slot is an HOUR, so it reads as a range: 15:00-16:00. It
      // used to print the same hour twice ("15:30-15:30"), which is
      // a range of zero.
      const end = new Date(anchor + 3_600_000);
      const endHour = end.toLocaleTimeString("en-US", {
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
        timeZone: timezone.value,
      });
      return { weekday: slot.weekday, hour: `${hour}-${endHour}` };
    } catch {
      // The zone could not be applied. The slot is still an hour, so
      // the range is still an hour - the fallback prints the UTC hour
      // the server grouped on, and the caption names the zone.
      const start = String(slot.hour).padStart(2, "0");
      const end = String((slot.hour + 1) % 24).padStart(2, "0");
      return {
        weekday: slot.weekday,
        hour: `${start}:00-${end}:00`,
      };
    }
  }

  function formatDate(iso: string | null): string {
    if (!iso) return "—";
    try {
      return new Date(iso).toLocaleDateString("en-US", {
        month: "short",
        day: "numeric",
        year: "numeric",
        timeZone: timezone.value,
      });
    } catch {
      return new Date(iso).toDateString();
    }
  }

  function clip(text: string, n = 140): string {
    return text.length > n ? `${text.slice(0, n).trimEnd()}…` : text;
  }
</script>

<div class="page-enter space-y-6">
  <!-- Header -->
  <div class="flex items-center justify-between gap-3 flex-wrap">
    <div>
      <h1 class="text-xl font-bold text-content">Research</h1>
      <p class="text-sm text-muted mt-1">
        What is working for you, what is working for the accounts you imported, and what you have not covered yet.
      </p>
    </div>
    {#if data}
      <span class="text-xs text-muted">Last {data.window_days} days</span>
    {/if}
  </div>

  {#if error}
    <div class="bg-error/10 border border-error/30 rounded-lg p-4" role="alert">
      <p class="text-error text-sm">{error}</p>
      <button onclick={() => fetchData()} class="mt-2 text-sm text-accent hover:text-accent-strong min-h-11">Retry</button>
    </div>
  {:else if loading && !data}
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <Skeleton variant="card" rows={2} />
      <Skeleton variant="card" rows={2} />
    </div>
  {:else if data}
    <!-- (a) Top-performing own posts -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-medium text-content">Your top posts</h2>
        <a href="/analytics" class="text-xs whitespace-nowrap text-accent hover:text-accent-strong min-h-11 inline-flex items-center">Full analytics</a>
      </div>
      {#if data.top_own_posts.length === 0}
        <EmptyState
          icon="post"
          title="No published posts with engagement yet"
          description="Publish a post and let its metrics sync, then it shows up here ranked by likes, comments and shares."
        />
      {:else}
        <ul>
          {#each data.top_own_posts as post, i (post.post_id)}
            <li class="flex items-start gap-3 py-2.5 border-b border-line last:border-0">
              <span class="text-xs text-faint w-4 shrink-0 mt-0.5 tabular-nums">{i + 1}</span>
              <div class="flex-1 min-w-0">
                <p class="text-sm text-content-secondary truncate">{clip(post.content)}</p>
                <div class="flex gap-3 mt-1 text-[10px] text-muted">
                  <span class="truncate">{providerLabel(post.provider)}</span>
                  <span>{formatDate(post.published_at)}</span>
                </div>
              </div>
              <span class="text-xs font-medium text-content shrink-0 flex items-center gap-1">
                <Icon name="heart" class="w-3 h-3 text-viz-like" />
                {num(post.engagement)}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- (b) Recent external/competitor posts -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-medium text-content">Competitor posts, ranked by engagement</h2>
        <a href="/feed" class="text-xs whitespace-nowrap text-accent hover:text-accent-strong min-h-11 inline-flex items-center">Open feed</a>
      </div>
      {#if data.top_external_posts.length === 0}
        <EmptyState
          icon="feed"
          title="No imported posts with engagement"
          description="Import competitor posts from Settings → Channels, then open the Feed and refresh so their metrics sync."
          actionLabel="Open feed"
          onaction={() => { window.location.href = "/feed"; }}
        />
      {:else}
        <ul>
          {#each data.top_external_posts as post, i (post.post_id)}
            <li class="flex items-start gap-3 py-2.5 border-b border-line last:border-0">
              <span class="text-xs text-faint w-4 shrink-0 mt-0.5 tabular-nums">{i + 1}</span>
              <div class="flex-1 min-w-0">
                <p class="text-sm text-content-secondary truncate">{clip(post.text)}</p>
                <div class="flex gap-3 mt-1 text-[10px] text-muted">
                  <span class="truncate">{providerLabel(post.provider)}</span>
                  {#if post.author_handle}
                    <span class="truncate">@{post.author_handle}</span>
                  {/if}
                  <span>{formatDate(post.published_at)}</span>
                </div>
              </div>
              <span class="text-xs font-medium text-content shrink-0 flex items-center gap-1">
                <Icon name="heart" class="w-3 h-3 text-viz-like" />
                {num(post.engagement)}
              </span>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- (c) Posting-time insight -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <div class="flex items-center justify-between mb-1">
        <!--
          The timing data is computed from the IMPORTED FEED (the
          competitor posts the feed refresher keeps in sync), not from the
          founder's own posts. It used to be headed "When your posts
          perform best", which told the founder these were their own
          best slots - they were the niche's, and a founder acting on them
          would be copying a schedule that was measured on someone else's
          audience.
        -->
        <h2 class="text-sm font-medium text-content">When posts in this niche perform best</h2>
        {#if data.timing?.low_confidence}
          <span class="text-xs text-warning">Small sample</span>
        {/if}
      </div>
      {#if !data.timing || data.timing.best_slots.length === 0}
        <EmptyState
          icon="clock"
          title="No posting-time signal yet"
          description="Once the imported feed has posts with engagement metrics, the niche's best weekday and hour show up here."
        />
      {:else}
        <p class="text-xs text-muted mb-4">
          Based on {data.timing.posts_analyzed}
          {data.timing.posts_analyzed === 1 ? "post" : "posts"} from your imported feed
          {#if data.timing.low_confidence}
            — too few to call a pattern, treat as a hint
          {/if}
        </p>
        <ul class="space-y-2.5">
          {#each data.timing.best_slots as slot (slot.label)}
            <li class="flex items-center gap-3">
              <span class="text-xs text-muted w-32 shrink-0 truncate">{slotLabel(slot).weekday}</span>
              <span class="text-xs text-faint w-20 shrink-0 tabular-nums">{slotLabel(slot).hour}</span>
              <div class="flex-1 bg-background-input rounded-full h-6 overflow-hidden">
                <!--
                  Solid fill, not /60, for the same contrast reason the
                  analytics channel-mix bars use: a 60%-alpha brand over a
                  light track composites too pale for the white count label.

                  The width is PROPORTIONAL to the best slot. It used to be
                  `100%` whenever the value was positive, so every bar was
                  full and the chart compared nothing - three slots with
                  8470, 5410 and 3720 drew three identical bars.
                -->
                <div
                  class="h-full bg-accent-fill rounded-full flex items-center justify-end px-2"
                  style="width: {slot.engagement_per_post > 0
                    ? Math.max(6, Math.round((slot.engagement_per_post / maxSlotEngagement) * 100))
                    : 0}%"
                >
                  <span class="text-[10px] font-medium text-accent-fg">{slot.engagement_per_post.toFixed(1)}</span>
                </div>
              </div>
              <span class="text-[10px] text-muted w-24 shrink-0 text-right">
                {slot.posts} {slot.posts === 1 ? "post" : "posts"}
              </span>
            </li>
          {/each}
        </ul>
        <!--
          The times are in the APP's timezone, not UTC. The server
          groups the posts by their UTC hour - that is the grouping key
          - and the clock is then rendered in the timezone the founder
          schedules in, so the table shows the hour they would post at.

          The caption says which zone, because a table of times with
          no zone is a table nobody can act on.
        -->
        <p class="text-xs text-muted mt-3">
          Average engagement per post in each slot, in your timezone ({timezone.value}).
        </p>
      {/if}
    </div>

    <!-- (d) Content gaps -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-medium text-content">Topics you have not covered</h2>
      </div>
      {#if data.content_gaps.length === 0}
        <EmptyState
          icon="search"
          title="No content gaps found"
          description="Nothing in your imported competitor posts stands out as a topic you have not written about yet — import more competitor posts to widen the comparison."
        />
      {:else}
        <p class="text-xs text-muted mb-3">
          Words your imported competitor posts use that yours do not.
        </p>
        <ul class="space-y-3">
          {#each data.content_gaps as gap (gap.word)}
            <li>
              <div class="flex items-baseline gap-2">
                <span class="text-sm font-medium text-content">{gap.word}</span>
                <span class="text-[10px] text-muted">
                  in {gap.external_posts} {gap.external_posts === 1 ? "competitor post" : "competitor posts"}
                </span>
              </div>
              {#if gap.example}
                <p class="text-xs text-muted mt-0.5 italic">"{clip(gap.example, 110)}"</p>
              {:else}
                <p class="text-xs text-muted mt-0.5">No example text available.</p>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <!-- Connected feed items -->
    <div class="bg-surface border border-line rounded-xl p-5">
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-sm font-medium text-content">Your feeds are publishing</h2>
        <a href="/settings/rss" class="text-xs text-accent hover:text-accent-strong min-h-11 inline-flex items-center">Manage feeds</a>
      </div>
      {#if data.connected_feed_items.length === 0}
        <EmptyState
          icon="rss"
          title="No feed items yet"
          description="Connect an RSS feed under Settings → Automation to see what your sources are publishing here."
          actionLabel="Add a feed"
          onaction={() => { window.location.href = "/settings/rss"; }}
        />
      {:else}
        <ul class="space-y-2">
          {#each data.connected_feed_items as item (item.id)}
            <li class="flex items-start gap-3 py-1.5">
              <span class="text-xs text-muted w-24 shrink-0 pt-0.5">{formatDate(item.published_at)}</span>
              {#if safeUrl(item.url)}
                <a
                  href={safeUrl(item.url)}
                  target="_blank"
                  rel="noopener noreferrer"
                  class="text-sm text-content-secondary flex-1 min-w-0 truncate hover:text-content min-h-11 inline-flex items-center"
                >{clip(item.title, 90)}</a>
              {:else}
                <span class="text-sm text-content-secondary flex-1 min-w-0 truncate">{clip(item.title, 90)}</span>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  {/if}
</div>
