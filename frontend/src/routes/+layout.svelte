<script lang="ts">
  import '../app.css';
  import { realtime } from '$lib/stores/realtime';
  import { timezone } from '$lib/stores/timezone.svelte';
  import { initKeyboardShortcuts, destroyKeyboardShortcuts } from '$lib/stores/keyboard.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';
  import StreakBadge from '$lib/streak/StreakBadge.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';

  let { children, data } = $props();
  let showShortcuts = $state(false);

  const navSections = [
    {
      title: '',
      items: [
        { href: '/', label: 'Dashboard', icon: 'dashboard' },
      ],
    },
    {
      title: 'Publish',
      items: [
        { href: '/calendar', label: 'Calendar', icon: 'calendar' },
        { href: '/posts', label: 'Posts', icon: 'post' },
        { href: '/media', label: 'Media', icon: 'media' },
        { href: '/tags', label: 'Tags', icon: 'tag' },
      ],
    },
    {
      title: 'Engage',
      items: [
        { href: '/feed', label: 'Feed', icon: 'feed' },
        { href: '/comments', label: 'Comments', icon: 'comment' },
        { href: '/dms', label: 'DMs', icon: 'dm' },
        { href: '/automation', label: 'Automation', icon: 'automation' },
      ],
    },
    {
      title: 'Insights',
      items: [
        { href: '/search', label: 'Search', icon: 'search' },
        { href: '/analytics', label: 'Analytics', icon: 'analytics' },
      ],
    },
    {
      title: 'Channels',
      items: [
        { href: '/channels', label: 'Channels', icon: 'channel' },
      ],
    },
    {
      title: 'Settings',
      items: [
        { href: '/settings', label: 'General', icon: 'settings' },
        { href: '/settings/profile', label: 'Brand Profile', icon: 'profile' },
        { href: '/settings/rss', label: 'RSS Autopost', icon: 'rss' },
        { href: '/settings/signatures', label: 'Signatures', icon: 'signature' },
        { href: '/settings/developer', label: 'Developer', icon: 'developer' },
        { href: '/settings/webhooks', label: 'Webhooks', icon: 'webhook' },
        { href: '/settings/mcp', label: 'MCP & CLI', icon: 'mcp' },
        { href: '/settings/notifications', label: 'Notifications', icon: 'notification' },
      ],
    },
  ];

  onMount(() => {
    if (data.authenticated) {
      realtime.connect();
      initKeyboardShortcuts(() => showShortcuts = true);
    }
  });

  onDestroy(() => {
    destroyKeyboardShortcuts();
  });
</script>

<Toast />

{#if !data.authenticated}
  {@render children()}
{:else}
  <div class="flex h-screen overflow-hidden bg-background">
    <aside class="w-56 bg-background border-r border-line flex flex-col flex-shrink-0 overflow-y-auto">
      <div class="h-14 flex items-center justify-between px-5 border-b border-line sticky top-0 bg-background z-10">
        <span class="text-indigo-400 font-bold text-lg">Social Forge</span>
        <div class="flex items-center gap-2">
          <StreakBadge />
          <NotificationBell />
        </div>
      </div>
      <nav class="flex-1 py-3 px-2">
        {#each navSections as section}
          {#if section.title}
            <div class="px-3 pt-4 pb-1 text-[10px] font-semibold uppercase tracking-wider text-muted-dark">
              {section.title}
            </div>
          {/if}
          {#each section.items as item}
            <a href={item.href}
              class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors duration-200 cursor-pointer
                {$page.url.pathname === item.href
                  ? 'bg-surface-hover text-indigo-400 font-medium'
                  : 'text-muted hover:text-content hover:bg-surface-hover'}"
            >
              <Icon name={item.icon} class="w-4 h-4 flex-shrink-0" />
              {item.label}
            </a>
          {/each}
        {/each}
      </nav>
      <!-- Timezone picker -->
      <div class="px-3 py-3 border-t border-line">
        <label class="text-[10px] font-semibold uppercase tracking-wider text-muted-dark block mb-1">Timezone</label>
        <select
          value={timezone.value}
          onchange={(e) => timezone.set(e.currentTarget.value)}
          class="w-full px-2 py-1.5 bg-background-input border border-line rounded-lg text-xs text-content focus:outline-none focus:border-indigo-500"
        >
          {#each timezone.commonTimezones as tz}
            <option value={tz}>{tz}</option>
          {/each}
        </select>
      </div>
    </aside>
    <main class="flex-1 overflow-auto">
      <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
    </main>
  </div>
{/if}

<!-- Keyboard shortcut cheat-sheet -->
{#if showShortcuts}
  <div
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-50"
    role="dialog"
    onclick={() => showShortcuts = false}
    onkeydown={(e) => { if (e.key === 'Escape') showShortcuts = false; }}
  >
    <div
      class="bg-surface border border-line rounded-xl p-6 max-w-md w-full mx-4"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold">Keyboard Shortcuts</h3>
        <button onclick={() => showShortcuts = false} class="text-muted hover:text-white text-xl">&times;</button>
      </div>
      <div class="space-y-2 text-sm">
        <div class="flex justify-between"><span class="text-muted">New post</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">n</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Search</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">/</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Go to Calendar</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g c</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Go to Posts</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g p</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Go to Feed</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g f</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Go to Analytics</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g a</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Go to Media</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">g m</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Show this help</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">?</kbd></div>
        <div class="flex justify-between"><span class="text-muted">Close modal</span><kbd class="px-2 py-0.5 bg-surface-hover rounded text-xs font-mono">Esc</kbd></div>
      </div>
      <p class="text-xs text-muted-dark mt-4 text-center">Shortcuts are disabled while typing in inputs.</p>
    </div>
  </div>
{/if}
