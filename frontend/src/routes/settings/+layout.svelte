<script lang="ts">
  // Settings sidebar-tab layout (Phase 7, v19).
  //
  // 2-column layout: left sidebar (260px) of tab labels, right content
  // area where each settings sub-page renders. Active tab highlighted;
  // URL syncs to /settings/{tab}. Mobile: sidebar collapses to a dropdown.
  //
  // Inspired by postiz-app's settings.component.tsx (2-column tab layout).
  // Adapted for Social Forge's single-user model — no Teams, no Approved Apps.

  import { page } from '$app/stores';
  import Icon from '$lib/ui/Icon.svelte';

  let { children, data } = $props();

  // Settings tabs — filtered for single-user (no Teams, no Approved Apps).
  const tabs = [
    { href: '/settings', label: 'General', icon: 'settings', exact: true },
    { href: '/settings/profile', label: 'Brand Profile', icon: 'profile' },
    { href: '/settings/rss', label: 'RSS Autopost', icon: 'rss' },
    { href: '/settings/signatures', label: 'Signatures', icon: 'signature' },
    { href: '/settings/notifications', label: 'Notifications', icon: 'notification' },
    { href: '/settings/developer', label: 'Developer', icon: 'developer' },
    { href: '/settings/webhooks', label: 'Webhooks', icon: 'webhook' },
    { href: '/settings/mcp', label: 'MCP & CLI', icon: 'mcp' },
  ];

  let currentPath = $derived($page.url.pathname);
  let mobileSidebarOpen = $state(false);

  function isActive(href: string, exact: boolean): boolean {
    if (exact) return currentPath === href;
    return currentPath.startsWith(href);
  }
</script>

<div class="page-enter flex flex-col lg:flex-row gap-6 max-w-6xl mx-auto">
  <!-- Sidebar (desktop) / Dropdown (mobile) -->
  <aside class="lg:w-56 lg:flex-shrink-0">
    <!-- Mobile: toggle button -->
    <button
      onclick={() => mobileSidebarOpen = !mobileSidebarOpen}
      class="lg:hidden w-full flex items-center justify-between px-4 py-2.5 bg-surface border border-line rounded-lg text-sm mb-2"
    >
      <span>{tabs.find(t => isActive(t.href, t.exact))?.label || 'Settings'}</span>
      <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        {#if mobileSidebarOpen}
          <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
        {:else}
          <line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/>
        {/if}
      </svg>
    </button>

    <!-- Tab list -->
    <nav class="hidden lg:block bg-surface border border-line rounded-xl p-2">
      {#each tabs as tab}
        <a
          href={tab.href}
          class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors
            {isActive(tab.href, tab.exact)
              ? 'bg-surface-hover text-brand-400 font-medium'
              : 'text-muted hover:text-content hover:bg-surface-hover'}"
        >
          <Icon name={tab.icon} class="w-4 h-4 flex-shrink-0" />
          {tab.label}
        </a>
      {/each}
    </nav>

    <!-- Mobile: collapsible tab list -->
    {#if mobileSidebarOpen}
      <nav class="lg:hidden bg-surface border border-line rounded-xl p-2">
        {#each tabs as tab}
          <a
            href={tab.href}
            onclick={() => mobileSidebarOpen = false}
            class="flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors
              {isActive(tab.href, tab.exact)
                ? 'bg-surface-hover text-brand-400 font-medium'
                : 'text-muted hover:text-content hover:bg-surface-hover'}"
          >
            <Icon name={tab.icon} class="w-4 h-4 flex-shrink-0" />
            {tab.label}
          </a>
        {/each}
      </nav>
    {/if}
  </aside>

  <!-- Content area -->
  <main class="flex-1 min-w-0">
    {@render children()}
  </main>
</div>
