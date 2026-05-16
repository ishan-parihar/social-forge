<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/ui/Button.svelte';
  import Modal from '$lib/ui/Modal.svelte';
  import Spinner from '$lib/ui/Spinner.svelte';
  import WebhookForm from '$lib/developer/WebhookForm.svelte';
  import { developerApi, type ApiKeySummary, type ApiKeyCreated, type Webhook, type WebhookDelivery } from '$lib/api/developer';

  // ── Tab state ────────────────────────────────────────────────
  let activeTab = $state<'keys' | 'webhooks'>('keys');

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

  // ── Webhooks state ──────────────────────────────────────────
  let webhooks = $state<Webhook[]>([]);
  let loadingWebhooks = $state(true);
  let webhooksError = $state<string | null>(null);

  let showWebhookForm = $state(false);
  let editWebhook = $state<Webhook | null>(null);
  let savingWebhook = $state(false);
  let deletingWebhookId = $state<string | null>(null);

  let testingWebhookId = $state<string | null>(null);
  let testResult = $state<{ status_code: number; response_body: string } | null>(null);

  let viewingDeliveries = $state<string | null>(null);
  let deliveries = $state<WebhookDelivery[]>([]);
  let loadingDeliveries = $state(false);

  onMount(() => {
    loadKeys();
    loadWebhooks();
  });

  // ── API Key handlers ─────────────────────────────────────────

  async function loadKeys() {
    loadingKeys = true;
    keysError = null;
    try {
      const r = await developerApi.listKeys();
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
      const r = await developerApi.createKey(newKeyName.trim(), newKeyExpiry || undefined);
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
      const r = await developerApi.revokeKey(id);
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
      const r = await developerApi.regenerateKey(id);
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

  // ── Webhook handlers ─────────────────────────────────────────

  async function loadWebhooks() {
    loadingWebhooks = true;
    webhooksError = null;
    try {
      const r = await developerApi.listWebhooks();
      if (r.error) { webhooksError = r.error; }
      else if (r.data) { webhooks = r.data; }
    } catch (e: unknown) {
      webhooksError = e instanceof Error ? e.message : 'Failed to load webhooks';
    } finally {
      loadingWebhooks = false;
    }
  }

  function openCreateWebhook() {
    editWebhook = null;
    showWebhookForm = true;
  }

  function openEditWebhook(wh: Webhook) {
    editWebhook = wh;
    showWebhookForm = true;
  }

  function closeWebhookForm() {
    showWebhookForm = false;
    editWebhook = null;
  }

  async function handleWebhookSave(data: { name: string; url: string; secret?: string; event_types: string[]; is_active?: boolean }) {
    savingWebhook = true;
    try {
      if (editWebhook) {
        await developerApi.updateWebhook(editWebhook.id, data);
      } else {
        await developerApi.createWebhook(data);
      }
      closeWebhookForm();
      loadWebhooks();
    } catch (e: unknown) {
      webhooksError = e instanceof Error ? e.message : 'Failed to save webhook';
    } finally {
      savingWebhook = false;
    }
  }

  async function handleDeleteWebhook(id: string) {
    if (!confirm('Delete this webhook permanently?')) return;
    deletingWebhookId = id;
    try {
      const r = await developerApi.deleteWebhook(id);
      if (r.error) { webhooksError = r.error; }
      else { loadWebhooks(); }
    } catch (e: unknown) {
      webhooksError = e instanceof Error ? e.message : 'Failed to delete webhook';
    } finally {
      deletingWebhookId = null;
    }
  }

  async function handleTestWebhook(id: string) {
    testingWebhookId = id;
    testResult = null;
    try {
      const r = await developerApi.testWebhook(id);
      if (r.error) { webhooksError = r.error; }
      else if (r.data) {
        testResult = { status_code: r.data.status_code, response_body: r.data.response_body };
      }
    } catch (e: unknown) {
      webhooksError = e instanceof Error ? e.message : 'Failed to test webhook';
    } finally {
      testingWebhookId = null;
    }
  }

  async function handleViewDeliveries(id: string) {
    viewingDeliveries = viewingDeliveries === id ? null : id;
    if (viewingDeliveries === id) {
      loadingDeliveries = true;
      deliveries = [];
      try {
        const r = await developerApi.getDeliveries(id);
        if (r.error) { webhooksError = r.error; }
        else if (r.data) { deliveries = r.data; }
      } catch (e: unknown) {
        webhooksError = e instanceof Error ? e.message : 'Failed to load deliveries';
      } finally {
        loadingDeliveries = false;
      }
    }
  }

  function formatDate(d: string | null): string {
    if (!d) return '—';
    return new Date(d).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }
</script>

<div class="space-y-6">
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
    <button
      onclick={() => (activeTab = 'webhooks')}
      class="px-4 py-2 text-sm font-medium transition-colors border-b-2 -mb-px
        {activeTab === 'webhooks' ? 'text-indigo-400 border-indigo-500' : 'text-[#6b7280] border-transparent hover:text-[#d1d5db]'}"
    >
      Webhooks
    </button>
  </div>

  {#if activeTab === 'keys'}
    <!-- ═══════ API KEYS TAB ═══════ -->
    <div class="space-y-4">
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

  {:else if activeTab === 'webhooks'}
    <!-- ═══════ WEBHOOKS TAB ═══════ -->
    <div class="space-y-4">
      {#if webhooksError}
        <div class="bg-[#131720] border border-red-500/30 rounded-xl p-4 text-sm text-red-400">
          {webhooksError}
          <button onclick={loadWebhooks} class="ml-2 underline">Retry</button>
        </div>
      {/if}

      <div>
        <Button onclick={openCreateWebhook}>
          Create Webhook
        </Button>
      </div>

      <!-- Webhook form modal -->
      <Modal open={showWebhookForm} title={editWebhook ? 'Edit Webhook' : 'Create Webhook'} onclose={closeWebhookForm}>
        <WebhookForm
          webhook={editWebhook ?? undefined}
          onSave={handleWebhookSave}
          onCancel={closeWebhookForm}
        />
        {#if savingWebhook}
          <div class="flex justify-center py-4">
            <Spinner size="md" />
          </div>
        {/if}
      </Modal>

      <!-- Test result modal -->
      <Modal open={testResult !== null} title="Webhook Test Result" onclose={() => (testResult = null)}>
        {#if testResult}
          <div class="space-y-3">
            <div>
              <span class="text-xs text-[#6b7280] block mb-1">Status Code</span>
              <span class="px-2 py-0.5 rounded text-xs font-medium {testResult.status_code >= 200 && testResult.status_code < 300 ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'}">
                {testResult.status_code}
              </span>
            </div>
            <div>
              <span class="text-xs text-[#6b7280] block mb-1">Response Body</span>
              <pre class="px-3 py-2 bg-[#0d1117] border border-[#1e2435] rounded-lg text-xs font-mono text-[#d1d5db] max-h-48 overflow-auto whitespace-pre-wrap">{testResult.response_body || '(empty)'}</pre>
            </div>
            <Button variant="ghost" onclick={() => (testResult = null)}>Close</Button>
          </div>
        {/if}
      </Modal>

      <!-- Webhook list -->
      {#if loadingWebhooks}
        <div class="flex justify-center py-12">
          <Spinner size="lg" />
        </div>
      {:else if webhooks.length === 0}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-8 text-center">
          <p class="text-[#6b7280] text-sm">No webhooks configured.</p>
        </div>
      {:else}
        <div class="space-y-3">
          {#each webhooks as wh (wh.id)}
            <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4 space-y-3">
              <div class="flex items-start justify-between gap-3">
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-2">
                    <h4 class="text-sm font-medium">{wh.name}</h4>
                    {#if wh.is_active}
                      <span class="px-2 py-0.5 rounded text-xs font-medium bg-green-500/20 text-green-400">Active</span>
                    {:else}
                      <span class="px-2 py-0.5 rounded text-xs font-medium bg-gray-500/20 text-gray-400">Disabled</span>
                    {/if}
                  </div>
                  <p class="text-xs text-[#6b7280] mt-1 font-mono truncate">{wh.url}</p>
                </div>
                <div class="flex gap-1 flex-shrink-0">
                  <Button size="sm" variant="ghost" onclick={() => openEditWebhook(wh)}>Edit</Button>
                  <Button size="sm" variant="ghost" onclick={() => handleDeleteWebhook(wh.id)} disabled={deletingWebhookId === wh.id}>
                    {deletingWebhookId === wh.id ? '...' : 'Delete'}
                  </Button>
                </div>
              </div>

              <!-- Event types -->
              <div class="flex flex-wrap gap-1.5">
                {#each wh.event_types as et (et)}
                  <span class="px-2 py-0.5 rounded text-xs font-medium bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">{et}</span>
                {/each}
              </div>

              <!-- Meta + actions -->
              <div class="flex items-center justify-between text-xs text-[#6b7280]">
                <span>Created {formatDate(wh.created_at)}{#if wh.last_triggered_at} &middot; Last triggered {formatDate(wh.last_triggered_at)}{/if}</span>
                <div class="flex gap-2">
                  <button onclick={() => handleTestWebhook(wh.id)} disabled={testingWebhookId === wh.id} class="hover:text-indigo-400 transition-colors">
                    {testingWebhookId === wh.id ? 'Testing...' : 'Test Delivery'}
                  </button>
                  <button onclick={() => handleViewDeliveries(wh.id)} class="hover:text-indigo-400 transition-colors">
                    {viewingDeliveries === wh.id ? 'Hide Deliveries' : 'View Deliveries'}
                  </button>
                </div>
              </div>

              <!-- Deliveries inline -->
              {#if viewingDeliveries === wh.id}
                <div class="border-t border-[#1e2435] pt-3">
                  {#if loadingDeliveries}
                    <div class="flex justify-center py-4">
                      <Spinner size="md" />
                    </div>
                  {:else if deliveries.length === 0}
                    <p class="text-xs text-[#6b7280] text-center py-2">No deliveries yet.</p>
                  {:else}
                    <div class="space-y-2">
                      {#each deliveries as d (d.id)}
                        <div class="bg-[#0d1117] border border-[#1e2435] rounded-lg p-3 text-xs">
                          <div class="flex items-center justify-between mb-1">
                            <span class="text-[#6b7280]">{d.event_type}</span>
                            <div class="flex items-center gap-2">
                              {#if d.status_code}
                                <span class="text-[#6b7280]">HTTP {d.status_code}</span>
                              {/if}
                              {#if d.status === 'delivered'}
                                <span class="px-1.5 py-0.5 rounded text-xs font-medium bg-green-500/20 text-green-400">Delivered</span>
                              {:else if d.status === 'failed'}
                                <span class="px-1.5 py-0.5 rounded text-xs font-medium bg-red-500/20 text-red-400">Failed</span>
                              {:else}
                                <span class="px-1.5 py-0.5 rounded text-xs font-medium bg-yellow-500/20 text-yellow-400">{d.status}</span>
                              {/if}
                            </div>
                          </div>
                          <div class="flex justify-between text-[#4b5563]">
                            <span>{formatDate(d.attempted_at)}</span>
                            {#if d.delivered_at}
                              <span>Delivered {formatDate(d.delivered_at)}</span>
                            {/if}
                          </div>
                          {#if d.response_body}
                            <details class="mt-1">
                              <summary class="text-[#6b7280] cursor-pointer hover:text-[#d1d5db]">Response body</summary>
                              <pre class="mt-1 px-2 py-1 bg-[#131720] rounded text-xs font-mono text-[#d1d5db] max-h-24 overflow-auto whitespace-pre-wrap">{d.response_body}</pre>
                            </details>
                          {/if}
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>
