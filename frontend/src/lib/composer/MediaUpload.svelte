<script lang="ts">
  import { mediaApi, type MediaItem } from "$lib/api/media";

  let { items = [], onAdd, onRemove }: {
    items?: MediaItem[];
    onAdd?: (item: MediaItem) => void;
    onRemove?: (id: string) => void;
  } = $props();

  let uploading = $state(false);

  async function handleUpload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    uploading = true;
    for (const file of input.files) {
      const r = await mediaApi.upload(file);
      if (r.data) onAdd?.(r.data);
    }
    uploading = false;
    input.value = "";
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    if (!e.dataTransfer?.files.length) return;
    const file = e.dataTransfer.files[0];
    mediaApi.upload(file).then(r => { if (r.data) onAdd?.(r.data); });
  }
</script>

<div
  ondragover={(e) => e.preventDefault()}
  ondrop={handleDrop}
  class="border-2 border-dashed border-[#1e2435] rounded-lg p-4 text-center hover:border-indigo-500/50 transition-colors"
>
  {#if items.length > 0}
    <div class="grid grid-cols-4 gap-2 mb-3">
      {#each items as item}
        <div class="relative group">
          {#if item.mime_type.startsWith("image/")}
            <img src={item.url} alt={item.original_name} class="w-full h-20 object-cover rounded-lg" />
          {:else}
            <div class="w-full h-20 bg-[#1e2435] rounded-lg flex items-center justify-center text-xs text-[#6b7280]">
              {item.original_name}
            </div>
          {/if}
          <button
            onclick={() => onRemove?.(item.id)}
            class="absolute -top-1 -right-1 w-5 h-5 bg-red-500 text-white rounded-full text-xs opacity-0 group-hover:opacity-100 transition-opacity"
          >&times;</button>
        </div>
      {/each}
    </div>
  {/if}

  {#if uploading}
    <div class="text-sm text-[#6b7280]">Uploading...</div>
  {:else}
    <label class="cursor-pointer text-sm text-indigo-400 hover:text-indigo-300">
      <input type="file" multiple accept="image/*,video/*" onchange={handleUpload} class="hidden" />
      {items.length > 0 ? "Add more media" : "Drop media here or click to upload"}
    </label>
  {/if}
</div>
