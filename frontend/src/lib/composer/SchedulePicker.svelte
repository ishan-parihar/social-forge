<script lang="ts">
  let { scheduledAt, onChange }: {
    scheduledAt?: string | null;
    onChange?: (iso: string | null) => void;
  } = $props();

  let scheduled = $state(!!scheduledAt);
  let dateStr = $state(scheduledAt ? scheduledAt.split("T")[0] : "");
  let timeStr = $state(scheduledAt ? scheduledAt.split("T")[1]?.slice(0, 5) : "");

  function update() {
    if (scheduled && dateStr && timeStr) {
      onChange?.(`${dateStr}T${timeStr}:00.000Z`);
    } else {
      onChange?.(null);
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
  {/if}
</div>
