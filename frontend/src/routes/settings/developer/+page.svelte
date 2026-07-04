<script lang="ts">
  import { toast } from "$lib/stores/toast";
  import { onMount } from 'svelte';
  import Button from '$lib/ui/Button.svelte';
  import Modal from '$lib/ui/Modal.svelte';
  import Spinner from '$lib/ui/Spinner.svelte';
  import { developerApi, type ApiKeySummary, type ApiKeyCreated } from '$lib/api/developer';

  // ── Tab state ────────────────────────────────────────────────
  let activeTab = $state<'keys'>('keys');

  // ── API Keys state ───────────────────────────────────────────
  let keys = $state<ApiKeySummary[]>([]);
  let loadingKeys = $state(true);
  let keysError = $state<string | null>(null);

  let showKeyForm = $state(false);
  let newKeyName = $state('');
  let newKeyExpiry = $state('');
  let creatingKey = $state(false);
  let keyFormError = $state<string | null>(null);
  let justCreatedKey = $state<ApiKeyCreated | null>(null);
  let copied = $state(false);
  let revivingKey = $state<string | null>(null);
  let regeneratingKey = $state<string | null>(null);

  onMount(() => {
    loadKeys();
  });

  // ── API Key handlers ─────────────────────────────────────────

  async function loadKeys() {
    loadingKeys = true;
    keysError = null;
    try {
      if (r.error) { keysError = r.error; }
      else if (r.data) { keys = r.data; }
    } catch (e: unknown) {
      keysError = e instanceof Error ? e.message : 'Failed to load API keys';
    } finally {
      loadingKeys = false;
    }
  }

  async function handleCreateKey() {
    if (!newKeyName.trim()) { keyFormError = 'Name is required'; return; }
    creatingKey = true;
    keyFormError = null;
    justCreatedKey = null;
    try {
      if (r.error) { keyFormError = r.error; }
      else if (r.data) {
        justCreatedKey = r.data;
        newKeyName = '';
        newKeyExpiry = '';
        showKeyForm = false;
        loadKeys();
      }
    } catch (e: unknown) {
      keyFormError = e instanceof Error ? e.message : 'Failed to create API key';
    } finally {
      creatingKey = false;
    }
  }

  async function handleRevokeKey(id: string) {
    if (!confirm('Revoke this API key? This action cannot be undone.')) return;
    revivingKey = id;
    try {
      if (r.error) { keysError = r.error; }
      else { loadKeys(); }
    } catch (e: unknown) {
      keysError = e instanceof Error ? e.message : 'Failed to revoke API key';
    } finally {
      revivingKey = null;
    }
  }

  async function handleRegenerateKey(id: string) {
    if (!confirm('Regenerate this API key? The old key will stop working immediately.')) return;
    regeneratingKey = id;
    justCreatedKey = null;
    try {
      if (r.error) { keysError = r.error; }
      else if (r.data) {
        justCreatedKey = r.data;
        loadKeys();
      }
    } catch (e: unknown) {
      keysError = e instanceof Error ? e.message : 'Failed to regenerate API key';
    } finally {
      regeneratingKey = null;
    }
  }

  async function copyToClipboard(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    } catch {
      // fallback
      const ta = document.createElement('textarea');
      ta.value = text;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
      copied = true;
      setTimeout(() => { copied = false; }, 2000);
    }
  }

  function dismissKeyAlert() {
    justCreatedKey = null;
  }


  function formatDate(d: string | null): string {
    if (!d) return '—';
    return new Date(d).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="page-enter space-y-6">
  <div>
    <h2 class="text-xl font-semibold">Developer Portal</h2>
    <p class="text-sm text-[#6b7280] mt-1">Manage API keys for programmatic access and configure outgoing webhooks.</p>
  </div>

  <!-- Tabs -->
  <div class="flex gap-1 border-b border-[#1e2435]">
    <button
      onclick={() => (activeTab = 'keys')}
      class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px
        {activeTab === 'keys' ? 'text-indigo-400 border-indigo-500' : 'text-[#6b7280] border-transparent hover:text-[#d1d5db]'}"
    >
      API Keys
    </button>
    <a
      href="/settings/webhooks"
      class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px
        text-[#6b7280] border-transparent hover:text-[#d1d5db]"
    >
      Webhooks →
    </a>
  </div>

  {#if activeTab === 'keys'}
    <!-- ═══════ API KEYS TAB ═══════ -->
    <div class="page-enter space-y-6">
      {#if keysError}
        <div class="bg-[#131720] border border-red-500/30 rounded-xl p-4 text-sm text-red-400">
          {keysError}
          <button onclick={loadKeys} class="ml-2 underline">Retry</button>
        </div>
      {/if}

      {#if justCreatedKey}
        <div class="bg-indigo-500/10 border border-indigo-500/30 rounded-xl p-4 space-y-2">
          <div class="flex items-start justify-between">
            <div>
              <p class="text-sm font-medium text-indigo-400">API Key Created</p>
              <p class="text-xs text-[#6b7280] mt-1">Copy this key now — you won't be able to see it again.</p>
            </div>
            <button onclick={dismissKeyAlert} aria-label="Dismiss" class="text-[#6b7280] hover:text-white text-sm">&times;</button>
          </div>
          <div class="flex items-center gap-2">
            <code class="flex-1 px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm font-mono text-indigo-300 break-all">
              {justCreatedKey.full_key}
            </code>
            <Button size="sm" onclick={() => copyToClipboard(justCreatedKey.full_key)}>
              {copied ? 'Copied!' : 'Copy'}
            </Button>
          </div>
        </div>
      {/if}

      <!-- Create form -->
      <div>
        <Button onclick={() => { showKeyForm = !showKeyForm; justCreatedKey = null; }}>
          {showKeyForm ? 'Cancel' : 'Generate API Key'}
        </Button>
      </div>

      {#if showKeyForm}
        {#if keyFormError}
          <div class="text-sm text-red-400 mb-2">{keyFormError}</div>
        {/if}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-5 space-y-4">
          <div>
            <label for="key-name" class="text-xs text-[#6b7280] block mb-1">Key Name</label>
            <input
              id="key-name"
              type="text"
              bind:value={newKeyName}
              placeholder="e.g. CI/CD Pipeline"
              class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
            />
          </div>
          <div>
            <label for="key-expiry" class="text-xs text-[#6b7280] block mb-1">Expiry (optional — RFC 3339 format)</label>
            <input
              id="key-expiry"
              type="text"
              bind:value={newKeyExpiry}
              placeholder="e.g. 2027-01-01T00:00:00Z"
              class="w-full px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-sm focus:border-indigo-500 outline-none"
            />
          </div>
          <Button onclick={handleCreateKey} disabled={creatingKey}>
            {creatingKey ? 'Creating...' : 'Create'}
          </Button>
        </div>
      {/if}

      <!-- Key list -->
      {#if loadingKeys}
        <div class="flex justify-center py-12">
          <Spinner size="lg" />
        </div>
      {:else if keys.length === 0 && !showKeyForm}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-8 text-center">
          <p class="text-[#6b7280] text-sm">No API keys yet. Create one to use the Social Forge API.</p>
        </div>
      {:else}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl overflow-hidden">
          <table class="w-full text-sm">
            <thead>
              <tr class="border-b border-[#1e2435] text-left text-xs text-[#6b7280] uppercase tracking-wider">
                <th class="px-4 py-3 font-medium">Name</th>
                <th class="px-4 py-3 font-medium">Key</th>
                <th class="px-4 py-3 font-medium">Created</th>
                <th class="px-4 py-3 font-medium">Last Used</th>
                <th class="px-4 py-3 font-medium">Status</th>
                <th class="px-4 py-3 font-medium text-right">Actions</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-[#1e2435]">
              {#each keys as key (key.id)}
                <tr class="hover:bg-[#1a1f2e] transition-colors">
                  <td class="px-4 py-3 text-[#d1d5db]">{key.name}</td>
                  <td class="px-4 py-3 font-mono text-xs text-[#6b7280]">
                    sf_key_{key.key_prefix}...
                  </td>
                  <td class="px-4 py-3 text-[#6b7280] text-xs">{formatDate(key.created_at)}</td>
                  <td class="px-4 py-3 text-[#6b7280] text-xs">{key.last_used_at ? formatDate(key.last_used_at) : 'Never'}</td>
                  <td class="px-4 py-3">
                    {#if key.is_active}
                      <span class="px-2 py-0.5 rounded text-xs font-medium bg-green-500/20 text-green-400">Active</span>
                    {:else}
                      <span class="px-2 py-0.5 rounded text-xs font-medium bg-red-500/20 text-red-400">Revoked</span>
                    {/if}
                  </td>
                  <td class="px-4 py-3 text-right">
                    <div class="flex items-center justify-end gap-1">
                      {#if key.is_active}
                        <Button size="sm" variant="ghost" onclick={() => handleRegenerateKey(key.id)} disabled={regeneratingKey === key.id}>
                          {regeneratingKey === key.id ? '...' : 'Regen'}
                        </Button>
                        <Button size="sm" variant="ghost" onclick={() => handleRevokeKey(key.id)} disabled={revivingKey === key.id}>
                          {revivingKey === key.id ? '...' : 'Revoke'}
                        </Button>
                      {/if}
                    </div>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>

  {/if}
</div>
