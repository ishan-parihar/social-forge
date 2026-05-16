<script lang="ts">
  import Button from '$lib/ui/Button.svelte';
  import type { Webhook } from '$lib/api/developer';

  let { webhook, onSave, onCancel }: {
    webhook?: Webhook;
    onSave: (data: { name: string; url: string; secret?: string; event_types: string[]; is_active?: boolean }) => void;
    onCancel: () => void;
  } = $props();

  let name = $state(webhook?.name ?? '');
  let url = $state(webhook?.url ?? '');
  let secret = $state(webhook?.secret ?? '');
  let event_types = $state<string[]>(webhook?.event_types ?? []);
  let is_active = $state(webhook?.is_active ?? true);
  let showSecret = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);

  const allEventTypes = [
    { value: 'post.created', label: 'Post Created' },
    { value: 'post.published', label: 'Post Published' },
    { value: 'post.failed', label: 'Post Failed' },
    { value: 'post.scheduled', label: 'Post Scheduled' },
  ];

  function toggleEventType(val: string) {
    if (event_types.includes(val)) {
      event_types = event_types.filter(v => v !== val);
    } else {
      event_types = [...event_types, val];
    }
  }

  async function handleSubmit() {
    error = null;
    if (!name.trim()) { error = 'Name is required'; return; }
    if (!url.trim()) { error = 'URL is required'; return; }
    if (!url.trim().startsWith('https://')) {
      error = 'URL must start with https://';
      return;
    }
    if (event_types.length === 0) { error = 'Select at least one event type'; return; }

    saving = true;
    try {
      const data: { name: string; url: string; secret?: string; event_types: string[]; is_active?: boolean } = {
        name: name.trim(),
        url: url.trim(),
        event_types,
        is_active,
      };
      // Empty secret means keep existing — no change on edit
      if (secret.trim()) data.secret = secret.trim();
      onSave(data);
    } finally {
      saving = false;
    }
  }
</script>

<div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
  {#if error}
    <div class="text-sm text-red-400">{error}</div>
  {/if}

  <div>
    <label for="wh-name" class="text-xs text-[#6b7280] block mb-1">Name</label>
    <input
      id="wh-name"
      type="text"
      bind:value={name}
      placeholder="e.g. Production Hook"
      class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
    />
  </div>

  <div>
    <label for="wh-url" class="text-xs text-[#6b7280] block mb-1">URL</label>
    <input
      id="wh-url"
      type="url"
      bind:value={url}
      placeholder="https://example.com/webhook"
      class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
    />
  </div>

  <div>
    <label for="wh-secret" class="text-xs text-[#6b7280] block mb-1">Secret (optional — used for HMAC signing)</label>
    <div class="relative">
      <input
        id="wh-secret"
        type={showSecret ? 'text' : 'password'}
        bind:value={secret}
        placeholder="Leave empty for no signing"
        class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none pr-10"
      />
      <button
        type="button"
        onclick={() => (showSecret = !showSecret)}
        class="absolute right-2 top-1/2 -translate-y-1/2 text-xs text-[#6b7280] hover:text-white"
        aria-label={showSecret ? 'Hide secret' : 'Show secret'}
      >
        {showSecret ? 'Hide' : 'Show'}
      </button>
    </div>
  </div>

  <div>
    <span class="text-xs text-[#6b7280] block mb-1">Event Types</span>
    <div class="flex flex-wrap gap-2">
      {#each allEventTypes as et (et.value)}
        <label class="flex items-center gap-1.5 text-sm cursor-pointer">
          <input
            type="checkbox"
            checked={event_types.includes(et.value)}
            onchange={() => toggleEventType(et.value)}
            class="accent-indigo-500"
          />
          <span class="text-[#d1d5db]">{et.label}</span>
        </label>
      {/each}
    </div>
  </div>

  {#if webhook}
    <div>
      <label class="flex items-center gap-2 text-sm cursor-pointer">
        <input type="checkbox" bind:checked={is_active} class="accent-indigo-500" />
        <span class="text-[#d1d5db]">Active</span>
      </label>
    </div>
  {/if}

  <div class="flex gap-2 pt-2">
    <Button onclick={handleSubmit} disabled={saving}>
      {saving ? 'Saving...' : webhook ? 'Update' : 'Create'}
    </Button>
    <Button variant="ghost" onclick={onCancel}>Cancel</Button>
  </div>
</div>
