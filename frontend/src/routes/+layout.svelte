<script lang="ts">
  import '../app.css';
  import { realtime } from '$lib/stores/realtime';
  import Toast from '$lib/components/Toast.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let { children, data } = $props();

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
    }
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
        <NotificationBell />
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
    </aside>
    <main class="flex-1 overflow-auto">
      <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
    </main>
  </div>
{/if}
