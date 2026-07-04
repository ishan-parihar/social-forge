<script lang="ts">
  import { notificationsApi, type Notification } from '$lib/api/notifications';

  let { open, onclose, containerEl }: { open: boolean; onclose: () => void; containerEl?: HTMLDivElement } = $props();

  let notifications = $state<Notification[]>([]);
  let loading = $state(true);
  let loadError = $state<string | null>(null);
  let markingAll = $state(false);
  let panelStyle = $state('');

  async function loadNotifications() {
    loading = true;
    loadError = null;
    const res = await notificationsApi.list(10, 0);
    if (res.data) {
      notifications = res.data.data;
    } else if (res.error) {
      loadError = res.error;
    }
    loading = false;
  }

  async function handleMarkRead(id: string) {
    const res = await notificationsApi.markRead(id);
    if (res.data) {
      notifications = notifications.map(n =>
        n.id === id ? { ...n, is_read: true } : n
      );
    }
  }

  async function handleMarkAllRead() {
    markingAll = true;
    const res = await notificationsApi.markAllRead();
    if (res.data) {
      notifications = notifications.map(n => ({ ...n, is_read: true }));
    }
    markingAll = false;
  }

  function relativeTime(dateStr: string): string {
    const diff = Date.now() - new Date(dateStr).getTime();
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

  $effect(() => {
    if (!open) return;
    // Position panel below the bell button
    if (containerEl) {
      const rect = containerEl.getBoundingClientRect();
      panelStyle = `position:fixed; top:${rect.bottom + 8}px; left:${rect.left}px; z-index:9999;`;
    }
    const cb = onclose;
    function handleClick(e: MouseEvent) {
      if (containerEl && containerEl.contains(e.target as Node)) return;
      cb();
    }
    function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') cb();
    }
    const timer = setTimeout(() => {
      document.addEventListener('click', handleClick);
      document.addEventListener('keydown', handleKeydown);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('click', handleClick);
      document.removeEventListener('keydown', handleKeydown);
    };
  });

  $effect(() => {
    if (open) loadNotifications();
  });
</script>

{#if open}
  <div
    class="w-80 bg-surface border border-line rounded-xl shadow-2xl overflow-hidden"
    style={panelStyle}
  >
    <div class="flex items-center justify-between px-4 py-3 border-b border-line">
      <span class="text-sm font-medium text-content-secondary">Notifications</span>
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
        <div class="px-4 py-8 text-center text-sm text-muted">No notifications yet</div>
      {:else}
        {#each notifications as n (n.id)}
          <button
            onclick={() => handleMarkRead(n.id)}
            class="w-full text-left px-4 py-3 border-b border-line last:border-b-0 hover:bg-surface-hover transition-colors"
          >
            <div class="flex items-start gap-2">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  {#if !n.is_read}
                    <span class="w-2 h-2 rounded-full bg-indigo-500 flex-shrink-0"></span>
                  {/if}
                  <span class="text-sm font-medium text-content-secondary {n.is_read ? 'ml-4' : ''}">{n.title}</span>
                </div>
                <p class="text-xs text-muted mt-0.5">{truncate(n.body, 100)}</p>
              </div>
              <span class="text-xs text-muted-dark flex-shrink-0 pt-0.5">{relativeTime(n.created_at)}</span>
            </div>
          </button>
        {/each}
      {/if}
    </div>
  </div>
{/if}
