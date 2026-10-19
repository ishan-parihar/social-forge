<script lang="ts">
  // v25 F6 - the agent activity log.
  //
  // One merged timeline of everything the app did on the operator's
  // behalf, read from three append-only tables the backend already
  // maintained (events_log, automation_logs, publish_attempts) and that no
  // single surface showed. This page is strictly READ-ONLY: no row is
  // writable from here, and no request here is anything but a GET.
  //
  // Pagination is "load more" rather than an infinite scroll: the feed is
  // an operator's audit trail, and appending to a list they may be reading
  // mid-scan moves rows out from under the cursor. The page refetches only
  // on a filter change, or on an SSE post event.
  import { onMount, onDestroy } from "svelte";
  import Icon from "$lib/ui/Icon.svelte";
  import Skeleton from "$lib/ui/Skeleton.svelte";
  import EmptyState from "$lib/ui/EmptyState.svelte";
  import Badge from "$lib/ui/Badge.svelte";
  import { activityApi, type ActivityRow, type ActivityKind, type ActivityRange } from "$lib/api/activity";
  import { providerLabel } from "$lib/providers";
  import { realtime } from "$lib/stores/realtime";
  import { goto } from "$app/navigation";

  let kind = $state<ActivityKind>("all");
  let range = $state<ActivityRange>("24h");
  let provider = $state("all");
  let rows = $state<ActivityRow[]>([]);
  let loading = $state(true);
  let loadingMore = $state(false);
  let error = $state<string | null>(null);
  let nextBefore = $state<number | null>(null);
  let exhausted = $state(false);

  // Providers present in the loaded rows. The join already returns one, so
  // the filter needs no second request — but it is only as complete as what
  // has been fetched, which is the right trade for a paginated feed: a
  // provider with no activity in range has nothing to filter to anyway.
  let providers = $derived.by(() => {
    const unique = [...new Set(rows.map(r => r.provider).filter((p): p is string => !!p))].sort();
    return [
      { value: "all", label: "All providers" },
      ...unique.map(p => ({ value: p, label: providerLabel(p) })),
    ];
  });

  async function fetchPage(signal?: AbortSignal, cursor?: number | null) {
    const res = await activityApi.list({ kind, range, provider, before: cursor ?? null }, signal);
    if (signal?.aborted) return;
    if (res.error) {
      error = res.error;
      return false;
    }
    if (!res.data) return false;
    const fresh = res.data.rows;
    nextBefore = res.data.next_before;
    // A page short of a full one means there is nothing further back, so the
    // "Load more" button stops instead of issuing a guaranteed-empty request.
    exhausted = nextBefore === null || fresh.length === 0;
    rows = cursor == null ? fresh : [...rows, ...fresh];
    return true;
  }

  async function load(reset = false) {
    if (reset) {
      loading = true;
      error = null;
    } else {
      loadingMore = true;
    }
    const controller = new AbortController();
    const ok = await fetchPage(controller.signal, reset ? null : nextBefore);
    if (ok && reset) error = null;
    loading = false;
    loadingMore = false;
  }

  // Any filter change is a fresh first page.
  $effect(() => {
    void kind; void range; void provider;
    load(true);
  });

  // Activity is derived from post state, so a publish/fail invalidates it.
  let unsubscribers: (() => void)[] = [];
  onMount(() => {
    for (const evt of ["post_published", "post_failed", "post_deleted", "post_created", "post_scheduled", "lagged"]) {
      unsubscribers.push(realtime.on(evt, () => load(true)));
    }
  });
  onDestroy(() => unsubscribers.forEach(fn => fn()));

  // ── Row rendering ──

  const kindLabel: Record<ActivityRow["kind"], string> = {
    event: "Event",
    automation: "Automation",
    publish: "Publish",
  };

  // Matches Badge's variant vocabulary exactly: content-colour text on a
  // hue ring, which is the only badge treatment that clears 4.5:1 at 12px.
  function badgeVariant(row: ActivityRow): "success" | "error" | "info" | "warning" {
    if (row.ok) return "success";
    if (row.kind === "automation" && row.summary.includes("skipped")) return "warning";
    return "error";
  }

  function relative(epochSeconds: number): string {
    const secs = Math.max(0, Math.floor(Date.now() / 1000) - epochSeconds);
    if (secs < 60) return "just now";
    const mins = Math.floor(secs / 60);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return days === 1 ? "yesterday" : `${days}d ago`;
  }
