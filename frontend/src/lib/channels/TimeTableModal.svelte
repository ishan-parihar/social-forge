<script lang="ts">
  // TimeTableModal — per-channel posting time preset editor (Phase 2, v19).
  //
  // Lets the user add/remove preferred posting times for a channel.
  // These times show as "ghost slots" in DayView — empty drop targets
  // at the preset times — so the user sees when their channel "usually"
  // posts. The find-slot endpoint returns the next future preset time.
  //
  // Inspired by postiz-app's time.table.tsx.

  import { integrationsApi, type TimeslotEntry } from '$lib/api/integrations';
  import { toast } from '$lib/stores/toast';

  let { integrationId, integrationName, onclose } = $props<{
    integrationId: string;
    integrationName: string;
    onclose: () => void;
  }>();

  let loading = $state(true);
  let saving = $state(false);
  let timeslots = $state<TimeslotEntry[]>([]);
  let newHour = $state(9);
  let newMinute = $state(0);

  // Load existing timeslots on mount.
  $effect(() => {
    loadTimeslots();
  });

  async function loadTimeslots() {
    loading = true;
    const r = await integrationsApi.list();
    if (r.data) {
      const int = r.data.integrations.find(i => i.id === integrationId);
      if (int?.posting_times) {
        timeslots = [...int.posting_times].sort((a, b) => a.time - b.time);
      }
    }
    loading = false;
  }

  function addSlot() {
    const minutes = newHour * 60 + newMinute;
    // Check for duplicates.
    if (timeslots.some(s => s.time === minutes)) {
      toast('That time is already in the list', 'error');
      return;
    }
    // Max 3 slots (backend enforces this).
    if (timeslots.length >= 3) {
      toast('Maximum 3 time slots allowed', 'error');
      return;
    }
    timeslots = [...timeslots, { time: minutes }].sort((a, b) => a.time - b.time);
  }

  function removeSlot(minutes: number) {
    timeslots = timeslots.filter(s => s.time !== minutes);
  }

  async function save() {
    saving = true;
    const r = await integrationsApi.updateTimeslots(integrationId, timeslots);
    if (r.error) {
      toast(`Failed to save: ${r.error}`, 'error');
    } else {
      toast('Time slots saved', 'success');
      onclose();
    }
    saving = false;
  }

  // Format minutes-of-day as "HH:MM AM/PM".
  function formatTime(minutes: number): string {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    const period = h >= 12 ? 'PM' : 'AM';
    const displayH = h === 0 ? 12 : h > 12 ? h - 12 : h;
    return `${displayH}:${String(m).padStart(2, '0')} ${period}`;
  }
</script>

<div class="space-y-4">
  <p class="text-sm text-muted">
    Set preferred posting times for <span class="text-content font-medium">{integrationName}</span>.
    These show as ghost slots in DayView and are used by "find next slot".
  </p>

  {#if loading}
    <div class="text-center py-8 text-sm text-muted">Loading...</div>
  {:else}
    <!-- Existing timeslots -->
    {#if timeslots.length > 0}
      <div class="space-y-2">
        <div class="text-xs text-muted uppercase tracking-wider">Current time slots</div>
        {#each timeslots as slot (slot.time)}
          <div class="flex items-center justify-between bg-surface-hover rounded-lg px-3 py-2">
            <span class="text-sm font-mono">{formatTime(slot.time)}</span>
            <button
              onclick={() => removeSlot(slot.time)}
              class="text-muted hover:text-red-400 text-sm"
              aria-label="Remove time slot"
            >&times;</button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="text-center py-6 text-sm text-muted">
        No time slots set. The "find next slot" feature will default to +2 hours from now.
      </div>
    {/if}

    <!-- Add new slot -->
    <div class="border-t border-line pt-4">
      <div class="text-xs text-muted uppercase tracking-wider mb-2">Add a time slot</div>
      <div class="flex items-center gap-2">
        <select
          bind:value={newHour}
          class="px-2 py-1.5 bg-background-input border border-line rounded-lg text-sm"
        >
          {#each Array.from({ length: 24 }, (_, i) => i) as h}
            <option value={h}>{String(h).padStart(2, '0')}</option>
          {/each}
        </select>
        <span class="text-muted">:</span>
        <select
          bind:value={newMinute}
          class="px-2 py-1.5 bg-background-input border border-line rounded-lg text-sm"
        >
          <option value={0}>00</option>
          <option value={15}>15</option>
          <option value={30}>30</option>
          <option value={45}>45</option>
        </select>
        <button
          onclick={addSlot}
          class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm transition-colors"
        >Add</button>
      </div>
      <p class="text-xs text-muted mt-2">Preview: {formatTime(newHour * 60 + newMinute)}</p>
    </div>

    <!-- Actions -->
    <div class="flex justify-end gap-2 pt-4 border-t border-line">
      <button
        onclick={onclose}
        class="px-4 py-2 text-sm text-muted hover:text-white border border-line rounded-lg transition-colors"
      >Cancel</button>
      <button
        onclick={save}
        disabled={saving}
        class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg transition-colors"
      >{saving ? 'Saving...' : 'Save'}</button>
    </div>
  {/if}
</div>
