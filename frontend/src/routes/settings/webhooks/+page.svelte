<script lang="ts">
  import { onMount } from "svelte";
  import { webhooksApi, type Webhook, type WebhookDelivery } from "$lib/api/webhooks";
  import { toast } from "$lib/stores/toast";
  import SettingsBreadcrumb from "$lib/settings/SettingsBreadcrumb.svelte";

  let webhooks = $state<Webhook[]>([]);
  let loading = $state(true);
  let showModal = $state(false);
  let editingWebhook = $state<Webhook | null>(null);
  let saving = $state(false);
  let showDeliveries = $state<string | null>(null);
  let deliveries = $state<WebhookDelivery[]>([]);
  let loadingDeliveries = $state(false);

  // Form state
  let formName = $state("");
  let formUrl = $state("");
  let formSecret = $state("");
  let formEventTypes = $state<string[]>(["post.published"]);
  let formActive = $state(true);

  const availableEventTypes = [
    "post.created",
    "post.scheduled",
    "post.published",
    "post.failed",
    "integration.connected",
    "integration.disconnected",
    "comment.received",
    "dm.received",
  ];

  async function load() {
    loading = true;
    const r = await webhooksApi.list();
    if (r.data) {
      webhooks = r.data.webhooks;
    } else if (r.error) {
      toast(`Failed to load webhooks: ${r.error}`, "error");
    }
    loading = false;
  }

  function openCreate() {
    editingWebhook = null;
    formName = "";
    formUrl = "";
    formSecret = "";
    formEventTypes = ["post.published"];
    formActive = true;
    showModal = true;
  }

  function openEdit(wh: Webhook) {
    editingWebhook = wh;
    formName = wh.name;
    formUrl = wh.url;
    formSecret = wh.secret || "";
    formEventTypes = wh.event_types;
    formActive = wh.is_active;
    showModal = true;
  }

  async function saveWebhook() {
    if (!formName.trim() || !formUrl.trim()) return;
    saving = true;
    const body = {
      name: formName,
      url: formUrl,
      ...(formSecret.trim() && { secret: formSecret }),
      event_types: formEventTypes,
      is_active: formActive,
    };
    const r = editingWebhook
      ? await webhooksApi.update(editingWebhook.id, body)
      : await webhooksApi.create(body);
    if (r.error) {
      toast(`Failed to save: ${r.error}`, "error");
    } else {
      toast(editingWebhook ? "Webhook updated" : "Webhook created", "success");
      showModal = false;
      await load();
    }
    saving = false;
  }

  async function deleteWebhook(id: string) {
    if (!confirm("Delete this webhook?")) return;
    const r = await webhooksApi.delete(id);
    if (r.error) {
      toast(`Delete failed: ${r.error}`, "error");
    } else {
      toast("Webhook deleted", "success");
      await load();
    }
  }

  async function testWebhook(id: string) {
    const r = await webhooksApi.test(id);
    if (r.error) {
      toast(`Test failed: ${r.error}`, "error");
    } else {
      toast("Test delivery sent", "success");
    }
  }

  async function viewDeliveries(id: string) {
    showDeliveries = id;
    loadingDeliveries = true;
    const r = await webhooksApi.deliveries(id);
    if (r.data) {
      deliveries = r.data.deliveries;
    } else if (r.error) {
      toast(`Failed to load deliveries: ${r.error}`, "error");
    }
    loadingDeliveries = false;
  }

  function toggleEventType(type: string) {
    if (formEventTypes.includes(type)) {
      formEventTypes = formEventTypes.filter(t => t !== type);
    } else {
      formEventTypes = [...formEventTypes, type];
    }
  }

  onMount(load);
</script>