</script>

<svelte:head>
  <title>Activity — Social Forge</title>
</svelte:head>

<div class="max-w-4xl">
  <div class="flex items-start justify-between gap-4 mb-5">
    <div>
      <h1 class="text-xl font-bold text-content">Activity</h1>
      <p class="text-sm text-muted mt-1">What the app did on your behalf, newest first.</p>
    </div>
  </div>

  <!-- Filters. Every control is labelled: the native <label> is what makes
       the association work, so each select gets its own id. -->
  <div class="flex flex-wrap items-end gap-3 mb-5">
    <div>
      <label for="activity-kind" class="block text-xs text-muted mb-1">Type</label>
      <select
        id="activity-kind"
        bind:value={kind}
        class="px-3 py-2 bg-surface-hover border border-line rounded-lg text-sm text-content"
      >
        <option value="all">All</option>
        <option value="event">Events</option>
        <option value="automation">Automation</option>
        <option value="publish">Publish attempts</option>
      </select>
    </div>
    <div>
      <label for="activity-range" class="block text-xs text-muted mb-1">Range</label>
      <select
        id="activity-range"
        bind:value={range}
        class="px-3 py-2 bg-surface-hover border border-line rounded-lg text-sm text-content"
      >
        <option value="24h">Last 24 hours</option>
        <option value="7d">Last 7 days</option>
        <option value="30d">Last 30 days</option>
      </select>
    </div>
    <div>
      <label for="activity-provider" class="block text-xs text-muted mb-1">Provider</label>
      <select
        id="activity-provider"
        bind:value={provider}
        class="px-3 py-2 bg-surface-hover border border-line rounded-lg text-sm text-content"
      >
        {#each providers as p (p.value)}
          <option value={p.value}>{p.label}</option>
        {/each}
      </select>
    </div>
  </div>

  {#if error}
    <div role="alert" class="mb-5 p-4 rounded-lg bg-error/10 border border-error/40 text-sm text-content">
      Could not load activity: {error}
    </div>
  {/if}

  {#if loading}
    <div class="space-y-2" aria-busy="true" aria-live="polite">
      {#each Array(6) as _, i (i)}
        <div class="flex items-center gap-3 p-3 bg-surface border border-line rounded-lg">
          <Skeleton width="4rem" height="1.25rem" rounded="md" />
          <Skeleton height="1rem" />
        </div>
      {/each}
    </div>
  {:else if rows.length === 0}
    <EmptyState
      icon="activity"
      title="No activity yet"
      description="Publishes, automation replies and failed attempts show up here as they happen. Try widening the range."
    />
  {:else}
    <ul class="space-y-2">
      {#each rows as row (row.id)}
        <li class="flex items-start gap-3 p-3 bg-surface border border-line rounded-lg">
          <span class="shrink-0 w-20 text-xs text-muted tabular-nums pt-0.5">{relative(row.at)}</span>
          <Badge variant={badgeVariant(row)}>{kindLabel[row.kind]}</Badge>
          <div class="min-w-0 flex-1">
            <p class="text-sm text-content break-words">{row.summary}</p>
            <div class="flex items-center gap-3 mt-1 text-xs text-muted">
              {#if row.provider}
                <span>{providerLabel(row.provider)}</span>
              {/if}
              {#if row.post_id}
                <!-- A real anchor, so it is keyboard-reachable and
                     middle-clickable without JS. -->
                <a href="/posts/{row.post_id}" class="text-accent hover:underline">View post</a>
              {/if}
              {#if row.rule_id}
                <a href="/automation" class="text-accent hover:underline">View rule</a>
              {/if}
            </div>
          </div>
        </li>
      {/each}
    </ul>

    <div class="mt-4 flex justify-center">
      {#if exhausted}
        <p class="text-xs text-muted">That's everything in this window.</p>
      {:else}
        <button
          onclick={() => load(false)}
          disabled={loadingMore}
          class="px-4 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {loadingMore ? "Loading…" : "Load more"}
        </button>
      {/if}
    </div>
  {/if}
</div>
