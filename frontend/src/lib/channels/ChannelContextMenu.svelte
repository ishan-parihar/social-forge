<script lang="ts">
    import Dropdown from "$lib/ui/Dropdown.svelte";
    import TimeSlotEditor from "./TimeSlotEditor.svelte";
    import type { TimeslotEntry } from "$lib/api/integrations";

    let {
        integrationId,
        integrationName = "",
        currentTimeslots = [] as TimeslotEntry[],
        disabled = false,
        isRefreshing = false,
        onRefreshToken,
        onReconnect,
        onRename,
        onToggleDisable,
        onCopyId,
        onDelete,
    }: {
        integrationId: string;
        integrationName?: string;
        currentTimeslots?: TimeslotEntry[];
        disabled?: boolean;
        isRefreshing?: boolean;
        onRefreshToken?: () => void;
        onReconnect?: () => void;
        onRename?: () => void;
        onToggleDisable?: () => void;
        onCopyId?: () => void;
        onDelete?: () => void;
    } = $props();

    let showTimeSlots = $state(false);
    let menuItems = $derived([
        {
            label: "Time slots",
            onclick: () => { showTimeSlots = true; }
        },
        ...(onRefreshToken ? [{
            label: isRefreshing ? "Refreshing…" : "Refresh token",
            onclick: onRefreshToken,
            disabled: isRefreshing
        }] : []),
        ...(onReconnect ? [{
            label: isRefreshing ? "Reconnecting…" : "Reconnect",
            onclick: onReconnect,
            disabled: isRefreshing
        }] : []),
        ...(onRename ? [{
            label: "Rename",
            onclick: onRename
        }] : []),
        {
            label: disabled ? "Enable" : "Disable",
            onclick: () => onToggleDisable?.()
        },
        ...(onCopyId ? [{
            label: "Copy Channel ID",
            onclick: onCopyId
        }] : []),
        ...(onDelete ? [{
            label: "Delete",
            onclick: onDelete,
            variant: "danger" as const
        }] : []),
    ]);
</script>

<div>
    <Dropdown items={menuItems} align="right">
        <button
            aria-label="Channel actions"
            class="text-muted hover:text-white transition-colors text-lg leading-none"
        >&#8942;</button>
    </Dropdown>

    <TimeSlotEditor
        integrationId={integrationId}
        show={showTimeSlots}
        initialTimeslots={currentTimeslots}
        onclose={() => { showTimeSlots = false; }}
    />
</div>
