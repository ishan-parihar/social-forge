<script lang="ts">
  import { onMount } from 'svelte';

  let {
    integrationId = '',
    onSelect,
    onclose,
  }: {
    integrationId?: string;
    onSelect?: (track: { id: string; title: string; artist: string }) => void;
    onclose?: () => void;
  } = $props();

  let query = $state('');
  let tracks = $state<Array<{ id: string; title: string; artist: string; cover_url?: string; duration_ms?: number }>>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let selectedId = $state<string | null>(null);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  async function search(q: string) {
    if (!integrationId) return;
    loading = true;
    error = null;
    try {
      const params = new URLSearchParams();
      if (q) params.set('q', q);
      const res = await fetch(`/api/integrations/${integrationId}/music?${params}`, { credentials: 'include' });
      if (!res.ok) {
        const err = await res.json().catch(() => ({}));
        throw new Error(err.error || `HTTP ${res.status}`);
      }
      const data = await res.json();
      tracks = data.tracks || [];
    } catch (e) {
      error = e instanceof Error ? e.message : 'Search failed';
      tracks = [];
    }
    loading = false;
  }

  function handleInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => search(query), 400);
  }

  function formatDuration(ms?: number): string {
    if (!ms) return '';
    const s = Math.floor(ms / 1000);
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
  }

  function handleSelect(track: typeof tracks[0]) {
    selectedId = track.id;
    onSelect?.({ id: track.id, title: track.title, artist: track.artist });
  }

  onMount(() => {
    search(''); // Load trending on open
  });
</script>

<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
  <div class="bg-surface border border-line rounded-xl w-full max-w-md mx-4 overflow-hidden">
    <div class="flex items-center justify-between px-4 py-3 border-b border-line">
      <h3 class="text-sm font-semibold">🎵 Add Music</h3>
      <button onclick={onclose} class="text-muted hover:text-white text-xl">&times;</button>
    </div>

    <div class="p-4 space-y-3">
      <input
        type="text"
        bind:value={query}
        oninput={handleInput}
        placeholder="Search trending music..."
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none"
      />

      {#if error}
        <div class="text-xs text-red-400 bg-red-500/10 rounded-lg p-2">
          {error}
          <br>
          <span class="text-muted">Note: Music search requires an Instagram Business/Creator account with the Facebook Login flow.</span>
        </div>
      {/if}

      {#if loading}
        <div class="text-center py-6 text-muted text-sm">Loading...</div>
      {:else if tracks.length === 0 && !error}
        <div class="text-center py-6 text-muted text-sm">No tracks found</div>
      {:else}
        <div class="max-h-80 overflow-y-auto space-y-1">
          {#each tracks as track (track.id)}
            <button
              onclick={() => handleSelect(track)}
              class="w-full flex items-center gap-3 p-2 rounded-lg transition-colors text-left
                {selectedId === track.id ? 'bg-brand-500/20 ring-1 ring-brand-500' : 'hover:bg-surface-hover'}"
            >
              {#if track.cover_url}
                <img src={track.cover_url} alt="" class="w-10 h-10 rounded object-cover shrink-0" />
              {:else}
                <div class="w-10 h-10 rounded bg-surface-hover flex items-center justify-center text-muted shrink-0">🎵</div>
              {/if}
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium truncate">{track.title}</p>
                <p class="text-xs text-muted truncate">{track.artist}</p>
              </div>
              {#if track.duration_ms}
                <span class="text-xs text-muted shrink-0">{formatDuration(track.duration_ms)}</span>
              {/if}
              {#if selectedId === track.id}
                <span class="text-brand-400 text-sm shrink-0">✓</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}

      {#if selectedId}
        <button
          onclick={onclose}
          class="w-full px-4 py-2 bg-brand-500 hover:bg-brand-600 text-white rounded-lg text-sm font-medium"
        >
          Done
        </button>
      {/if}
    </div>
  </div>
</div>
