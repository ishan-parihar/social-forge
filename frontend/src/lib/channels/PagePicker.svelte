<script lang="ts">
  import { integrationsApi, type PageInfo } from "$lib/api/integrations";
  import Modal from "$lib/ui/Modal.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";

  let {
    provider = "",
    integrationId = "",
    show = false,
    onClose,
    onSuccess,
  }: {
    provider?: string;
    integrationId?: string;
    show?: boolean;
    onClose?: () => void;
    onSuccess?: () => void;
  } = $props();

  let pages = $state<PageInfo[]>([]);
  let selected = $state<Set<string>>(new Set());
  let loading = $state(true);
  let connecting = $state(false);
  let errorMsg = $state("");
  let connectedCount = $state(0);

  async function loadPages() {
    if (!integrationId) return;
    loading = true;
    errorMsg = "";
    const r = await integrationsApi.availablePages(integrationId);
    if (r.error) {
      errorMsg = r.error;
    } else if (r.data) {
      pages = r.data.pages;
    }
    loading = false;
  }

  function togglePage(id: string) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    selected = next;
  }

  async function connectSelected() {
    if (selected.size === 0) return;
    connecting = true;
    errorMsg = "";
    connectedCount = 0;
    const toConnect = [...selected];
    for (const pageId of toConnect) {
      const r = await integrationsApi.connectPage(integrationId, pageId);
      if (r.error) {
        errorMsg = `Failed to connect ${pageId}: ${r.error}`;
        break;
      }
      connectedCount++;
    }
    connecting = false;
    if (!errorMsg) {
      selected = new Set();
      onSuccess?.();
    }
  }

  function handleClose() {
    if (!connecting) {
      onClose?.();
    }
  }

  $effect(() => {
    if (show && integrationId) loadPages();
  });
</script>

<Modal open={show} title="Select {provider} Pages to Connect" onclose={handleClose}>
  <div class="space-y-4">
    {#if loading}
      <div class="flex items-center justify-center py-8">
        <Spinner size="md" />
        <span class="ml-3 text-sm text-muted">Loading pages...</span>
      </div>
    {:else if pages.length === 0}
      <div class="text-center text-sm text-muted py-8">
        No pages found. Make sure you have admin access to at least one page.
      </div>
    {:else}
      <p class="text-xs text-muted">Select the pages you want to connect. You can always add more later.</p>
      <div class="space-y-2 max-h-64 overflow-y-auto">
        {#each pages as page (page.id)}
          <label class="flex items-center gap-3 p-3 bg-background-input border border-line rounded-lg cursor-pointer hover:border-brand-500/50 transition-colors">
            <input
              type="checkbox"
              checked={selected.has(page.id)}
              onchange={() => togglePage(page.id)}
              class="w-4 h-4 rounded border-line bg-surface-hover text-brand-500 focus:ring-brand-500 focus:ring-offset-0"
            />
            {#if page.picture}
              <img src={page.picture} alt="" class="w-8 h-8 rounded-full" />
            {:else}
              <div class="w-8 h-8 rounded-full bg-surface-hover flex items-center justify-center text-xs">📄</div>
            {/if}
            <div class="flex-1 min-w-0">
              <div class="text-sm text-white truncate">{page.name}</div>
              {#if page.username}
                <div class="text-xs text-muted">@{page.username}</div>
              {/if}
            </div>
          </label>
        {/each}
      </div>
    {/if}

    {#if errorMsg}
      <div class="text-sm text-error bg-error/10 border border-error/20 rounded-lg px-3 py-2">
        {errorMsg}
      </div>
    {/if}

    {#if connectedCount > 0}
      <div class="text-sm text-success">
        Connected {connectedCount} page{connectedCount > 1 ? 's' : ''}!
      </div>
    {/if}

    <div class="flex justify-end gap-2 pt-2">
      <Button variant="secondary" onclick={handleClose} disabled={connecting}>
        {#if connectedCount > 0}Done{:else}Cancel{/if}
      </Button>
      <Button variant="primary" onclick={connectSelected} disabled={connecting || selected.size === 0}>
        {#if connecting}
          <span class="flex items-center gap-2">
            <Spinner size="sm" /> Connecting...
          </span>
        {:else}
          Connect {selected.size} Page{selected.size !== 1 ? 's' : ''}
        {/if}
      </Button>
    </div>
  </div>
</Modal>
