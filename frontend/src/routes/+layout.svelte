<script lang="ts">
  import '../app.css';
  import { realtime } from '$lib/stores/realtime';
  import { timezone } from '$lib/stores/timezone.svelte';
  import { initKeyboardShortcuts, destroyKeyboardShortcuts } from '$lib/stores/keyboard.svelte';
  import { theme } from '$lib/stores/theme.svelte';
  import { auth } from '$lib/api/auth';
  import { goto } from '$app/navigation';
  import Toast from '$lib/components/Toast.svelte';
  import ModalManager from '$lib/components/ModalManager.svelte';
  import ShortcutsModal from '$lib/components/ShortcutsModal.svelte';
  import { modals } from '$lib/stores/modals.svelte';
  import { composer } from '$lib/stores/composer.svelte';
  import ComposerModal from '$lib/composer/ComposerModal.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';
  import StreakBadge from '$lib/streak/StreakBadge.svelte';
  import OnboardingModal from '$lib/onboarding/OnboardingModal.svelte';
  import Icon from '$lib/ui/Icon.svelte';
  import { page } from '$app/stores';
  import { onMount, onDestroy } from 'svelte';
  import { browser } from '$app/environment';

  let { children, data } = $props();
  let showOnboarding = $state(false);
  // Mobile sidebar state (U-8): on screens < lg, the sidebar is hidden
  // behind a hamburger toggle. State resets on navigation (route change).
  let sidebarOpen = $state(false);

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

  // Phase 2: when the composer store opens, register the ComposerModal
  // with the modal manager. Uses a guard to avoid re-opening on every
  // reactivity tick. The composer store's `open` flag is the single
  // source of truth; the modal id tracks the ModalManager entry.
  let composerModalId: string | null = null;
  $effect(() => {
    if (composer.open && !composerModalId) {
      composerModalId = modals.open(ComposerModal, {}, {
        title: '',
        closeOnClickOutside: false,
        closeOnEscape: false,
        withCloseButton: false,
        fullScreen: true,
      });
    } else if (!composer.open && composerModalId) {
      modals.close(composerModalId);
      composerModalId = null;
    }
  });
  // prevents the sidebar from staying open after the user taps a
  // nav item on mobile.
  $effect(() => {
    $page.url.pathname;
    sidebarOpen = false;
  });

  onMount(() => {
    theme.init();
    if (data.authenticated) {
      realtime.connect();
      initKeyboardShortcuts(() => modals.open(ShortcutsModal, {}, {
        title: 'Keyboard Shortcuts',
        size: 'max-w-md',
      }));
      if (browser && !localStorage.getItem('social-forge-onboarded')) {
        showOnboarding = true;
      }
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
    <!-- Mobile sidebar overlay: clicking outside the sidebar closes it -->
    {#if sidebarOpen}
      <div
        class="fixed inset-0 bg-black/60 z-30 lg:hidden"
        onclick={() => sidebarOpen = false}
        role="presentation"
      ></div>
    {/if}
    <aside
      class="w-56 bg-background border-r border-line flex flex-col flex-shrink-0 overflow-y-auto fixed lg:static inset-y-0 left-0 z-40 transition-transform duration-200
        {sidebarOpen ? 'translate-x-0' : '-translate-x-full lg:translate-x-0'}"
    >
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
      <!-- Sidebar footer: timezone + theme toggle -->
      <div class="px-3 py-3 border-t border-line space-y-2">
        <div>
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
        <button
          onclick={() => theme.toggle()}
          class="flex items-center gap-2 px-2 py-1.5 text-xs text-muted hover:text-content transition-colors"
          title="Toggle dark/light mode"
        >
          {#if theme.value === 'dark'}
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="5"/><line x1="12" y1="1" x2="12" y2="3"/><line x1="12" y1="21" x2="12" y2="23"/><line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/><line x1="1" y1="12" x2="3" y2="12"/><line x1="21" y1="12" x2="23" y2="12"/><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/><line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/></svg>
            Light mode
          {:else}
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/></svg>
            Dark mode
          {/if}
        </button>
        <button
          onclick={async () => { await auth.logout(); goto('/login'); }}
          class="flex items-center gap-2 px-2 py-1.5 text-xs text-muted hover:text-red-400 transition-colors"
          title="Log out"
        >
          <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><polyline points="16 17 21 12 16 7"/><line x1="21" y1="12" x2="9" y2="12"/></svg>
          Log out
        </button>
      </div>
    </aside>
    <main class="flex-1 overflow-auto">
      <!-- Mobile top bar with hamburger toggle (visible only < lg) -->
      <div class="lg:hidden sticky top-0 z-20 bg-background border-b border-line px-4 py-3 flex items-center justify-between">
        <button
          onclick={() => sidebarOpen = !sidebarOpen}
          class="text-content hover:text-indigo-400 transition-colors p-1 -ml-1"
          aria-label="Toggle navigation"
        >
          <svg class="w-6 h-6" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            {#if sidebarOpen}
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            {:else}
              <line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/>
            {/if}
          </svg>
        </button>
        <span class="text-indigo-400 font-bold">Social Forge</span>
        <div class="flex items-center gap-2">
          <StreakBadge />
          <NotificationBell />
        </div>
      </div>
      <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
    </main>
  </div>
{/if}

<!-- First-run onboarding -->
{#if showOnboarding}
  <OnboardingModal onClose={() => showOnboarding = false} />
{/if}

<!-- Global modal manager: renders the modal stack (Phase 0).
     Mounted once at the layout level so any component can open
     modals via the `modals` store. -->
<ModalManager />
