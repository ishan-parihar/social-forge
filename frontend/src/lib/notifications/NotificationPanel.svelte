<script lang="ts">
  import { notificationsApi, type Notification } from '$lib/api/notifications';

  let { open, onclose }: { open: boolean; onclose: () => void } = $props();

  let notifications = $state<Notification[]>([]);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let markingAll = $state(false);

  async function loadNotifications() {
    loading = true;
    loadError = null;
    const res = await notificationsApi.list(10, 0);
    if (res.data) {
      notifications = res.data.data;
    } else if (res.error) {
      loadError = res.error;
      console.warn('Failed to load notifications:', res.error);
    }
    loading = false;
  }

  async function handleMarkRead(id: string) {
    const res = await notificationsApi.markRead(id);
    if (res.data) {
      notifications = notifications.map(n =>
        n.id === id ? { ...n, is_read: true } : n
      );
    } else if (res.error) {
      console.warn('Failed to mark notification as read:', res.error);
    }
  }

  async function handleMarkAllRead() {
    markingAll = true;
    try {
      const res = await notificationsApi.markAllRead();
      if (res.data) {
        notifications = notifications.map(n => ({ ...n, is_read: true }));
      } else if (res.error) {
        console.warn('Failed to mark all notifications as read:', res.error);
      }
    } finally {
      markingAll = false;
    }
  }

  function relativeTime(dateStr: string): string {
    const now = Date.now();
    const date = new Date(dateStr).getTime();
    const diff = now - date;
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    if (days < 7) return `${days}d ago`;
    return new Date(dateStr).toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
  }

  function truncate(text: string, max: number): string {
    return text.length > max ? text.slice(0, max) + '...' : text;
  }

  let panelEl: HTMLDivElement | undefined = $state(undefined);

  $effect(() => {
    if (!open) return;
    const cb = onclose;
    function handleClick(e: MouseEvent) {
      if (panelEl && !panelEl.contains(e.target as Node)) {
        cb();
      }
    }
    const timer = setTimeout(() => {
      document.addEventListener('click', handleClick);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('click', handleClick);
    };
  });

  $effect(() => {
    if (open) {
      loadNotifications();
    }
  });
</script>

{#if open}
  <div
    bind:this={panelEl}
    class="absolute top-full right-0 mt-2 w-80 bg-[#131720] border border-[#1e2435] rounded-xl shadow-2xl z-50 overflow-hidden"
  >
    <div class="flex items-center justify-between px-4 py-3 border-b border-[#1e2435]">
      <span class="text-sm font-medium text-[#d1d5db]">Notifications</span>
      <button
        onclick={handleMarkAllRead}
        disabled={markingAll}
        class="text-xs text-indigo-400 hover:text-indigo-300 transition-colors disabled:opacity-40"
      >
        {markingAll ? 'Marking...' : 'Mark all read'}
      </button>
    </div>

    <div class="max-h-96 overflow-y-auto">
      {#if loading}
        <div class="flex justify-center py-8">
          <svg class="animate-spin h-5 w-5 text-indigo-500" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
          </svg>
        </div>
      {:else if loadError}
        <div class="px-4 py-8 text-center text-sm text-red-400">Failed to load notifications</div>
      {:else if notifications.length === 0}
        <div class="px-4 py-8 text-center text-sm text-[#6b7280]">
          No notifications yet
        </div>
      {:else}
        {#each notifications as n (n.id)}
          <button
            onclick={() => handleMarkRead(n.id)}
            class="w-full text-left px-4 py-3 border-b border-[#1e2435] last:border-b-0 hover:bg-[#1a1f2e] transition-colors"
          >
            <div class="flex items-start gap-2">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  {#if !n.is_read}
                    <span class="w-2 h-2 rounded-full bg-indigo-500 flex-shrink-0"></span>
                  {/if}
                  <span class="text-sm font-medium text-[#d1d5db] {n.is_read ? 'ml-4' : ''}">{n.title}</span>
                </div>
                <p class="text-xs text-[#6b7280] mt-0.5">{truncate(n.body, 100)}</p>
              </div>
              <span class="text-xs text-[#4b5563] flex-shrink-0 pt-0.5">{relativeTime(n.created_at)}</span>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>
{/if}
