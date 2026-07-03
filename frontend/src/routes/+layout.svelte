<script lang="ts">
  import '../app.css';
  import { realtime } from '$lib/stores/realtime';
  import Toast from '$lib/components/Toast.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let { children, data } = $props();

  const nav = [
    { href: '/', label: 'Dashboard', icon: '▦' },
    { href: '/calendar', label: 'Calendar', icon: '📅' },
    { href: '/feed', label: 'Feed', icon: '📰' },
    { href: '/comments', label: 'Comments', icon: '💬' },
    { href: '/dms', label: 'DMs', icon: '✉' },
    { href: '/automation', label: 'Automation', icon: '⚡' },
    { href: '/analytics', label: 'Analytics', icon: '📊' },
    { href: '/posts', label: 'Posts', icon: '📄' },
    { href: '/channels', label: 'Channels', icon: '🔗' },
    { href: '/tags', label: 'Tags', icon: '🏷' },
    { href: '/media', label: 'Media', icon: '🖼' },
    { href: '/settings', label: 'Settings', icon: '⚙' },
    { href: '/settings/rss', label: 'RSS Autopost', icon: '📡' },
    { href: '/settings/signatures', label: 'Signatures', icon: '✍' },
    { href: '/settings/developer', label: 'Developer', icon: '🔧' },
    { href: '/settings/mcp', label: 'MCP & CLI', icon: '🤖' },
    { href: '/settings/notifications', label: 'Notifications', icon: '🔔' },
  ];

  onMount(() => {
    // Only connect realtime if we're authenticated (not on /login).
    if (data.authenticated) {
      realtime.connect();
    }
  });
</script>

<Toast />

{#if !data.authenticated}
  {@render children()}
{:else}
  <div class="flex h-screen overflow-hidden bg-[#0b0e14]">
    <aside class="w-56 bg-[#0b0e14] border-r border-[#1e2435] flex flex-col flex-shrink-0">
      <div class="h-14 flex items-center justify-between px-5 border-b border-[#1e2435]">
        <span class="text-indigo-400 font-bold text-lg">Social Forge</span>
        <NotificationBell />
      </div>
      <nav class="flex-1 py-3 space-y-0.5 px-2">
        {#each nav as item}
          <a href={item.href}
            class="flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm transition-colors
              {$page.url.pathname === item.href ? 'bg-[#1a1f2e] text-indigo-400' : 'text-[#6b7280] hover:text-[#e8edf5] hover:bg-[#1a1f2e]'}"
          >
            <span class="w-4 h-4 flex items-center justify-center">{item.icon}</span>
            {item.label}
          </a>
        {/each}
      </nav>
    </aside>
    <main class="flex-1 overflow-auto">
      <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
    </main>
  </div>
{/if}

