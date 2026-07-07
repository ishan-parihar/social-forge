<script lang="ts">
  import { onMount } from 'svelte';
  import { notificationsApi, type NotificationPrefs } from '$lib/api/notifications';
  import { toast } from '$lib/stores/toast';

  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let prefs = $state<NotificationPrefs>({
    post_published: 'push',
    post_failed: 'push',
    analytics_weekly: 'push',
    quiet_hours_start: null,
    quiet_hours_end: null,
    timezone: 0,
  });

  let quietHoursEnabled = $state(false);

  const notificationTypes: { key: keyof NotificationPrefs; label: string; description: string }[] = [
    { key: 'post_published', label: 'Post Published', description: 'When a scheduled post is successfully published' },
    { key: 'post_failed', label: 'Post Failed', description: 'When a scheduled post fails to publish' },
    { key: 'analytics_weekly', label: 'Weekly Analytics', description: 'Weekly analytics summary for your channels' },
  ];

  async function loadPrefs() {
    loading = true;
    error = null;
    const res = await notificationsApi.getPrefs();
    if (res.data) {
      prefs = res.data;
      quietHoursEnabled = !!(res.data.quiet_hours_start && res.data.quiet_hours_end);
    } else if (res.error) {
      error = res.error;
    }
    loading = false;
  }

  async function handleSave() {
    saving = true;
    error = null;
    const payload: Partial<NotificationPrefs> = {
      ...prefs,
      quiet_hours_start: quietHoursEnabled ? prefs.quiet_hours_start : null,
      quiet_hours_end: quietHoursEnabled ? prefs.quiet_hours_end : null,
    };
    const res = await notificationsApi.updatePrefs(payload);
    if (res.data) {
      prefs = res.data;
      toast('Notification preferences saved', 'success');
    } else if (res.error) {
      error = res.error;
      toast(res.error, 'error');
    }
    saving = false;
  }

  function handleRetry() {
    loadPrefs();
  }

  onMount(loadPrefs);
</script>

<div class="page-enter space-y-6">
  <div>
    <h2 class="text-xl font-semibold text-content">Notification Settings</h2>
    <p class="text-sm text-muted mt-1">Configure how and when you receive notifications.</p>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <svg class="animate-spin h-6 w-6 text-brand-500" fill="none" viewBox="0 0 24 24">
        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z" />
      </svg>
    </div>
  {:else if error}
    <div class="bg-surface border border-error/30 rounded-xl p-5 text-center space-y-3">
      <p class="text-sm text-error">Failed to load notification preferences</p>
      <p class="text-xs text-muted">{error}</p>
      <button
        onclick={handleRetry}
        class="px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white text-sm rounded-lg transition-colors"
      >
        Retry
      </button>
    </div>
  {:else}
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <h3 class="text-sm font-medium text-content">Notification Types</h3>
      <div class="page-enter space-y-3">
        {#each notificationTypes as nt (nt.key)}
          <div class="flex items-center justify-between py-2">
            <div>
              <p class="text-sm text-content-secondary">{nt.label}</p>
              <p class="text-xs text-muted">{nt.description}</p>
            </div>
            <select
              bind:value={prefs[nt.key]}
              class="bg-line text-content-secondary border border-line rounded px-3 py-1.5 text-sm focus:outline-none focus:border-brand-500/50 transition-colors"
            >
              <option value="push">Push</option>
              <option value="email">Email</option>
              <option value="none">None</option>
            </select>
          </div>
        {/each}
      </div>
    </div>

    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <div class="flex items-center justify-between">
        <h3 class="text-sm font-medium text-content">Quiet Hours</h3>
        <label class="relative inline-flex items-center cursor-pointer">
          <input type="checkbox" bind:checked={quietHoursEnabled} class="sr-only peer" />
          <div class="w-9 h-5 bg-line rounded-full peer peer-checked:bg-brand-600 peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all"></div>
        </label>
      </div>
      {#if quietHoursEnabled}
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="quiet-start" class="text-xs text-muted block mb-1">Start Time</label>
            <input
              id="quiet-start"
              type="time"
              bind:value={prefs.quiet_hours_start}
              class="w-full bg-line text-content-secondary border border-line rounded px-3 py-1.5 text-sm focus:outline-none focus:border-brand-500/50 transition-colors"
            />
          </div>
          <div>
            <label for="quiet-end" class="text-xs text-muted block mb-1">End Time</label>
            <input
              id="quiet-end"
              type="time"
              bind:value={prefs.quiet_hours_end}
              class="w-full bg-line text-content-secondary border border-line rounded px-3 py-1.5 text-sm focus:outline-none focus:border-brand-500/50 transition-colors"
            />
          </div>
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-3">
      <button
        onclick={handleSave}
        disabled={saving}
        class="px-5 py-2 bg-brand-600 hover:bg-brand-500 disabled:bg-brand-800 disabled:text-muted text-white text-sm rounded-lg transition-colors duration-150"
      >
        {saving ? 'Saving...' : 'Save Preferences'}
      </button>
    </div>
  {/if}
</div>
