<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { notificationsApi } from '$lib/api/notifications';
  import { realtime } from '$lib/stores/realtime';
  import NotificationPanel from './NotificationPanel.svelte';

  let unreadCount = $state(0);
  let panelOpen = $state(false);
  let containerEl: HTMLDivElement | undefined = $state(undefined);

  async function fetchUnreadCount() {
    const res = await notificationsApi.unreadCount();
    if (res.data) {
      unreadCount = res.data.count;
    } else if (res.error) {
      console.warn('Failed to fetch unread count:', res.error);
    }
  }

  function togglePanel(e: MouseEvent) {
    e.stopPropagation();
    panelOpen = !panelOpen;
  }

  function closePanel() {
    panelOpen = false;
    fetchUnreadCount();
  }

  let unsubscribers: (() => void)[] = [];

  onMount(() => {
    fetchUnreadCount();
    // Polling fallback (every 60s) in case SSE drops silently.
    // The realtime listener below handles the common case where the
    // backend broadcasts `notification_new` and we get instant updates.
    const interval = setInterval(fetchUnreadCount, 60000);
    unsubscribers.push(realtime.on('notification_new', () => fetchUnreadCount()));
    return () => clearInterval(interval);
  });

  onDestroy(() => {
    unsubscribers.forEach(fn => fn());
  });
</script>

<div class="relative" bind:this={containerEl}>
  <button
    onclick={togglePanel}
    class="relative p-1.5 rounded-lg text-muted hover:text-white transition-colors duration-150"
    aria-label="Notifications"
  >
    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
      <path stroke-linecap="round" stroke-linejoin="round" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9" />
    </svg>
    {#if unreadCount > 0}
      <span class="absolute -top-0.5 -right-0.5 inline-flex items-center justify-center min-w-[18px] h-[18px] px-1 text-[10px] font-bold leading-none text-white bg-error rounded-full">
        {unreadCount > 99 ? '99+' : unreadCount}
      </span>
    {/if}
  </button>

  <NotificationPanel open={panelOpen} onclose={closePanel} {containerEl} />
</div>
