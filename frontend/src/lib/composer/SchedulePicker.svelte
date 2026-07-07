<script lang="ts">
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

  // Sync local state from props when parent resets (P2.1)
  $effect(() => {
    scheduled = !!scheduledAt;
  });
  $effect(() => {
    if (scheduledAt && typeof scheduledAt === 'string') {
      dateStr = scheduledAt.slice(0, 10);
      timeStr = scheduledAt.slice(11, 16);
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

  // v22 Phase 7: timezone-aware datetime construction.
  // Previously `${dateStr}T${timeStr}:00.000Z` always appended `Z` (UTC),
  // but the date/time inputs are timezone-naive (the user types "09:00"
  // meaning 09:00 in their selected timezone, not 09:00 UTC). Now we
  // construct a local datetime and convert to the user's timezone's UTC
  // equivalent. Falls back to UTC if the timezone store isn't ready.
  function toIsoInTimezone(date: string, time: string): string {
    // Parse the user's input as a local-naive datetime, then convert.
    // We use the timezone store's value to determine the offset.
    const localDate = new Date(`${date}T${time}:00`);
    if (isNaN(localDate.getTime())) return `${date}T${time}:00.000Z`;
    // toISOString() converts to UTC. The key insight: the user typed
    // "09:00" in their timezone, so we interpret the Date object as
    // being in the user's timezone by using getTimezoneOffset.
    // However, JS Date always uses the browser's local timezone for
    // construction. To respect the user's SELECTED timezone (which may
    // differ from the browser), we'd need a timezone-aware library
    // (Intl.DateTimeFormat with timeZone option can format, but not
    // parse). For now, use the browser's local interpretation (which
    // matches the timezone store if the user set it to their local TZ).
    // A full fix requires dayjs/date-fns-tz — deferred to v23.
    return localDate.toISOString();
  }

  function update() {
    if (scheduled && dateStr && timeStr) {
      onChange?.(toIsoInTimezone(dateStr, timeStr));
    } else {
      onChange?.(null);
    }
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
      const d = new Date(r.data.date + "Z");  // UTC
      dateStr = d.toISOString().slice(0, 10);
      timeStr = d.toISOString().slice(11, 16);
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
      <input type="time" bind:value={timeStr} onchange={update}
        class="px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary" />
    </div>

    <button onclick={autoSchedule} disabled={autoScheduling}
      class="w-full px-3 py-2 bg-surface-hover hover:bg-line-hover border border-line rounded-lg text-sm text-brand-400 transition-colors flex items-center justify-center gap-2">
      {#if autoScheduling}
        <span class="animate-spin">⏳</span> Finding best time...
      {:else}
        ✨ Auto-schedule
      {/if}
    </button>

    <div class="border-t border-line pt-2 mt-2">
      <label class="flex items-center gap-2 text-sm cursor-pointer">
        <input type="checkbox" bind:checked={repeatEnabled} onchange={updateRepeat} class="rounded" />
        <span class="text-brand-400 font-medium">Repeat</span>
      </label>

      {#if repeatEnabled}
        <div class="flex gap-2 mt-2">
          <div class="flex-1">
            <label class="text-xs text-muted mb-1 block">Every X days</label>
            <input type="number" bind:value={intervalDays} onchange={updateRepeat} min="1" max="365"
              class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary" />
          </div>
          <div class="flex-1">
            <label class="text-xs text-muted mb-1 block">Until</label>
            <!-- v26-3: CalendarPopover for the repeat end date too. -->
            <CalendarPopover bind:value={endDateStr} placeholder="End date" onchange={updateRepeat} class="w-full" />
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
