<script lang="ts">
  import Modal from '$lib/ui/Modal.svelte';
  import Button from '$lib/ui/Button.svelte';
  import { modals } from '$lib/stores/modals.svelte';

  export interface PostSet {
    id: string;
    name: string;
    description?: string;
    content: string;
    channelIds: string[];
    scheduledAt?: string | null;
    createdAt: string;
  }

  let { open = false, onclose, currentContent = '', currentChannelIds: _ch = [], currentScheduleAt = null as string | null, onLoad }: {
    open?: boolean;
    onclose?: () => void;
    currentContent?: string;
    currentChannelIds?: string[];
    currentScheduleAt?: string | null;
    onLoad?: (set: PostSet) => void;
  } = $props();

  let tab = $state<'save' | 'load'>('save');
  let saveName = $state('');
  let saveDescription = $state('');
  let savedSets = $state<PostSet[]>([]);
  let saveError = $state<string | null>(null);
  let loading = $state(false);

  async function loadSets() {
    loading = true;
    try {
      const res = await fetch('/api/sets', { credentials: 'include' });
      if (res.ok) {
        const data = await res.json();
        savedSets = (data as Array<any>).map(s => ({
          id: s.id,
          name: s.name,
          description: s.description,
          content: typeof s.content === 'string' ? s.content : (s.content?.content || ''),
          channelIds: s.channel_ids || [],
          scheduledAt: s.content?.scheduledAt || null,
          createdAt: s.created_at,
        }));
      }
    } catch {
      savedSets = [];
    }
    loading = false;
  }

  $effect(() => {
    if (open) {
      loadSets();
      tab = 'save';
      saveName = '';
      saveDescription = '';
      saveError = null;
    }
  });

  async function handleSave() {
    if (!saveName.trim()) {
      saveError = 'Name is required';
      return;
    }
    saveError = null;
    try {
      const res = await fetch('/api/sets', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          name: saveName.trim(),
          description: saveDescription.trim() || undefined,
          content: { content: currentContent, scheduledAt: currentScheduleAt },
          channel_ids: _ch,
        }),
      });
      if (res.ok) {
        saveName = '';
        saveDescription = '';
        await loadSets();
        tab = 'load';
      } else {
        saveError = 'Failed to save set';
      }
    } catch {
      saveError = 'Network error';
    }
  }

  function handleLoad(set: PostSet) {
    onLoad?.(set);
    onclose?.();
  }

  async function handleDelete(e: MouseEvent, id: string) {
    e.stopPropagation();
    if (!(await modals.areYouSure({
      title: 'Delete this post set?',
      message: 'The template will be permanently deleted. Posts already created from it are unaffected.',
      confirmLabel: 'Delete',
      cancelLabel: 'Cancel',
      danger: true,
    }))) return;
    try {
      await fetch(`/api/sets/${id}`, { method: 'DELETE', credentials: 'include' });
      savedSets = savedSets.filter(s => s.id !== id);
    } catch {
      // ignore
    }
  }
</script>

<Modal {open} title="Post Sets" {onclose}>
  <div class="flex gap-4 border-b border-line mb-4">
    <button
      onclick={() => (tab = 'save')}
      class="pb-2 text-sm font-medium transition-colors"
      class:text-indigo-400={tab === 'save'}
      class:text-muted={tab !== 'save'}
      class:border-b-2={tab === 'save'}
      class:border-indigo-500={tab === 'save'}
      class:border-transparent={tab !== 'save'}
    >
      Save
    </button>
    <button
      onclick={() => (tab = 'load')}
      class="pb-2 text-sm font-medium transition-colors"
      class:text-indigo-400={tab === 'load'}
      class:text-muted={tab !== 'load'}
      class:border-b-2={tab === 'load'}
      class:border-indigo-500={tab === 'load'}
      class:border-transparent={tab !== 'load'}
    >
      Load
    </button>
  </div>

  {#if tab === 'save'}
    <div class="space-y-3">
      {#if saveError}
        <div class="text-sm text-red-400">{saveError}</div>
      {/if}
      <div>
        <label for="ps-name" class="text-xs text-muted block mb-1">Name</label>
        <input
          id="ps-name"
          type="text"
          bind:value={saveName}
          placeholder="e.g. Weekly promotion"
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none"
        />
      </div>
      <div>
        <label for="ps-desc" class="text-xs text-muted block mb-1">Description (optional)</label>
        <textarea
          id="ps-desc"
          bind:value={saveDescription}
          placeholder="Describe this post set..."
          rows="2"
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none resize-y"
        ></textarea>
      </div>
      <div class="text-xs text-muted">
        Saves current content, selected channels, and schedule.
      </div>
      <Button onclick={handleSave}>
        Save Set
      </Button>
    </div>
  {:else}
    <div class="space-y-2 max-h-60 overflow-y-auto">
      {#if loading}
        <div class="text-sm text-muted text-center py-6">Loading...</div>
      {:else if savedSets.length === 0}
        <div class="text-sm text-muted text-center py-6">
          No saved post sets. Save a set on the "Save" tab to reuse it later.
        </div>
      {:else}
        {#each savedSets as set (set.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => handleLoad(set)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleLoad(set); } }}
            class="w-full text-left bg-background border border-line rounded-lg p-3 hover:bg-surface-hover transition-colors cursor-pointer"
            aria-label={'Load set: ' + set.name}
          >
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium truncate">{set.name}</p>
                {#if set.description}
                  <p class="text-xs text-muted mt-0.5 truncate">{set.description}</p>
                {/if}
                <p class="text-xs text-muted mt-1">
                  {set.channelIds.length} channel(s) &middot;
                  {new Date(set.createdAt).toLocaleDateString()}
                </p>
              </div>
              <button
                onclick={(e) => handleDelete(e, set.id)}
                aria-label="Delete set"
                class="text-muted hover:text-red-400 transition-colors text-sm flex-shrink-0"
              >
                &times;
              </button>
            </div>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</Modal>
