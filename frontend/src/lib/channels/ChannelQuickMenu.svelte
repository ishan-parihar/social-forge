<script lang="ts">
  // ChannelQuickMenu - per-channel ⋯ dropdown for the calendar sidebar
  // (Phase 5, v19).
  //
  // Quick actions for a channel without leaving the current page:
  //   - Create post for this channel → composer.openCreate(undefined, [id])
  //   - Copy channel ID
  //   - Edit time slots → opens TimeTableModal
  //   - Disable / Enable
  //   - Delete (with confirm, refuses if posts exist)
  //
  // Inspired by postiz-app's menu/menu.tsx (10-item context menu).
  // Adapted for Switchpost's single-user model (no customer groups).
  //
  // This is distinct from ChannelContextMenu.svelte (which is a
  // presentational component used by ChannelCard on the /channels page).
  // ChannelQuickMenu is self-contained - it does the API calls itself.

  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { composer } from '$lib/stores/composer.svelte';
  import { modals } from '$lib/stores/modals.svelte';
  import { toast } from '$lib/stores/toast';
  import TimeTableModal from './TimeTableModal.svelte';
  import { goto } from '$app/navigation';

  let { integration, onclose }: {
    integration: Integration;
    onclose: () => void;
  } = $props();

  async function createPostHere() {
    composer.openCreate(undefined, [integration.id]);
    onclose();
  }

  async function copyChannelId() {
    try {
      await navigator.clipboard.writeText(integration.id);
      toast('Channel ID copied', 'success');
    } catch {
      toast('Failed to copy', 'error');
    }
    onclose();
  }

  function editTimeSlots() {
    modals.open(TimeTableModal, {
      integrationId: integration.id,
      integrationName: integration.provider_name,
      onclose: () => {},
    }, {
      title: 'Posting Time Slots',
      size: 'max-w-md',
    });
    onclose();
  }

  async function toggleDisable() {
    const newState = !integration.disabled;
    const r = await integrationsApi.toggleDisable(integration.id, newState);
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast(newState ? 'Channel disabled' : 'Channel enabled', 'success');
    }
    onclose();
  }

  async function deleteChannel() {
    const ok = await modals.areYouSure({
      title: 'Delete this channel?',
      message: `This will disconnect ${integration.provider_name}. Posts already scheduled for this channel will remain but won't publish.`,
      confirmLabel: 'Delete',
      danger: true,
    });
    if (!ok) {
      onclose();
      return;
    }
    const r = await integrationsApi.disconnect(integration.id);
    if (r.error) {
      toast(`Failed: ${r.error}`, 'error');
    } else {
      toast('Channel deleted', 'success');
    }
    onclose();
  }

  function reconnect() {
    goto('/channels');
    onclose();
  }
</script>

<!-- Backdrop to close on outside click -->
<div class="fixed inset-0 z-40" onclick={onclose} role="presentation"></div>

<!-- Menu -->
//
// A MENU, and it says so.
//
// The popover had no role at all, so a screen reader announced a
// stack of unnamed buttons with no indication that it was a menu and
// no way to know what was in it until it was read out one item at a
// time. `role="menu"` plus `role="menuitem"` on each row is what a
// screen reader expects, and it is what the plan's F5 gate names
// ("aria for modals/pickers").
<div
  role="menu"
  aria-label="Channel actions"
  class="absolute right-0 top-full mt-1 w-56 bg-surface border border-line rounded-lg shadow-xl z-50 py-1 text-sm"
>
  <button role="menuitem"
    onclick={createPostHere}
    class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors flex items-center gap-2"
  >
    <span>✏️</span> Create post here
  </button>

  <button role="menuitem"
    onclick={copyChannelId}
    class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors flex items-center gap-2"
  >
    <span>📋</span> Copy channel ID
  </button>

  <button role="menuitem"
    onclick={editTimeSlots}
    class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors flex items-center gap-2"
  >
    <span>⏰</span> Edit time slots
  </button>

  {#if integration.refresh_needed}
    <button role="menuitem"
      onclick={reconnect}
      class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors flex items-center gap-2 text-warning"
    >
      <span>🔄</span> Reconnect
    </button>
  {/if}

  <div class="border-t border-line my-1"></div>

  <button role="menuitem"
    onclick={toggleDisable}
    class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors flex items-center gap-2"
  >
    <span>{integration.disabled ? '✅' : '🚫'}</span>
    {integration.disabled ? 'Enable' : 'Disable'}
  </button>

  <button role="menuitem"
    onclick={deleteChannel}
    class="w-full text-left px-3 py-2 hover:bg-surface-hover transition-colors flex items-center gap-2 text-error"
  >
    <span>🗑️</span> Delete
  </button>
</div>
