<script lang="ts">
  // SchedulePicker — when a post goes out (v25 F2).
  //
  // F2's substantive change is the timezone. The picker had carried a
  // three-paragraph comment explaining that it could not honour the user's
  // selected zone and deferred the fix to v23 — the inputs are wall-clock
  // ("09:00") but were converted with the browser's own offset, so a user in
  // Berlin scheduling 09:00 got 09:00 UTC and the post fired four hours off
  // from what they asked for. That is now done properly, with Intl and no
  // dependency: see zonedWallClockToIso below.
  //
  // The chosen zone is also shown next to the inputs and the resolved UTC
  // instant is echoed back under them, because "did it understand my zone?" is
  // the one question a scheduler has to answer. The timezone store is global and
  // persisted, so setting it here fixes every other date in the app too.
  import { postsApi } from "$lib/api/posts";
  import { toast } from "$lib/stores/toast";
  import { timezone } from "$lib/stores/timezone.svelte";
  import CalendarPopover from "$lib/ui/CalendarPopover.svelte";

  let { scheduledAt, onChange, recurring, onRecurringChange, integrationId }: {
    scheduledAt?: string | null;
    onChange?: (iso: string | null) => void;
    recurring?: { intervalDays: number; endDate: string } | null;
    onRecurringChange?: (r: { intervalDays: number; endDate: string } | null) => void;
    integrationId?: string;
  } = $props();

  let scheduled = $state(!!scheduledAt);
  let dateStr = $state(scheduledAt ? scheduledAt.split("T")[0] : "");
  let timeStr = $state(scheduledAt ? scheduledAt.split("T")[1]?.slice(0, 5) : "");

  let repeatEnabled = $state(!!recurring);
  let intervalDays = $state(recurring?.intervalDays ?? 7);
  let endDateStr = $state(recurring?.endDate?.split("T")[0] ?? "");

  let autoScheduling = $state(false);

  // Zone list: the store's curated set, plus whatever the browser reports when
  // it is not already in the list, so a user outside the curated zones is not
  // forced onto UTC.
  let zoneOptions = $derived.by(() => {
    const base = timezone.commonTimezones;
    const here = Intl.DateTimeFormat().resolvedOptions().timeZone;
    return here && !base.includes(here) ? [here, ...base] : base;
  });
  let browserZone = $derived(Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC');
  let zoneLabel = $derived(timezone.value.replace(/_/g, ' '));

  /** The UTC instant for `date` + `time` wall-clock read in `zone`. */
  function wallClockToUtc(date: string, time: string, zone: string): string {
    // Anchor: pretend the wall clock IS UTC. That gives a ts whose *fields*
    // are exactly what the user typed, which is what we need to measure the
    // zone's offset against.
    const anchor = Date.parse(`${date}T${time}:00Z`);
    if (Number.isNaN(anchor)) return `${date}T${time}:00.000Z`;

    const offsetAt = (ts: number) => {
      // Ask Intl what the wall clock reads in `zone` at that instant, rebuild
      // it as a UTC instant, and the difference is the zone's offset.
      const parts = new Intl.DateTimeFormat('en-US', {
        timeZone: zone, hourCycle: 'h23',
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit', second: '2-digit',
      }).formatToParts(new Date(ts));
      const get = (t: string) => Number(parts.find(p => p.type === t)?.value ?? '0');
      // hourCycle 'h23' keeps midnight at 00. (With `hour12: false` some ICU
      // builds render it as 24, which would push the whole day forward.)
      return Date.UTC(get('year'), get('month') - 1, get('day'), get('hour'), get('minute'), get('second')) - ts;
    };

    let ts = anchor - offsetAt(anchor);
    // One correction pass: near a DST transition the offset at the anchor can
    // differ from the offset at the answer, which would land the post an hour
    // out. Re-measuring at the candidate instant fixes it.
    const adjusted = anchor - offsetAt(ts);
    if (adjusted !== ts) ts = adjusted;
    return new Date(ts).toISOString();
  }

  /** Read a UTC instant as a wall clock in `zone`, as {date, time}. */
  function utcToWallClock(iso: string, zone: string): { date: string; time: string } {
    try {
      const parts = new Intl.DateTimeFormat('en-CA', {
        timeZone: zone, hourCycle: 'h23',
        year: 'numeric', month: '2-digit', day: '2-digit',
        hour: '2-digit', minute: '2-digit',
      }).formatToParts(new Date(iso));
      const get = (t: string) => parts.find(p => p.type === t)?.value ?? '';
      return { date: `${get('year')}-${get('month')}-${get('day')}`, time: `${get('hour')}:${get('minute')}` };
    } catch {
      return { date: iso.slice(0, 10), time: iso.slice(11, 16) };
    }
  }

  // The instant the current inputs resolve to, and how it reads back — shown
  // to the user so a zone mistake is visible before they commit.
  let resolved = $derived(
    dateStr && timeStr ? wallClockToUtc(dateStr, timeStr, timezone.value) : null,
  );
  let resolvedBack = $derived(resolved ? utcToWallClock(resolved, timezone.value) : null);
  // If the round trip does not land on what the user typed, the zone or the
  // input is unparseable. Say so rather than scheduling the wrong instant.
  let roundTripOk = $derived(
    !resolvedBack || (resolvedBack.date === dateStr && resolvedBack.time === timeStr),
  );

  // Sync local state from props when parent resets (P2.1)
  $effect(() => {
    scheduled = !!scheduledAt;
  });
  // The parent stores UTC. Render the fields as wall clock in the CHOSEN zone,
  // so switching zones re-labels the same instant instead of silently shifting
  // the post to a different time.
  $effect(() => {
    if (scheduledAt && typeof scheduledAt === 'string') {
      const w = utcToWallClock(scheduledAt, timezone.value);
      dateStr = w.date;
      timeStr = w.time;
    }
  });
  $effect(() => {
    if (recurring) {
      repeatEnabled = true;
      intervalDays = recurring.intervalDays;
      endDateStr = recurring.endDate;
    } else {
      repeatEnabled = false;
    }
  });

  function update() {
    if (scheduled && dateStr && timeStr) {
      onChange?.(wallClockToUtc(dateStr, timeStr, timezone.value));
    } else {
      onChange?.(null);
    }
  }

  function setZone(next: string) {
    // Deliberately does NOT re-emit. The $effect above re-reads the stored
    // instant in the new zone, so switching the zone RE-LABELS the same
    // moment (09:00 Berlin → 03:00 New York) rather than moving the post to a
    // different time. Re-emitting here would take the old wall clock, read it
    // in the new zone, and land the post hours away from what the user picked.
    timezone.set(next);
  }

  async function autoSchedule() {
    autoScheduling = true;
    try {
      const r = await postsApi.findSlot(integrationId);
      if (r.error) {
        toast(`Auto-schedule failed: ${r.error}`, "error");
        return;
      }
      if (!r.data?.date) {
        toast("No available slot returned", "error");
        return;
      }
      // The backend returns a UTC instant. Read it back as wall clock in the
      // zone the user is looking at — v25 F2: this used to slice the UTC
      // string and hand UTC wall-clock to a converter that then shifted it a
      // second time, so auto-schedule landed hours off in any non-UTC zone.
      const slot = new Date(r.data.date.endsWith('Z') ? r.data.date : r.data.date + 'Z');
      const w = utcToWallClock(slot.toISOString(), timezone.value);
      dateStr = w.date;
      timeStr = w.time;
      scheduled = true;
      update();
    } catch (e) {
      toast(`Auto-schedule failed: ${e instanceof Error ? e.message : "unknown"}`, "error");
    } finally {
      autoScheduling = false;
    }
  }

  function updateRepeat() {
    if (repeatEnabled && endDateStr) {
      onRecurringChange?.({ intervalDays, endDate: `${endDateStr}T23:59:59.000Z` });
    } else {
      onRecurringChange?.(null);
    }
  }
</script>

<div class="space-y-2">
  <label class="flex items-center gap-2 text-sm cursor-pointer">
    <input type="checkbox" bind:checked={scheduled} onchange={update} class="rounded" />
    Schedule for later
  </label>

  {#if scheduled}
    <!-- v26-3: replaced native <input type="date"> with CalendarPopover.
         The time input stays native — it's compact and consistent enough. -->
    <div class="flex gap-2">
      <CalendarPopover bind:value={dateStr} placeholder="Select date" onchange={update} class="flex-1" />
      <!-- v25 F5: the time input had no label at all — no id/for, no
           aria-label — so it announced as an unlabelled field. -->
      <input type="time" bind:value={timeStr} onchange={update} aria-label="Time"
        class="px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary" />
    </div>

    <!-- v25 F2: timezone picker. Writes through the global timezone store so
         the calendar, the analytics range labels, and this picker all agree. -->
    <div>
      <label for="sched-tz" class="text-xs text-muted mb-1 block">Timezone</label>
      <select
        id="sched-tz"
        value={timezone.value}
        onchange={(e) => setZone(e.currentTarget.value)}
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary"
      >
        {#each zoneOptions as z (z)}
          <option value={z}>
            {z.replace(/_/g, ' ')}{z === browserZone ? ' (this device)' : ''}
          </option>
        {/each}
      </select>
    </div>

    <!-- Read-back line. This is the whole point of the picker: it proves which
         instant the wall clock resolved to, in the zone that will publish it. -->
    {#if resolved && roundTripOk}
      <!-- v25 F5: role=status. This read-back is the entire reason the picker
           exists (it proves which instant the wall clock resolved to), and it
           was silent: a screen-reader user changed the date and heard nothing. -->
      <p class="text-[11px] text-muted" role="status">
        {dateStr} {timeStr} in {zoneLabel}
        <span class="text-faint">→ {new Date(resolved).toISOString().replace('.000Z', 'Z')} (UTC)</span>
      </p>
    {:else if resolved}
      <p class="text-[11px] text-warning" role="alert">
        {dateStr} {timeStr} does not round-trip in {zoneLabel} — the clocks change at that moment. Pick another time.
      </p>
    {/if}

    <button onclick={autoSchedule} disabled={autoScheduling} aria-busy={autoScheduling}
      class="w-full px-3 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm text-accent transition-colors flex items-center justify-center gap-2">
      {#if autoScheduling}
        <span class="inline-block w-4 h-4 border-2 border-line border-t-accent rounded-full animate-spin" aria-hidden="true"></span>
        Finding best time in {zoneLabel}...
      {:else}
        Auto-schedule
      {/if}
    </button>

    <div class="border-t border-line pt-2 mt-2">
      <label class="flex items-center gap-2 text-sm cursor-pointer">
        <input type="checkbox" bind:checked={repeatEnabled} onchange={updateRepeat} class="rounded" />
        <span class="text-accent font-medium">Repeat</span>
      </label>

      {#if repeatEnabled}
        <div class="flex gap-2 mt-2">
          <div class="flex-1">
            <label for="rep-interval" class="text-xs text-muted mb-1 block">Every X days</label>
            <input id="rep-interval" type="number" bind:value={intervalDays} onchange={updateRepeat} min="1" max="365"
              class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary" />
          </div>
          <div class="flex-1">
            <label for="rep-end" class="text-xs text-muted mb-1 block">Until</label>
            <!-- v26-3: CalendarPopover for the repeat end date too. -->
            <CalendarPopover bind:value={endDateStr} placeholder="End date" onchange={updateRepeat} class="w-full" />
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
