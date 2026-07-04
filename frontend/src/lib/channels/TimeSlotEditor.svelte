<script lang="ts">
    import { integrationsApi, type TimeslotEntry } from "$lib/api/integrations";
    import Modal from "$lib/ui/Modal.svelte";
    import Spinner from "$lib/ui/Spinner.svelte";

    let {
        integrationId,
        initialTimeslots = [] as TimeslotEntry[],
        show = false,
        onclose = () => {},
    }: {
        integrationId: string;
        initialTimeslots?: TimeslotEntry[];
        show?: boolean;
        onclose?: () => void;
    } = $props();

    let timeslots = $state<TimeslotEntry[]>(initialTimeslots.map(t => ({ ...t })));
    let saving = $state(false);
    let error = $state<string | null>(null);
    let success = $state(false);

    $effect(() => {
      if (show) {
        timeslots = initialTimeslots.map(t => ({ ...t }));
        error = null;
        success = false;
      }
    });

    function minToTime(m: number): string {
        const h = Math.floor(m / 60);
        const min = m % 60;
        return `${String(h).padStart(2, "0")}:${String(min).padStart(2, "0")}`;
    }

    function timeToMin(t: string): number {
        const [h, min] = t.split(":").map(Number);
        return h * 60 + min;
    }

    function addSlot() {
        if (timeslots.length >= 3) return;
        const now = new Date();
        const defaultMin = Math.ceil(now.getHours() * 60 / 60) * 60;
        timeslots = [...timeslots, { time: defaultMin % 1440 }];
    }

    function removeSlot(index: number) {
        timeslots = timeslots.filter((_, i) => i !== index);
    }

    function updateSlot(index: number, timeStr: string) {
        const newTime = timeToMin(timeStr);
        timeslots = timeslots.map((t, i) => i === index ? { time: newTime } : t);
    }

    async function save() {
        saving = true;
        error = null;
        success = false;
        try {
            const r = await integrationsApi.updateTimeslots(integrationId, timeslots);
            if (r.error) {
                error = r.error;
            } else {
                success = true;
                setTimeout(() => onclose(), 1000);
            }
        } catch (e: unknown) {
            error = (e instanceof Error ? e.message : String(e)) || "Failed to save time slots";
        } finally {
            saving = false;
        }
    }
</script>

<Modal open={show} title="Posting Time Slots" {onclose}>
    <div class="space-y-4">
        {#if error}
            <div class="text-sm text-red-400 bg-red-400/10 rounded-md px-3 py-2">{error}</div>
        {/if}
        {#if success}
            <div class="text-sm text-green-400 bg-green-400/10 rounded-md px-3 py-2">Saved successfully!</div>
        {/if}

        <p class="text-sm text-[#9ca3af]">Set up to 3 preferred posting times per day for this channel.</p>

        <div class="space-y-2">
            {#each timeslots as slot, i (i)}
                <div class="flex items-center gap-2">
                    <input
                        type="time"
                        value={minToTime(slot.time)}
                        oninput={(e) => updateSlot(i, e.currentTarget.value)}
                        class="bg-[#1a1f2e] border border-line rounded-md px-3 py-1.5 text-sm text-content-secondary focus:outline-none focus:ring-1 focus:ring-indigo-500"
                    />
                    <button
                        onclick={() => removeSlot(i)}
                        class="text-muted hover:text-red-400 transition-colors text-sm"
                        aria-label="Remove time slot"
                    >✕</button>
                </div>
            {/each}
        </div>

        {#if timeslots.length < 3}
            <button
                onclick={addSlot}
                class="text-sm text-indigo-400 hover:text-indigo-300 transition-colors"
            >+ Add slot</button>
        {:else}
            <p class="text-xs text-muted">Maximum 3 slots reached.</p>
        {/if}

        <div class="flex justify-end gap-2 pt-2 border-t border-line">
            <button
                onclick={onclose}
                class="px-3 py-1.5 text-sm text-[#9ca3af] hover:text-white transition-colors"
                disabled={saving}
            >Cancel</button>
            <button
                onclick={save}
                disabled={saving}
                class="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-md hover:bg-indigo-500 disabled:opacity-50 transition-colors"
            >
                {#if saving}
                    <Spinner size="sm" />
                {:else}
                    Save
                {/if}
            </button>
        </div>
    </div>
</Modal>
