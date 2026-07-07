<script lang="ts">
  // MediaUpload — upgraded media uploader (Phase 6, v19).
  //
  // Upgrades from v18's basic uploader:
  //   - Drag-to-reorder: native HTML5 DnD on the thumbnail strip
  //   - Alt text: click a thumbnail → inline edit popover
  //   - Clipboard paste: onpaste handler on the drop zone
  //   - Per-file upload progress
  //
  // No Uppy dependency (YAGNI — Uppy is 10+ packages, too heavy for a
  // solo-founder tool). The native File API + our existing mediaApi
  // covers all the needed functionality.

  import { mediaApi, type MediaItem } from "$lib/api/media";
  import MediaPopover from "$lib/media/MediaPopover.svelte";

  let { items = [], onAdd, onRemove, onReorder }: {
    items?: MediaItem[];
    onAdd?: (item: MediaItem) => void;
    onRemove?: (id: string) => void;
    onReorder?: (newItems: MediaItem[]) => void;
  } = $props();

  let uploading = $state(false);
  let uploadProgress = $state<Record<string, number>>({}); // filename → percent
  let mediaPopoverOpen = $state(false);
  let uploadError = $state<string | null>(null);

  // Alt text editing state.
  let editingAlt = $state<string | null>(null); // item.id being edited
  let altTextDraft = $state("");

  // Drag-to-reorder state.
  let draggedId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);

  async function handleUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    await uploadFiles(Array.from(input.files));
    input.value = "";
  }

  async function uploadFiles(files: File[]) {
    uploading = true;
    uploadError = null;
    for (const file of files) {
      uploadProgress = { ...uploadProgress, [file.name]: 0 };
      try {
        // Phase v21: use real XHR upload progress instead of the fake
        // setInterval that polled a fake progress value (+20 every 200ms,
        // capped at 90%). The fake progress was misleading — it showed
        // "90%" indefinitely on slow uploads and jumped to 100% only when
        // the response came back. XHR's upload.onprogress gives real
        // byte-level progress, which is what users expect especially for
        // video uploads.
        const r = await mediaApi.uploadWithProgress(file, (pct: number) => {
          uploadProgress = { ...uploadProgress, [file.name]: pct };
        });
        if (r.data) {
          onAdd?.(r.data);
        } else {
          uploadError = r.error || "Upload failed";
        }
      } catch (e) {
        uploadError = e instanceof Error ? e.message : "Upload failed";
      }
      const { [file.name]: _, ...rest } = uploadProgress;
      uploadProgress = rest;
    }
    uploading = false;
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    if (!e.dataTransfer?.files.length) return;
    uploadFiles(Array.from(e.dataTransfer.files));
  }

  // Clipboard paste: capture pasted images.
  function handlePaste(e: ClipboardEvent) {
    const files: File[] = [];
    for (const item of e.clipboardData?.items || []) {
      if (item.kind === 'file') {
        const file = item.getAsFile();
        if (file) files.push(file);
      }
    }
    if (files.length > 0) {
      e.preventDefault();
      uploadFiles(files);
    }
  }

  // Library pick.
  function handlePopoverSelect(url: string) {
    const isImage = /\.(png|jpe?g|webp|gif|avif|svg)$/i.test(url);
    onAdd?.({
      id: crypto.randomUUID(),
      url,
      mime_type: isImage ? 'image/jpeg' : 'application/octet-stream',
      original_name: url.split('/').pop() || 'library-media',
      file_size: 0,
    });
    mediaPopoverOpen = false;
  }

  // Drag-to-reorder handlers.
  function handleReorderDragStart(e: DragEvent, itemId: string) {
    draggedId = itemId;
    e.dataTransfer?.setData("text/plain", `reorder:${itemId}`);
    e.dataTransfer!.effectAllowed = "move";
  }
  function handleReorderDragOver(e: DragEvent, itemId: string) {
    e.preventDefault();
    if (draggedId && draggedId !== itemId) {
      dragOverId = itemId;
    }
  }
  function handleReorderDrop(e: DragEvent, targetId: string) {
    e.preventDefault();
    if (!draggedId || draggedId === targetId) return;
    const fromIdx = items.findIndex(i => i.id === draggedId);
    const toIdx = items.findIndex(i => i.id === targetId);
    if (fromIdx === -1 || toIdx === -1) return;
    const newItems = [...items];
    const [moved] = newItems.splice(fromIdx, 1);
    newItems.splice(toIdx, 0, moved);
    onReorder?.(newItems);
    draggedId = null;
    dragOverId = null;
  }
  function handleReorderDragEnd() {
    draggedId = null;
    dragOverId = null;
  }

  // Alt text editing.
  function startEditAlt(item: MediaItem) {
    editingAlt = item.id;
    altTextDraft = item.original_name || '';
  }
  function saveAlt() {
    if (editingAlt) {
      // Find the item and update its alt text (original_name used as alt).
      const item = items.find(i => i.id === editingAlt);
      if (item) {
        item.original_name = altTextDraft;
        // Trigger reactivity by creating a new array.
        onReorder?.([...items]);
      }
    }
    editingAlt = null;
    altTextDraft = '';
  }
