<script lang="ts">
  import { onMount } from 'svelte';
  import Skeleton from '$lib/ui/Skeleton.svelte';
  import ErrorState from '$lib/ui/ErrorState.svelte';
  import EmptyState from '$lib/ui/EmptyState.svelte';
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
    <h1 class="text-xl font-semibold text-content">Notification Settings</h1>
    <p class="text-sm text-muted mt-1">
      Configure which events reach your notification feed, and when to stay quiet.
    </p>
  </div>

  {#if loading}
    <div class="flex justify-center py-12">
      <Skeleton variant="row" rows={5} />
    </div>
  {:else if error}
    <div class="bg-surface border border-error/30 rounded-xl p-5 text-center space-y-3">
      <p class="text-sm text-error">Failed to load notification preferences</p>
      <p class="text-xs text-muted">{error}</p>
      <button
        onclick={handleRetry}
        class="px-4 py-2 bg-accent-fill hover:bg-accent-fill-hover text-accent-fg text-sm rounded-lg transition-colors"
      >
        Retry
      </button>
    </div>
  {:else}
    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <h2 class="text-sm font-medium text-content">Notification Types</h2>
      <div class="page-enter space-y-3">
        {#each notificationTypes as nt (nt.key)}
          <div class="flex items-center justify-between gap-3 py-2">
            <div class="min-w-0">
              <p class="text-sm text-content-secondary">{nt.label}</p>
              <p class="text-xs text-muted">{nt.description}</p>
            </div>
            <select
              bind:value={prefs[nt.key]}
              aria-label="{nt.label} delivery"
              class="min-h-11 shrink-0 bg-line text-content-secondary border border-line rounded px-3 py-1.5 text-sm focus:outline-none focus:border-accent/50 transition-colors"
            >
              <!--
                Social Forge has exactly one delivery channel today: the
                in-app notification feed. There is no push transport and
                no email transport, so offering "Email" would be a
                setting that silently does nothing. The preference values
                ("push" / "email" / "none") are still what the API
                stores, so the API contract does not change when a
                transport lands.
              -->
              <option value="push">In-app</option>
              <option value="none">None</option>
            </select>
          </div>
        {/each}
      </div>
    </div>

    <div class="bg-surface border border-line rounded-xl p-5 space-y-4">
      <div class="flex items-center justify-between">
        <h2 class="text-sm font-medium text-content">Quiet Hours</h2>
        <!--
          The LABEL carries the target. The checkbox is `sr-only` (the
          visible control is the styled track below it), so the label
          needs the 44px box itself - the track is only 9x20.
        -->
        <label class="relative inline-flex items-center cursor-pointer min-h-11 min-w-11 justify-center">
          <input type="checkbox" bind:checked={quietHoursEnabled} class="min-h-11 sr-only peer" />
          <div class="w-9 h-5 bg-line rounded-full peer peer-checked:bg-accent-fill peer-checked:after:translate-x-full after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all"></div>
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
              class="min-h-11 w-full bg-line text-content-secondary border border-line rounded px-3 py-1.5 text-sm focus:outline-none focus:border-accent/50 transition-colors"
            />
          </div>
          <div>
            <label for="quiet-end" class="text-xs text-muted block mb-1">End Time</label>
            <input
              id="quiet-end"
              type="time"
              bind:value={prefs.quiet_hours_end}
              class="min-h-11 w-full bg-line text-content-secondary border border-line rounded px-3 py-1.5 text-sm focus:outline-none focus:border-accent/50 transition-colors"
            />
          </div>
        </div>
      {/if}
    </div>

    <div class="flex items-center gap-3">
      <button
        onclick={handleSave}
        disabled={saving}
        class="px-5 py-2 bg-accent-fill hover:bg-accent-fill-hover disabled:bg-line-hover disabled:text-muted text-accent-fg text-sm rounded-lg transition-colors duration-150 min-h-11"
      >
        {saving ? 'Saving...' : 'Save Preferences'}
      </button>
    </div>
  {/if}
</div>
