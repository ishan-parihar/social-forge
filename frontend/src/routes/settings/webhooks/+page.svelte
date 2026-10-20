<script lang="ts">
  import { onMount } from "svelte";
  import { focusTrap } from '$lib/ui/focus-trap';
  import Skeleton from '$lib/ui/Skeleton.svelte';
  import { webhooksApi, type Webhook, type WebhookDelivery } from "$lib/api/webhooks";
  import { toast } from "$lib/stores/toast";
  import { modals } from '$lib/stores/modals.svelte';

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
      webhooks = r.data;
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
    if (!(await modals.areYouSure({
      title: 'Delete this webhook?',
      message: 'The webhook will be permanently deleted. Any pending deliveries will be cancelled.',
      confirmLabel: 'Delete',
      cancelLabel: 'Cancel',
      danger: true,
    }))) return;
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
      deliveries = r.data;
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
  <div class="flex flex-wrap items-center justify-between gap-3">
    <h1 class="text-xl font-semibold">Webhooks</h1>
    <button onclick={openCreate} class="px-4 py-2 bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg text-sm transition-colors min-h-11">
      + Add Webhook
    </button>
  </div>

  {#if loading}
    <Skeleton variant="row" rows={4} />
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
                <span class="px-2 py-0.5 text-xs rounded bg-success/20 ring-1 ring-inset ring-success/40 text-content">Active</span>
              {:else}
                <span class="px-2 py-0.5 text-xs rounded bg-hue-neutral/20 text-muted">Inactive</span>
              {/if}
            </div>
            <div class="flex gap-2">
              <button onclick={() => testWebhook(wh.id)} class="text-xs px-2 py-1 text-muted hover:text-accent border border min-h-11 min-w-11 inline-flex items-center justify-center-line rounded min-h-11">Test</button>
              <button onclick={() => viewDeliveries(wh.id)} class="text-xs px-2 py-1 text-muted hover:text-accent border border min-h-11 min-w-11 inline-flex items-center justify-center-line rounded min-h-11">Deliveries</button>
              <button onclick={() => openEdit(wh)} class="text-xs px-2 py-1 text-muted hover:text-accent border border min-h-11 min-w-11 inline-flex items-center justify-center-line rounded min-h-11">Edit</button>
              <button onclick={() => deleteWebhook(wh.id)} class="text-xs px-2 py-1 text-muted hover:text-error border border-line rounded min-h-11">Delete</button>
            </div>
          </div>
          <p class="text-xs text-muted truncate mb-2">{wh.url}</p>
          <div class="flex gap-1 flex-wrap">
            {#each wh.event_types as et}
              <span class="px-2 py-0.5 text-[10px] rounded bg-accent-fill/10 text-accent border border-accent/20">{et}</span>
            {/each}
          </div>
          {#if wh.last_triggered_at}
            <p class="text-[10px] text-faint mt-2">Last triggered: {new Date(wh.last_triggered_at).toLocaleString()}</p>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Create/Edit Modal -->
{#if showModal}
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <!--
  The scrim is a separate, aria-hidden BUTTON and the panel is its
  SIBLING.

  The wrapper used to carry `aria-hidden="true"`, which made the whole
  dialog invisible to a screen reader, and the shared focus trap skips
  anything in an aria-hidden subtree - so it found no focusable child
  and the focus never entered the dialog. See AGENTS.md 9.39.

  The scrim carries the aria-hidden and the tabindex="-1": a screen
  reader should not offer a nameless "close" button, and Escape
  already closes the dialog.
-->
    <button
      type="button"
      class="absolute inset-0 bg-overlay cursor-default"
      aria-hidden="true"
      tabindex="-1"
      onclick={() => { showModal = false }}
    ></button>
<div class="relative bg-background-input border border-line rounded-xl p-6 w-full max-w-md"
      role="dialog"
      aria-modal="true"
      aria-labelledby="webhook-title"
      tabindex="-1"
      onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); showModal = false; } }}
      use:focusTrap
    >
      <h2 id="webhook-title" class="text-lg font-semibold mb-4">{editingWebhook ? "Edit Webhook" : "New Webhook"}</h2>
      <div class="page-enter space-y-6">
        <div>
          <label for="webhook-name" class="text-xs text-muted mb-1 block">Name</label>
          <input id="webhook-name" bind:value={formName} placeholder="My Webhook" class="min-h-11 w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm" />
        </div>
        <div>
          <label for="webhook-url" class="text-xs text-muted mb-1 block">URL</label>
          <input id="webhook-url" bind:value={formUrl} placeholder="https://example.com/webhook" class="min-h-11 w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm" />
        </div>
        <div>
          <label for="webhook-secret" class="text-xs text-muted mb-1 block">Secret (optional)</label>
          <input id="webhook-secret" bind:value={formSecret} type="password" placeholder="Webhook signing secret" class="min-h-11 w-full px-3 py-2 bg-surface-hover border border-line rounded text-sm" />
        </div>
        <div>
          <span class="text-xs text-muted mb-2 block">Event Types</span>
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
          <input type="checkbox" bind:checked={formActive} class="min-h-11 rounded" />
          <span>Active</span>
        </label>
      </div>
      <div class="flex gap-3 justify-end mt-6">
        <button onclick={() => showModal = null} class="px-4 py-2 text-sm text-muted hover:text-content min-h-11">Cancel</button>
        <button onclick={saveWebhook} disabled={saving || !formName.trim() || !formUrl.trim()} class="px-4 py-2 text-sm bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded disabled:opacity-50 min-h-11">
          {saving ? "Saving..." : "Save"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Deliveries Modal -->
{#if showDeliveries}
  <div class="fixed inset-0 z-50 flex items-center justify-center p-4">
    <!--
      The scrim is a separate, aria-hidden BUTTON and the panel is its
      SIBLING.

      The wrapper used to carry `aria-hidden="true"`, which made the whole
      dialog invisible to a screen reader, and the shared focus trap skips
      anything in an aria-hidden subtree - so it found no focusable child
      and the focus never entered the dialog. See AGENTS.md 9.39.

      The scrim carries the aria-hidden and the tabindex="-1": a screen
      reader should not offer a nameless "close" button, and Escape
      already closes the dialog.
    -->
    <button
      type="button"
      class="absolute inset-0 bg-overlay cursor-default"
      aria-hidden="true"
      tabindex="-1"
      onclick={() => { showDeliveries = null }}
    ></button>
    <div class="relative bg-background-input border border-line rounded-xl p-6 w-full max-w-2xl max-h-[80vh] overflow-y-auto"
      role="dialog"
      aria-modal="true"
      aria-labelledby="webhook-deliveries-title"
      tabindex="-1"
      onkeydown={(e) => { if (e.key === "Escape") { e.stopPropagation(); showDeliveries = null; } }}
      use:focusTrap
    >
      <div class="flex items-center justify-between mb-4">
        <h2 class="text-lg font-semibold">Delivery History</h2>
        <button onclick={() => showDeliveries = null} class="text-muted hover:text-content min-h-11">✕</button>
      </div>
      {#if loadingDeliveries}
        <Skeleton variant="row" rows={3} />
      {:else if deliveries.length === 0}
        <div class="text-center py-8 text-sm text-muted">No deliveries yet</div>
      {:else}
        <div class="page-enter space-y-2">
          {#each deliveries as d (d.id)}
            <div class="bg-surface border border-line rounded-lg p-3">
              <div class="flex items-center justify-between mb-1">
                <span class="text-xs font-medium px-2 py-0.5 rounded {d.status_code && d.status_code >= 200 && d.status_code < 300 ? 'bg-success/20 ring-1 ring-inset ring-success/40 text-content' : 'bg-error/20 ring-1 ring-inset ring-error/40 text-content'}">{d.status_code || 'Pending'}</span>
                <span class="text-xs text-muted">{d.event_type}</span>
              </div>
              <p class="text-xs text-muted">{new Date(d.attempted_at).toLocaleString()}</p>
              {#if d.response_body}
                <p class="text-xs text-faint mt-1 truncate">Response: {d.response_body}</p>
              {/if}
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}
