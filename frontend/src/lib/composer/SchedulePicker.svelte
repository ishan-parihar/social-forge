<script lang="ts">
  import { postsApi } from "$lib/api/posts";

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

  function update() {
    if (scheduled && dateStr && timeStr) {
      onChange?.(`${dateStr}T${timeStr}:00.000Z`);
    } else {
      onChange?.(null);
    }
  }

  async function autoSchedule() {
    autoScheduling = true;
    try {
      const r = await postsApi.findSlot(integrationId);
      if (r.error) {
        console.error("Auto-schedule failed:", r.error);
        return;
      }
      if (!r.data?.date) {
        console.error("No available slot returned");
        return;
      }
      const d = new Date(r.data.date + "Z");  // UTC
      dateStr = d.toISOString().slice(0, 10);
      timeStr = d.toISOString().slice(11, 16);
      scheduled = true;
      update();
    } catch (e) {
      console.error("Auto-schedule failed:", e);
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
    <div class="flex gap-2">
      <input type="date" bind:value={dateStr} onchange={update}
        class="flex-1 px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db]" />
      <input type="time" bind:value={timeStr} onchange={update}
        class="flex-1 px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db]" />
    </div>

    <button onclick={autoSchedule} disabled={autoScheduling}
      class="w-full px-3 py-2 bg-[#1a1f2e] hover:bg-[#242b3d] border border-[#2a3045] rounded-lg text-sm text-indigo-400 transition-colors flex items-center justify-center gap-2">
      {#if autoScheduling}
        <span class="animate-spin">⏳</span> Finding best time...
      {:else}
        ✨ Auto-schedule
      {/if}
    </button>

    <div class="border-t border-[#1e2435] pt-2 mt-2">
      <label class="flex items-center gap-2 text-sm cursor-pointer">
        <input type="checkbox" bind:checked={repeatEnabled} onchange={updateRepeat} class="rounded" />
        <span class="text-indigo-400 font-medium">Repeat</span>
      </label>

      {#if repeatEnabled}
        <div class="flex gap-2 mt-2">
          <div class="flex-1">
            <label class="text-xs text-[#9ca3af] mb-1 block">Every X days</label>
            <input type="number" bind:value={intervalDays} onchange={updateRepeat} min="1" max="365"
              class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db]" />
          </div>
          <div class="flex-1">
            <label class="text-xs text-[#9ca3af] mb-1 block">Until</label>
            <input type="date" bind:value={endDateStr} onchange={updateRepeat}
              class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm text-[#d1d5db]" />
          </div>
        </div>
      {/if}
    </div>
  {/if}
</div>
