<script lang="ts">
  import { toast } from '$lib/stores/toast';
  let files = $state<{ id: string; original_name: string; url: string; mime_type: string; file_size: number }[]>([]);
  let uploading = $state(false);

  async function upload(e: Event) {
    const input = e.target as HTMLInputElement;
    if (!input.files?.length) return;
    uploading = true;
    for (const file of input.files) {
      const fd = new FormData();
      fd.append('file', file);
      const token = localStorage.getItem('token') || '';
      try {
        const res = await fetch('/api/media', { method: 'POST', headers: { 'Authorization': 'Bearer ' + token }, body: fd });
        if (res.ok) { files = [await res.json(), ...files]; toast('Uploaded ' + file.name, 'success'); }
        else toast('Upload failed: ' + file.name, 'error');
      } catch { toast('Upload error', 'error'); }
    }
    uploading = false;
    input.value = '';
  }

  function fmtSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1048576) return (bytes / 1024).toFixed(1) + ' KB';
    return (bytes / 1048576).toFixed(1) + ' MB';
  }
</script>

<div class="space-y-4">
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Media Library</h2>
    <label class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm cursor-pointer transition-colors">
      {uploading ? 'Uploading...' : '+ Upload'}
      <input type="file" multiple accept="image/*" onchange={upload} class="hidden" />
    </label>
  </div>
  <div class="grid grid-cols-4 gap-4">
    {#each files as file}
      <div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
        <img src={file.url} alt={file.original_name} class="w-full h-32 object-cover" />
        <div class="p-2">
          <p class="text-xs truncate">{file.original_name}</p>
          <p class="text-[10px] text-[#6b7280]">{fmtSize(file.file_size)}</p>
        </div>
      </div>
    {/each}
  </div>
  {#if files.length === 0}
    <div class="text-center py-12 text-sm text-[#6b7280]">No media uploaded yet.</div>
  {/if}
</div>
