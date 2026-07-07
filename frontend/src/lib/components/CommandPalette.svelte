<script lang="ts">
  // v22 Phase 4: Command palette (Cmd+K / Ctrl+K).
  //
  // Lightweight fuzzy-search command launcher. No external deps (cmdk,
  // fuse.js) — uses a simple case-insensitive substring match with
  // section grouping and keyboard navigation.
  //
  // Commands are built from the nav structure + a few action commands.
  // Recent commands are surfaced at the top (persisted to localStorage).
  import { goto } from '$app/navigation';
  import { composer } from '$lib/stores/composer.svelte';
  import { browser } from '$app/environment';
  import { onMount, onDestroy } from 'svelte';
  import Icon from '$lib/ui/Icon.svelte';

  let { onClose }: { onClose: () => void } = $props();

  type Command = {
    id: string;
    label: string;
    section: string;
    icon: string;
    action: () => void;
  };

  // Build the command list. Navigate commands use goto; action commands
  // call store methods or open modals.
  const commands: Command[] = [
    { id: 'goto-dashboard', label: 'Go to Dashboard', section: 'Navigate', icon: 'dashboard', action: () => goto('/') },
    { id: 'goto-calendar', label: 'Go to Calendar', section: 'Navigate', icon: 'calendar', action: () => goto('/calendar') },
    { id: 'goto-kanban', label: 'Go to Pipeline (Kanban)', section: 'Navigate', icon: 'analytics', action: () => goto('/kanban') },
    { id: 'goto-posts', label: 'Go to Posts', section: 'Navigate', icon: 'post', action: () => goto('/posts') },
    { id: 'goto-feed', label: 'Go to Feed', section: 'Navigate', icon: 'feed', action: () => goto('/feed') },
    { id: 'goto-comments', label: 'Go to Comments', section: 'Navigate', icon: 'comment', action: () => goto('/comments') },
    { id: 'goto-dms', label: 'Go to DMs', section: 'Navigate', icon: 'dm', action: () => goto('/dms') },
    { id: 'goto-automation', label: 'Go to Automation', section: 'Navigate', icon: 'automation', action: () => goto('/automation') },
    { id: 'goto-search', label: 'Go to Search', section: 'Navigate', icon: 'search', action: () => goto('/search') },
    { id: 'goto-analytics', label: 'Go to Analytics', section: 'Navigate', icon: 'analytics', action: () => goto('/analytics') },
    { id: 'goto-media', label: 'Go to Media', section: 'Navigate', icon: 'media', action: () => goto('/media') },
    { id: 'goto-tags', label: 'Go to Tags', section: 'Navigate', icon: 'tag', action: () => goto('/tags') },
    { id: 'goto-channels', label: 'Go to Channels', section: 'Navigate', icon: 'channel', action: () => goto('/channels') },
    { id: 'goto-settings', label: 'Go to Settings', section: 'Navigate', icon: 'settings', action: () => goto('/settings') },
    { id: 'goto-settings-profile', label: 'Go to Brand Profile', section: 'Navigate', icon: 'profile', action: () => goto('/settings/profile') },
    { id: 'goto-settings-signatures', label: 'Go to Signatures', section: 'Navigate', icon: 'signature', action: () => goto('/settings/signatures') },
    { id: 'goto-settings-webhooks', label: 'Go to Webhooks', section: 'Navigate', icon: 'webhook', action: () => goto('/settings/webhooks') },
    { id: 'goto-settings-developer', label: 'Go to Developer (API Keys)', section: 'Navigate', icon: 'developer', action: () => goto('/settings/developer') },
    // Actions
    { id: 'action-new-post', label: 'Create New Post', section: 'Actions', icon: 'post', action: () => { composer.openCreate(); } },
    { id: 'action-import-feed', label: 'Import Feed', section: 'Actions', icon: 'feed', action: () => goto('/feed') },
    { id: 'action-connect-channel', label: 'Connect Channel', section: 'Actions', icon: 'channel', action: () => goto('/channels') },
  ];

  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement | null = $state(null);

  // Recent commands (persisted).
  let recentIds: string[] = $state([]);
  if (browser) {
    try {
      recentIds = JSON.parse(localStorage.getItem('social-forge-recent-commands') || '[]');
    } catch {
      recentIds = [];
    }
  }

  // Filter commands by query (case-insensitive substring match on label).
  // Recent commands are surfaced at the top when there's no query.
  let filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) {
      // No query: show recent first, then the rest.
      const recent = recentIds
        .map((id) => commands.find((c) => c.id === id))
        .filter((c): c is Command => !!c);
      const recentSet = new Set(recent.map((c) => c.id));
      const rest = commands.filter((c) => !recentSet.has(c.id));
      return [...recent, ...rest];
    }
    return commands.filter((c) => c.label.toLowerCase().includes(q));
  });

  // Group filtered commands by section for display.
  let grouped = $derived.by(() => {
    const groups: { section: string; items: Command[] }[] = [];
    for (const c of filtered) {
      let g = groups.find((g) => g.section === c.section);
      if (!g) {
        g = { section: c.section, items: [] };
        groups.push(g);
      }
      g.items.push(c);
    }
    return groups;
  });

  // Flatten for index-based navigation.
  let flat = $derived(filtered);

  onMount(() => {
    inputEl?.focus();
    // Close on Escape.
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectedIndex = Math.min(flat.length - 1, selectedIndex + 1);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectedIndex = Math.max(0, selectedIndex - 1);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const cmd = flat[selectedIndex];
        if (cmd) runCommand(cmd);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  onDestroy(() => {});

  function runCommand(cmd: Command) {
    // Record in recent (move to front, cap at 5).
    recentIds = [cmd.id, ...recentIds.filter((id) => id !== cmd.id)].slice(0, 5);
    if (browser) {
      localStorage.setItem('social-forge-recent-commands', JSON.stringify(recentIds));
    }
    cmd.action();
    onClose();
  }

  // Reset selected index when query changes.
  $effect(() => {
    query;
    selectedIndex = 0;
  });
</script>

<!-- Backdrop -->
<div
  class="fixed inset-0 bg-black/50 z-50 flex items-start justify-center pt-[15vh] px-4"
  onclick={onClose}
  role="presentation"
>
  <!-- Palette panel -->
  <div
    class="bg-background-input border border-line rounded-lg shadow-2xl w-full max-w-lg overflow-hidden"
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-label="Command palette"
  >
    <!-- Search input -->
    <div class="flex items-center gap-3 px-4 py-3 border-b border-line">
      <Icon name="search" class="w-4 h-4 text-muted" />
      <input
        bind:this={inputEl}
        bind:value={query}
        placeholder="Type a command or search..."
        class="flex-1 bg-transparent border-none outline-none text-sm text-content placeholder:text-muted"
      />
      <kbd class="text-[10px] text-muted bg-surface-hover px-1.5 py-0.5 rounded">ESC</kbd>
    </div>
    <!-- Results -->
    <div class="max-h-[400px] overflow-y-auto py-2">
      {#if flat.length === 0}
        <div class="px-4 py-8 text-center text-sm text-muted">No commands match "{query}"</div>
      {:else}
        {#each grouped as group}
          <div class="px-4 py-1 text-[10px] font-semibold uppercase tracking-wider text-muted-dark">
            {group.section}
          </div>
          {#each group.items as cmd, i}
            {@const flatIndex = flat.indexOf(cmd)}
            <button
              onclick={() => runCommand(cmd)}
              onmouseenter={() => (selectedIndex = flatIndex)}
              class="w-full flex items-center gap-3 px-4 py-2 text-sm transition-colors text-left
                {selectedIndex === flatIndex ? 'bg-surface-hover text-brand-400' : 'text-content hover:bg-surface-hover'}"
            >
              <Icon name={cmd.icon} class="w-4 h-4 flex-shrink-0 text-muted" />
              <span class="flex-1">{cmd.label}</span>
              {#if recentIds.includes(cmd.id)}
                <span class="text-[10px] text-muted-dark">recent</span>
              {/if}
            </button>
          {/each}
        {/each}
      {/if}
    </div>
    <!-- Footer -->
    <div class="px-4 py-2 border-t border-line flex items-center justify-between text-[10px] text-muted-dark">
      <span>↑↓ navigate · ↵ select · ESC close</span>
      <span>Cmd+K</span>
    </div>
  </div>
</div>
