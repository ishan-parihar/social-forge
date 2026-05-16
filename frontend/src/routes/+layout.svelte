<script lang="ts">
  import '../app.css';
  import { initializeAuth, clearAuth, currentUser, isAuthenticated } from '$lib/stores/auth';
  import { auth } from '$lib/api/auth';
  import { realtime } from '$lib/stores/realtime';
  import Toast from '$lib/components/Toast.svelte';
  import { page } from '$app/stores';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';

  let { children } = $props();

  function logout() {
    clearAuth();
    goto('/login');
  }

  const nav = [
    { href: '/', label: 'Dashboard', icon: '▦' },
    { href: '/calendar', label: 'Calendar', icon: '📅' },
    { href: '/posts', label: 'Posts', icon: '📄' },
    { href: '/channels', label: 'Channels', icon: '🔗' },
    { href: '/tags', label: 'Tags', icon: '🏷' },
    { href: '/media', label: 'Media', icon: '🖼' },
    { href: '/settings', label: 'Settings', icon: '⚙' },
  ];

  onMount(() => {
    initializeAuth();
    if (!$isAuthenticated) return;
    auth.me().then(r => {
      if (r.data) { currentUser.set(r.data); realtime.connect(); }
      // Don't clear auth on failure — it may be transient (backend restart, etc.)
      // API calls will fail individually if the token is truly invalid
    });

    // Listen for unauthorized events from API client
    const onUnauthorized = () => { clearAuth(); goto('/login'); };
    window.addEventListener('auth:unauthorized', onUnauthorized);
    return () => window.removeEventListener('auth:unauthorized', onUnauthorized);
  });
</script>

<Toast />

{#if $page.url.pathname === '/login' || !$isAuthenticated}
  <main class="min-h-screen bg-[#0b0e14]">{@render children()}</main>
{:else}
  <div class="flex h-screen overflow-hidden bg-[#0b0e14]">
    <aside class="w-56 bg-[#0b0e14] border-r border-[#1e2435] flex flex-col flex-shrink-0">
      <div class="h-14 flex items-center px-5 border-b border-[#1e2435]">
        <span class="text-indigo-400 font-bold text-lg">Postiz</span>
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
        {#if $currentUser}
          <div class="text-xs text-[#6b7280] mb-2 truncate">{$currentUser.email}</div>
        {/if}
        <button onclick={logout} class="text-xs text-[#6b7280] hover:text-red-400 transition-colors">Logout</button>
      </div>
    </aside>
    <main class="flex-1 overflow-auto">
      <div class="max-w-6xl mx-auto p-6">{@render children()}</div>
    </main>
  </div>
{/if}