<div class="page-enter space-y-6">
  <SettingsBreadcrumb title="Webhooks" />
  <div class="flex items-center justify-between">
    <h2 class="text-xl font-semibold">Webhooks</h2>
    <button onclick={openCreate} class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-sm transition-colors">
      + Add Webhook
    </button>
  </div>

  {#if loading}
    <div class="text-center py-12 text-sm text-muted">Loading...</div>
  {:else if webhooks.length === 0}
    <div class="text-center py-12 text-sm text-muted">
      No webhooks configured. Create one to receive real-time event notifications.
    </div>
  {:else}
    <div class="page-enter space-y-3">
      {#each webhooks as wh (wh.id)}
        <div class="bg-surface border border-line rounded-xl p-4">
          <div class="flex items-center justify-between mb-2">
            <div class="flex items-center gap-2">
              <span class="font-medium text-sm">{wh.name}</span>
              {#if wh.is_active}
                <span class="px-2 py-0.5 text-xs rounded bg-green-500/20 text-green-400">Active</span>
              {:else}
                <span class="px-2 py-0.5 text-xs rounded bg-gray-500/20 text-gray-400">Inactive</span>
              {/if}
            </div>
            <div class="flex gap-2">
              <button onclick={() => testWebhook(wh.id)} class="text-xs px-2 py-1 text-muted hover:text-indigo-400 border border-line rounded">Test</button>
              <button onclick={() => viewDeliveries(wh.id)} class="text-xs px-2 py-1 text-muted hover:text-indigo-400 border border-line rounded">Deliveries</button>
              <button onclick={() => openEdit(wh)} class="text-xs px-2 py-1 text-muted hover:text-indigo-400 border border-line rounded">Edit</button>
              <button onclick={() => deleteWebhook(wh.id)} class="text-xs px-2 py-1 text-muted hover:text-red-400 border border-line rounded">Delete</button>
            </div>
          </div>
          <p class="text-xs text-muted truncate mb-2">{wh.url}</p>
          <div class="flex gap-1 flex-wrap">
            {#each wh.event_types as et}
              <span class="px-2 py-0.5 text-[10px] rounded bg-indigo-500/10 text-indigo-400 border border-indigo-500/20">{et}</span>
            {/each}
          </div>
          {#if wh.last_triggered_at}
            <p class="text-[10px] text-muted-dark mt-2">Last triggered: {new Date(wh.last_triggered_at).toLocaleString()}</p>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Create/Edit Modal -->
{#if showModal}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-md">
      <h3 class="text-lg font-semibold mb-4">{editingWebhook ? "Edit Webhook" : "New Webhook"}</h3>
      <div class="page-enter space-y-6">
        <div>
          <label class="text-xs text-muted mb-1 block">Name</label>
          <input bind:value={formName} placeholder="My Webhook" class="w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm" />
        </div>
        <div>
          <label class="text-xs text-muted mb-1 block">URL</label>
          <input bind:value={formUrl} placeholder="https://example.com/webhook" class="w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm" />
        </div>
        <div>
          <label class="text-xs text-muted mb-1 block">Secret (optional)</label>
          <input bind:value={formSecret} type="password" placeholder="Webhook signing secret" class="w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm" />
        </div>
        <div>
          <label class="text-xs text-muted mb-2 block">Event Types</label>
          <div class="page-enter space-y-1">
            {#each availableEventTypes as et}
              <label class="flex items-center gap-2 text-sm cursor-pointer">
                <input type="checkbox" checked={formEventTypes.includes(et)} onchange={() => toggleEventType(et)} class="rounded" />
                <span>{et}</span>
              </label>
            {/each}
          </div>
        </div>
        <label class="flex items-center gap-2 text-sm cursor-pointer">
          <input type="checkbox" bind:checked={formActive} class="rounded" />
          <span>Active</span>
        </label>
      </div>
      <div class="flex gap-3 justify-end mt-6">
        <button onclick={() => showModal = null} class="px-4 py-2 text-sm text-muted hover:text-white">Cancel</button>
        <button onclick={saveWebhook} disabled={saving || !formName.trim() || !formUrl.trim()} class="px-4 py-2 text-sm bg-indigo-600 hover:bg-indigo-500 rounded disabled:opacity-50">
          {saving ? "Saving..." : "Save"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Deliveries Modal -->
{#if showDeliveries}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50" role="dialog">
    <div class="bg-background-input border border-line rounded-xl p-6 w-full max-w-2xl max-h-[80vh] overflow-y-auto">
      <div class="flex items-center justify-between mb-4">
        <h3 class="text-lg font-semibold">Delivery History</h3>
        <button onclick={() => showDeliveries = null} class="text-muted hover:text-white">✕</button>
      </div>
      {#if loadingDeliveries}
        <div class="text-center py-8 text-sm text-muted">Loading...</div>
      {:else if deliveries.length === 0}
        <div class="text-center py-8 text-sm text-muted">No deliveries yet</div>
      {:else}
        <div class="page-enter space-y-2">
          {#each deliveries as d (d.id)}
            <div class="bg-surface border border-line rounded-lg p-3">
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs font-medium px-2 py-0.5 rounded {d.status_code && d.status_code >= 200 && d.status_code < 300 ? 'bg-green-500/20 text-green-400' : 'bg-red-500/20 text-red-400'}">{d.status_code || 'Pending'}</span>
                <span class="text-xs text-muted">{d.event_type}</span>
              </div>
              <p class="text-xs text-muted">{new Date(d.attempted_at).toLocaleString()}</p>
              {#if d.response_body}
                <p class="text-xs text-muted-dark mt-1 truncate">Response: {d.response_body}</p>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}