</script>

<svelte:window onpaste={handlePaste} />

<MediaPopover
  open={mediaPopoverOpen}
  onClose={() => mediaPopoverOpen = false}
  onSelect={handlePopoverSelect}
/>

<div
  ondragover={(e) => e.preventDefault()}
  ondrop={handleDrop}
  class="border-2 border-dashed border-line rounded-lg p-4 text-center hover:border-indigo-500/50 transition-colors"
>
  {#if items.length > 0}
    <div class="grid grid-cols-4 gap-2 mb-3">
      {#each items as item (item.id)}
        <div
          class="relative group cursor-grab active:cursor-grabbing {dragOverId === item.id ? 'ring-2 ring-indigo-500' : ''}"
          draggable={true}
          ondragstart={(e) => handleReorderDragStart(e, item.id)}
          ondragover={(e) => handleReorderDragOver(e, item.id)}
          ondrop={(e) => handleReorderDrop(e, item.id)}
          ondragend={handleReorderDragEnd}
        >
          {#if editingAlt === item.id}
            <!-- Alt text edit popover -->
            <div class="absolute inset-0 z-20 bg-surface border border-indigo-500 rounded-lg p-2 flex flex-col gap-1">
              <input
                type="text"
                bind:value={altTextDraft}
                placeholder="Alt text..."
                class="w-full px-2 py-1 text-xs bg-background-input border border-line rounded focus:border-indigo-500 outline-none"
                onkeydown={(e) => { if (e.key === 'Enter') saveAlt(); if (e.key === 'Escape') { editingAlt = null; } }}
              />
              <div class="flex gap-1">
                <button onclick={saveAlt} class="flex-1 text-[10px] bg-indigo-600 text-white rounded py-0.5">Save</button>
                <button onclick={() => editingAlt = null} class="flex-1 text-[10px] text-muted border border-line rounded py-0.5">Cancel</button>
              </div>
            </div>
          {/if}
          {#if item.mime_type.startsWith("image/")}
            <img src={item.url} alt={item.original_name} class="w-full h-20 object-cover rounded-lg" />
          {:else}
            <div class="w-full h-20 bg-surface-hover rounded-lg flex items-center justify-center text-xs text-muted">
              {item.original_name}
            </div>
          {/if}
          <!-- Hover toolbar -->
          <div class="absolute -top-1 -right-1 flex gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
            <button
              aria-label="Edit alt text"
              onclick={() => startEditAlt(item)}
              class="w-5 h-5 bg-indigo-600 text-white rounded-full text-[10px] flex items-center justify-center hover:bg-indigo-500"
              title="Edit alt text"
            >✎</button>
            <button
              aria-label="Remove media"
              onclick={() => onRemove?.(item.id)}
              class="w-5 h-5 bg-red-500 text-white rounded-full text-xs flex items-center justify-center hover:bg-red-400"
            >&times;</button>
          </div>
          <!-- Alt text indicator -->
          {#if item.original_name && item.original_name !== item.url.split('/').pop()}
            <div class="absolute bottom-0 left-0 right-0 bg-black/60 text-white text-[8px] px-1 py-0.5 truncate rounded-b-lg">
              {item.original_name}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}

  {#if uploading}
    <div class="space-y-1">
      {#each Object.entries(uploadProgress) as [name, pct]}
        <div class="flex items-center gap-2 text-xs text-muted">
          <span class="truncate flex-1">{name}</span>
          <div class="w-24 h-1.5 bg-surface-hover rounded-full overflow-hidden">
            <div class="h-full bg-indigo-500 rounded-full transition-all" style="width: {pct}%"></div>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="flex items-center justify-center gap-4">
      <label class="cursor-pointer text-sm text-indigo-400 hover:text-indigo-300">
        <input type="file" multiple accept="image/*,video/*" onchange={handleUpload} class="hidden" />
        {items.length > 0 ? "Add more media" : "Drop, paste, or click to upload"}
      </label>
      <span class="text-surface-hover">|</span>
      <button
        onclick={() => mediaPopoverOpen = true}
        class="text-sm text-indigo-400 hover:text-indigo-300 cursor-pointer"
      >
        Choose from library
      </button>
    </div>
  {/if}

  {#if uploadError}
    <p class="text-xs text-red-400 mt-1.5">{uploadError}</p>
  {/if}
</div>
