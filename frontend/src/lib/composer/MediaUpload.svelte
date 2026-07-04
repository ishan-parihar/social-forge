<script lang="ts">
  import { mediaApi, type MediaItem } from "$lib/api/media";
  import MediaPopover from "$lib/media/MediaPopover.svelte";

  let { items = [], onAdd, onRemove, onInsertUrl }: {
    items?: MediaItem[];
    onAdd?: (item: MediaItem) => void;
    onRemove?: (id: string) => void;
    onInsertUrl?: (url: string) => void;
  } = $props();

  let uploading = $state(false);
  let mediaPopoverOpen = $state(false);
  let uploadError = $state<string | null>(null);

  async function handleUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    uploading = true;
    uploadError = null;
    for (const file of input.files) {
      const r = await mediaApi.upload(file);
      if (r.data) {
        onAdd?.(r.data);
      } else {
        uploadError = r.error || "Upload failed";
      }
    }
    uploading = false;
    input.value = "";
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    if (!e.dataTransfer?.files.length) return;
    uploadError = null;
    const file = e.dataTransfer.files[0];
    mediaApi.upload(file).then(r => {
      if (r.data) {
        onAdd?.(r.data);
      } else {
        uploadError = r.error || "Upload failed";
      }
    });
  }

  function handlePopoverSelect(url: string) {
    onInsertUrl?.(url);
    mediaPopoverOpen = false;
  }
</script>

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
        <div class="relative group">
          {#if item.mime_type.startsWith("image/")}
            <img src={item.url} alt={item.original_name} class="w-full h-20 object-cover rounded-lg" />
          {:else}
            <div class="w-full h-20 bg-[#1e2435] rounded-lg flex items-center justify-center text-xs text-muted">
              {item.original_name}
            </div>
          {/if}
          <button
            aria-label="Remove media"
            onclick={() => onRemove?.(item.id)}
            class="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white rounded-full text-xs opacity-0 group-hover:opacity-100 transition-opacity"
          >&times;</button>
        </div>
      {/each}
    </div>
  {/if}

  {#if uploading}
    <div class="text-sm text-muted">Uploading...</div>
  {:else}
    <div class="flex items-center justify-center gap-4">
      <label class="cursor-pointer text-sm text-indigo-400 hover:text-indigo-300">
        <input type="file" multiple accept="image/*,video/*" onchange={handleUpload} class="hidden" />
        {items.length > 0 ? "Add more media" : "Drop media here or click to upload"}
      </label>
      {#if onInsertUrl}
        <span class="text-[#1e2435]">|</span>
        <button
          onclick={() => mediaPopoverOpen = true}
          class="text-sm text-indigo-400 hover:text-indigo-300 cursor-pointer"
        >
          Choose from library
        </button>
      {/if}
    </div>
  {/if}

  {#if uploadError}
    <p class="text-xs text-red-400 mt-1.5">{uploadError}</p>
  {/if}
</div>
