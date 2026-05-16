<script lang="ts">
  import Modal from '$lib/ui/Modal.svelte';
  import Button from '$lib/ui/Button.svelte';

  export interface PostSet {
    id: string;
    name: string;
    description: string;
    content: string;
    channelIds: string[];
    scheduledAt: string | null;
    createdAt: string;
  }

  let { open = false, onclose, currentContent = '', currentChannelIds = [], currentScheduleAt = null as string | null, onLoad }: {
    open?: boolean;
    onclose?: () => void;
    currentContent?: string;
    currentChannelIds?: string[];
    currentScheduleAt?: string | null;
    onLoad?: (set: PostSet) => void;
  } = $props();

  const STORAGE_KEY = 'social-forge-post-sets';

  let tab = $state<'save' | 'load'>('save');
  let saveName = $state('');
  let saveDescription = $state('');
  let savedSets = $state<PostSet[]>([]);
  let saveError = $state<string | null>(null);

  function loadSets() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      savedSets = raw ? JSON.parse(raw) : [];
    } catch {
      savedSets = [];
    }
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

  function handleSave() {
    if (!saveName.trim()) {
      saveError = 'Name is required';
      return;
    }
    saveError = null;
    const newSet: PostSet = {
      id: crypto.randomUUID(),
      name: saveName.trim(),
      description: saveDescription.trim(),
      content: currentContent,
      channelIds: [...currentChannelIds],
      scheduledAt: currentScheduleAt,
      createdAt: new Date().toISOString(),
    };
    const updated = [...savedSets, newSet];
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
    savedSets = updated;
    saveName = '';
    saveDescription = '';
  }

  function handleLoad(set: PostSet) {
    onLoad?.(set);
    onclose?.();
  }

  function handleDelete(e: MouseEvent, id: string) {
    e.stopPropagation();
    if (!confirm('Delete this post set?')) return;
    const updated = savedSets.filter(s => s.id !== id);
    localStorage.setItem(STORAGE_KEY, JSON.stringify(updated));
    savedSets = updated;
  }
</script>

<Modal {open} title="Post Sets" {onclose}>
  <div class="flex gap-4 border-b border-[#1e2435] mb-4">
    <button
      onclick={() => (tab = 'save')}
      class="pb-2 text-sm font-medium transition-colors"
      class:text-indigo-400={tab === 'save'}
      class:text-[#6b7280]={tab !== 'save'}
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
      class:text-[#6b7280]={tab !== 'load'}
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
        <label for="ps-name" class="text-xs text-[#6b7280] block mb-1">Name</label>
        <input
          id="ps-name"
          type="text"
          bind:value={saveName}
          placeholder="e.g. Weekly promotion"
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
        />
      </div>
      <div>
        <label for="ps-desc" class="text-xs text-[#6b7280] block mb-1">Description (optional)</label>
        <textarea
          id="ps-desc"
          bind:value={saveDescription}
          placeholder="Describe this post set..."
          rows="2"
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none resize-y"
        ></textarea>
      </div>
      <div class="text-xs text-[#6b7280]">
        Saves current content, selected channels, and schedule.
      </div>
      <Button onclick={handleSave}>
        Save Set
      </Button>
    </div>
  {:else}
    <div class="space-y-2 max-h-60 overflow-y-auto">
      {#if savedSets.length === 0}
        <div class="text-sm text-[#6b7280] text-center py-6">
          No saved post sets. Save a set on the "Save" tab to reuse it later.
        </div>
      {:else}
        {#each savedSets as set (set.id)}
          <div
            role="button"
            tabindex="0"
            onclick={() => handleLoad(set)}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleLoad(set); } }}
            class="w-full text-left bg-[#0b0e14] border border-[#1e2435] rounded-lg p-3 hover:bg-[#1a1f2e] transition-colors cursor-pointer"
            aria-label={'Load set: ' + set.name}
          >
            <div class="flex items-start justify-between gap-2">
              <div class="min-w-0 flex-1">
                <p class="text-sm font-medium truncate">{set.name}</p>
                {#if set.description}
                  <p class="text-xs text-[#6b7280] mt-0.5 truncate">{set.description}</p>
                {/if}
                <p class="text-xs text-[#6b7280] mt-1">
                  {set.channelIds.length} channel(s) &middot;
                  {new Date(set.createdAt).toLocaleDateString()}
                </p>
              </div>
              <button
                onclick={(e) => handleDelete(e, set.id)}
                aria-label="Delete set"
                class="text-[#6b7280] hover:text-red-400 transition-colors text-sm flex-shrink-0"
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
