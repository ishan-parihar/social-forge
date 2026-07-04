<script lang="ts">
  import { toast } from "$lib/stores/toast";
  import { onMount } from 'svelte';
  import Button from '$lib/ui/Button.svelte';
  import Spinner from '$lib/ui/Spinner.svelte';
  import { signaturesApi, type Signature } from '$lib/api/signatures';

  let signatures = $state<Signature[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);

  let showForm = $state(false);
  let newName = $state('');
  let newContent = $state('');
  let newProvider = $state('');
  let creating = $state(false);
  let formError = $state<string | null>(null);

  let editing = $state<string | null>(null);
  let editName = $state('');
  let editContent = $state('');
  let editProvider = $state('');
  let saving = $state(false);

  onMount(load);

  async function load() {
    loading = true;
    error = null;
    try {
      const r = await signaturesApi.list();
      if (r.error) {
        error = r.error;
      } else if (r.data) {
        signatures = r.data || [];
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to load signatures';
    } finally {
      loading = false;
    }
  }

  async function handleCreate() {
    if (!newName.trim() || !newContent.trim()) {
      formError = 'Name and content are required';
      return;
    }
    creating = true;
    formError = null;
    try {
      const r = await signaturesApi.create({
        name: newName.trim(),
        content: newContent.trim(),
        provider: newProvider.trim() || undefined,
      });
      if (r.error) {
        formError = r.error;
      } else {
        newName = '';
        newContent = '';
        newProvider = '';
        showForm = false;
        load();
      }
    } catch (e: unknown) {
      formError = e instanceof Error ? e.message : 'Failed to create signature';
    } finally {
      creating = false;
    }
  }

  function startEdit(sig: Signature) {
    formError = null;
    editing = sig.id;
    editName = sig.name;
    editContent = sig.content;
    editProvider = sig.provider ?? '';
  }

  function cancelEdit() {
    formError = null;
    editing = null;
  }

  async function handleUpdate(id: string) {
    if (!editName.trim() || !editContent.trim()) {
      formError = 'Name and content are required';
      return;
    }
    saving = true;
    try {
      const r = await signaturesApi.update(id, {
        name: editName.trim(),
        content: editContent.trim(),
        provider: editProvider.trim() || undefined,
      });
      if (r.error) {
        formError = r.error;
      } else {
        editing = null;
        load();
      }
    } catch (e: unknown) {
      formError = e instanceof Error ? e.message : 'Failed to update signature';
    } finally {
      saving = false;
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this signature?')) return;
    try {
      const r = await signaturesApi.delete(id);
      if (r.error) {
        error = r.error;
      } else {
        load();
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : 'Failed to delete signature';
    }
  }

  function preview(content: string): string {
    const stripped = content.replace(/<[^>]*>/g, '').trim();
    return stripped.length > 100 ? stripped.slice(0, 100) + '...' : stripped;
  }
</script>

<div class="page-enter space-y-6">
  <div>
    <h2 class="text-xl font-semibold">Signatures</h2>
    <p class="text-sm text-[#6b7280] mt-1">Create reusable signature blocks that you can insert into posts.</p>
  </div>

  {#if error}
    <div class="bg-[#131720] border border-red-500/30 rounded-xl p-4 text-sm text-red-400">
      {error}
      <button onclick={load} class="ml-2 underline">Retry</button>
    </div>
  {/if}

  {#if formError}
    <div class="text-sm text-red-400">{formError}</div>
  {/if}

  <!-- Create form -->
  <div>
    <Button onclick={() => (showForm = !showForm)}>
      {showForm ? 'Cancel' : 'New Signature'}
    </Button>
  </div>

  {#if showForm}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
      <div>
        <label for="sig-name" class="text-xs text-[#6b7280] block mb-1">Name</label>
        <input
          id="sig-name"
          type="text"
          bind:value={newName}
          placeholder="e.g. Standard CTA"
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
        />
      </div>
      <div>
        <label for="sig-content" class="text-xs text-[#6b7280] block mb-1">Content</label>
        <textarea
          id="sig-content"
          bind:value={newContent}
          placeholder="Signature text (HTML supported)..."
          rows="4"
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none resize-y"
        ></textarea>
      </div>
      <div>
        <label for="sig-provider" class="text-xs text-[#6b7280] block mb-1">Provider (optional — leave empty for global)</label>
        <input
          id="sig-provider"
          type="text"
          bind:value={newProvider}
          placeholder="e.g. x, linkedin, bluesky"
          class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
        />
      </div>
      <Button onclick={handleCreate} disabled={creating}>
        {creating ? 'Creating...' : 'Create'}
      </Button>
    </div>
  {/if}

  <!-- List -->
  {#if loading}
    <div class="flex justify-center py-12">
      <Spinner size="lg" />
    </div>
  {:else if signatures.length === 0 && !showForm}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-8 text-center">
      <p class="text-[#6b7280] text-sm">No signatures yet. Create one to quickly insert reusable content into your posts.</p>
    </div>
  {:else}
    <div class="page-enter space-y-3">
      {#each signatures as sig (sig.id)}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
          {#if editing === sig.id}
            <!-- Inline edit -->
            <div class="page-enter space-y-3">
              <div>
                <label for="edit-name-{sig.id}" class="text-xs text-[#6b7280] block mb-1">Name</label>
                <input
                  id="edit-name-{sig.id}"
                  type="text"
                  bind:value={editName}
                  class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
                />
              </div>
              <div>
                <label for="edit-content-{sig.id}" class="text-xs text-[#6b7280] block mb-1">Content</label>
                <textarea
                  id="edit-content-{sig.id}"
                  bind:value={editContent}
                  rows="3"
                  class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none resize-y"
                ></textarea>
              </div>
              <div>
                <label for="edit-provider-{sig.id}" class="text-xs text-[#6b7280] block mb-1">Provider</label>
                <input
                  id="edit-provider-{sig.id}"
                  type="text"
                  bind:value={editProvider}
                  placeholder="Global"
                  class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
                />
              </div>
              <div class="flex gap-2">
                <Button onclick={() => handleUpdate(sig.id)} disabled={saving}>
                  {saving ? 'Saving...' : 'Save'}
                </Button>
                <Button variant="ghost" onclick={cancelEdit}>Cancel</Button>
              </div>
            </div>
          {:else}
            <!-- Display -->
            <div class="flex items-start justify-between gap-3">
              <div class="flex-1 min-w-0">
                <div class="flex items-center gap-2">
                  <h4 class="text-sm font-medium">{sig.name}</h4>
                  {#if sig.provider}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">{sig.provider}</span>
                  {:else}
                    <span class="text-xs px-2 py-0.5 rounded-full bg-[#1e2435] text-[#6b7280]">Global</span>
                  {/if}
                </div>
                <p class="text-xs text-[#6b7280] mt-1">{preview(sig.content)}</p>
              </div>
              <div class="flex gap-1 flex-shrink-0">
                <Button size="sm" variant="ghost" onclick={() => startEdit(sig)} aria-label="Edit signature">Edit</Button>
                <Button size="sm" variant="ghost" onclick={() => handleDelete(sig.id)} aria-label="Delete signature">Delete</Button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
