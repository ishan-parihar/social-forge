<script lang="ts">
  import '../app.css';
  import { realtime } from '$lib/stores/realtime';
  import Toast from '$lib/components/Toast.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let { children, data } = $props();

  const navSections = [
    {
      title: '',
      items: [
        { href: '/', label: 'Dashboard', icon: '▦' },
      ],
    },
    {
      title: 'Publish',
      items: [
        { href: '/calendar', label: 'Calendar', icon: '📅' },
        { href: '/posts', label: 'Posts', icon: '📄' },
        { href: '/media', label: 'Media', icon: '🖼' },
        { href: '/tags', label: 'Tags', icon: '🏷' },
      ],
    },
    {
      title: 'Engage',
      items: [
        { href: '/feed', label: 'Feed', icon: '📰' },
        { href: '/comments', label: 'Comments', icon: '💬' },
        { href: '/dms', label: 'DMs', icon: '✉' },
        { href: '/automation', label: 'Automation', icon: '⚡' },
      ],
    },
    {
      title: 'Insights',
      items: [
        { href: '/analytics', label: 'Analytics', icon: '📊' },
      ],
    },
    {
      title: 'Channels',
      items: [
        { href: '/channels', label: 'Channels', icon: '🔗' },
      ],
    },
    {
      title: 'Settings',
      items: [
        { href: '/settings', label: 'General', icon: '⚙' },
        { href: '/settings/rss', label: 'RSS Autopost', icon: '📡' },
        { href: '/settings/signatures', label: 'Signatures', icon: '✍' },
        { href: '/settings/developer', label: 'Developer', icon: '🔧' },
        { href: '/settings/webhooks', label: 'Webhooks', icon: '🪝' },
        { href: '/settings/mcp', label: 'MCP & CLI', icon: '🤖' },
        { href: '/settings/notifications', label: 'Notifications', icon: '🔔' },
      ],
    },
  ];

  onMount(() => {
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
    <aside class="w-56 bg-[#0b0e14] border-r border-[#1e2435] flex flex-col flex-shrink-0 overflow-y-auto">
      <div class="h-14 flex items-center justify-between px-5 border-b border-[#1e2435] sticky top-0 bg-[#0b0e14] z-10">
        <span class="text-indigo-400 font-bold text-lg">Social Forge</span>
        <NotificationBell />
      </div>
      <nav class="flex-1 py-3 px-2">
        {#each navSections as section}
          {#if section.title}
            <div class="px-3 pt-4 pb-1 text-[10px] font-semibold uppercase tracking-wider text-[#4b5563]">
              {section.title}
            </div>
          {/if}
          {#each section.items as item}
            <a href={item.href}
              class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors
                {$page.url.pathname === item.href
                  ? 'bg-[#1a1f2e] text-indigo-400 font-medium'
                  : 'text-[#6b7280] hover:text-[#e8edf5] hover:bg-[#1a1f2e]'}"
            >
              <span class="w-4 h-4 flex items-center justify-center text-xs">{item.icon}</span>
              {item.label}
            </a>
          {/each}
        {/each}
      </nav>
    </aside>
    <main class="flex-1 overflow-auto">
      <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
    </main>
  </div>
{/if}
