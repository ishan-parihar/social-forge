<script lang="ts">
  import '../app.css';
  import { teamsApi, type Team } from '$lib/api/teams';
  import { realtime } from '$lib/stores/realtime';
  import Toast from '$lib/components/Toast.svelte';
  import NotificationBell from '$lib/notifications/NotificationBell.svelte';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let { children } = $props();

  let userTeams = $state<Team[]>([]);
  let showTeamSwitcher = $state(false);

  function toggleTeamSwitcher() {
    showTeamSwitcher = !showTeamSwitcher;
    teamsApi.list().then(r => { if (r.data) userTeams = r.data; });
  }

  const nav = [
    { href: '/', label: 'Dashboard', icon: '▦' },
    { href: '/calendar', label: 'Calendar', icon: '📅' },
    { href: '/analytics', label: 'Analytics', icon: '📊' },
    { href: '/posts', label: 'Posts', icon: '📄' },
    { href: '/channels', label: 'Channels', icon: '🔗' },
    { href: '/tags', label: 'Tags', icon: '🏷' },
    { href: '/media', label: 'Media', icon: '🖼' },
    { href: '/settings', label: 'Settings', icon: '⚙' },
    { href: '/settings/team', label: 'Team', icon: '👥' },
    { href: '/settings/developer', label: 'Developer', icon: '🔧' },
    { href: '/settings/signatures', label: 'Signatures', icon: '✍' },
    { href: '/settings/rss', label: 'RSS Autopost', icon: '📡' },
    { href: '/settings/billing', label: 'Billing', icon: '💳' },
  ];

  onMount(() => {
    realtime.connect();
  });
</script>

<Toast />

<div class="flex h-screen overflow-hidden bg-[#0b0e14]">
  <aside class="w-56 bg-[#0b0e14] border-r border-[#1e2435] flex flex-col flex-shrink-0">
    <div class="h-14 flex items-center px-5 border-b border-[#1e2435]">
      <span class="text-indigo-400 font-bold text-lg">Social Forge</span>
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
    <div class="p-4 border-t border-[#1e2435]">
      <div class="relative">
        <button
          onclick={toggleTeamSwitcher}
          class="w-full text-left text-xs text-[#6b7280] hover:text-indigo-400 transition-colors mb-2"
        >
          Switch Team ▾
        </button>
        {#if showTeamSwitcher}
          <div class="absolute bottom-full left-0 mb-1 w-full bg-[#1a1f2e] border border-[#1e2435] rounded-lg shadow-xl z-50 max-h-48 overflow-y-auto">
            {#if userTeams.length === 0}
              <div class="px-3 py-2 text-xs text-[#4b5563]">No teams</div>
            {:else}
              {#each userTeams as team (team.id)}
                <a
                  href="/settings/team"
                  onclick={() => { showTeamSwitcher = false; }}
                  class="block px-3 py-2 text-xs text-[#6b7280] hover:text-[#e8edf5] hover:bg-[#131720] transition-colors"
                >
                  {team.name}
                </a>
              {/each}
            {/if}
          </div>
        {/if}
      </div>
      <div class="px-3 py-1">
        <NotificationBell />
      </div>
    </div>
  </aside>
  <main class="flex-1 overflow-auto">
    <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
  </main>
</div>
