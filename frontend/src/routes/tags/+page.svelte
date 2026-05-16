<script lang="ts">
  import { onMount } from 'svelte';
  import { tagsApi, type Tag } from '$lib/api/tags';

  let tags = $state<Tag[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // Create form
  let newName = $state('');
  let newColor = $state('#6366f1');
  let creating = $state(false);

  // Edit state
  let editingId = $state<string | null>(null);
  let editName = $state('');
  let editColor = $state('');
  let savingEdit = $state(false);

  // Delete confirmation
  let deletingId = $state<string | null>(null);

  const presetColors = [
    '#6366f1', '#8b5cf6', '#d946ef', '#ec4899', '#f43f5e',
    '#ef4444', '#f97316', '#eab308', '#22c55e', '#14b8a6',
    '#06b6d4', '#3b82f6', '#64748b',
  ];

  async function loadTags() {
    loading = true;
    error = null;
    try {
      const r = await tagsApi.list();
      if (r.data) tags = r.data;
      else error = r.error || 'Failed to load tags';
    } catch (e: any) {
      error = e.message || 'Failed to load tags';
    }
    loading = false;
  }

  async function createTag() {
    if (!newName.trim()) return;
    creating = true;
    try {
      const r = await tagsApi.create({ name: newName.trim(), color: newColor });
      if (r.data) {
        tags = [...tags, r.data];
        newName = '';
        newColor = '#6366f1';
      } else {
        error = r.error || 'Failed to create tag';
      }
    } catch (e: any) {
      error = e.message || 'Failed to create tag';
    }
    creating = false;
  }

  function startEdit(tag: Tag) {
    editingId = tag.id;
    editName = tag.name;
    editColor = tag.color;
  }

  function cancelEdit() {
    editingId = null;
    editName = '';
    editColor = '';
  }

  async function saveEdit() {
    if (!editingId || !editName.trim()) return;
    savingEdit = true;
    try {
      const r = await tagsApi.update(editingId, { name: editName.trim(), color: editColor });
      if (r.data) {
        tags = tags.map(t => t.id === editingId ? r.data! : t);
        cancelEdit();
      } else {
        error = r.error || 'Failed to update tag';
      }
    } catch (e: any) {
      error = e.message || 'Failed to update tag';
    }
    savingEdit = false;
  }

  async function confirmDelete(id: string) {
    try {
      const r = await tagsApi.delete(id);
      if (r.data) {
        tags = tags.filter(t => t.id !== id);
      } else {
        error = r.error || 'Failed to delete tag';
      }
    } catch (e: any) {
      error = e.message || 'Failed to delete tag';
    }
    deletingId = null;
  }

  onMount(loadTags);
</script>

<div class="max-w-2xl mx-auto space-y-6">
  <h2 class="text-xl font-semibold">Manage Tags</h2>

  {#if error}
    <div class="bg-red-500/10 border border-red-500/30 text-red-400 text-sm rounded-lg p-3 flex items-center justify-between">
      <span>{error}</span>
      <button onclick={() => error = null} class="text-red-400/70 hover:text-red-400">&times;</button>
    </div>
  {/if}

  <!-- Create Tag -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Create Tag</h3>
    <div class="flex items-end gap-3">
      <div class="flex-1">
        <label class="text-xs text-[#6b7280] block mb-1">Name</label>
        <input
          type="text"
          bind:value={newName}
          placeholder="e.g. urgent, client, idea"
          onkeydown={(e) => e.key === 'Enter' && createTag()}
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
        />
      </div>
      <div>
        <label class="text-xs text-[#6b7280] block mb-1">Color</label>
        <div class="flex items-center gap-1">
          <input
            type="color"
            bind:value={newColor}
            class="w-9 h-9 rounded cursor-pointer bg-transparent border-0"
          />
          <div class="flex gap-0.5">
            {#each presetColors as c}
              <button
                onclick={() => newColor = c}
                class="w-5 h-5 rounded-full border border-[#1e2435] {newColor === c ? 'ring-2 ring-indigo-400' : ''}"
                style="background: {c}"
                title={c}
                aria-label="Select color {c}"
              ></button>
            {/each}
          </div>
        </div>
      </div>
      <button
        onclick={createTag}
        disabled={creating || !newName.trim()}
        class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 rounded-lg text-sm transition-colors"
      >
        {creating ? '...' : 'Add'}
      </button>
    </div>
  </div>

  <!-- Tags List -->
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <h3 class="text-sm font-semibold mb-3">Your Tags</h3>

    {#if loading}
      <div class="text-center py-8 text-sm text-[#6b7280]">Loading tags...</div>
    {:else if tags.length === 0}
      <div class="text-center py-8 text-sm text-[#6b7280]">
        No tags yet. Create one above.
      </div>
    {:else}
      <div class="space-y-2">
        {#each tags as tag (tag.id)}
          <div class="flex items-center gap-3 px-3 py-2 rounded-lg border border-[#1e2435] hover:bg-[#1a1f2e] transition-colors">
            {#if editingId === tag.id}
              <!-- Inline edit -->
              <input
                type="text"
                bind:value={editName}
                class="flex-1 px-2 py-1 bg-[#0d1117] border border-[#1e2435] rounded text-sm focus:border-indigo-500 outline-none"
              />
              <div class="flex items-center gap-1">
                <input type="color" bind:value={editColor} class="w-7 h-7 rounded cursor-pointer bg-transparent border-0" />
                {#each presetColors as c}
                  <button
                    onclick={() => editColor = c}
                    class="w-4 h-4 rounded-full border border-[#1e2435] {editColor === c ? 'ring-2 ring-indigo-400' : ''}"
                    style="background: {c}"
                    aria-label="Select color {c}"
                  ></button>
                {/each}
              </div>
              <button onclick={saveEdit} disabled={savingEdit || !editName.trim()} class="text-xs text-green-400 hover:underline px-1">Save</button>
              <button onclick={cancelEdit} class="text-xs text-[#6b7280] hover:underline px-1">Cancel</button>
            {:else}
              <!-- Display -->
              <span class="w-3 h-3 rounded-full flex-shrink-0" style="background: {tag.color}"></span>
              <span class="flex-1 text-sm">{tag.name}</span>
              <button onclick={() => startEdit(tag)} class="text-xs text-[#6b7280] hover:text-indigo-400 px-1" title="Edit" aria-label="Edit tag">&#9998;</button>
              {#if deletingId === tag.id}
                <span class="text-xs text-[#6b7280]">Delete?</span>
                <button onclick={() => confirmDelete(tag.id)} class="text-xs text-red-400 hover:underline px-1">Yes</button>
                <button onclick={() => deletingId = null} class="text-xs text-[#6b7280] hover:underline px-1">No</button>
              {:else}
                <button onclick={() => deletingId = tag.id} class="text-xs text-[#6b7280] hover:text-red-400 px-1" title="Delete" aria-label="Delete tag">&#128465;</button>
              {/if}
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
