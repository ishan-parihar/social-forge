<script lang="ts">
  import { integrationsApi } from "$lib/api/integrations";
  import Modal from "$lib/ui/Modal.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Spinner from "$lib/ui/Spinner.svelte";

  let {
    provider = "",
    show = false,
    onClose,
    onSuccess,
  }: {
    provider?: string;
    show?: boolean;
    onClose?: () => void;
    onSuccess?: () => void;
  } = $props();

  let apiKey = $state("");
  let instanceUrl = $state("");
  let label = $state("");
  let submitting = $state(false);
  let errorMsg = $state("");

  function reset() {
    apiKey = "";
    instanceUrl = "";
    label = "";
    errorMsg = "";
    submitting = false;
  }

  async function handleSubmit() {
    if (!apiKey.trim()) {
      errorMsg = "API key is required";
      return;
    }
    submitting = true;
    errorMsg = "";
    try {
      const body: import("$lib/api/integrations").ConnectApiKeyRequest = {
        provider,
        api_key: apiKey.trim(),
        instance_url: instanceUrl.trim() || undefined,
        label: label.trim() || undefined,
      };
      const r = await integrationsApi.connectApiKey(body);
      if (r.error) {
        errorMsg = r.error;
      } else {
        reset();
        onSuccess?.();
      }
    } catch (e: unknown) {
      errorMsg = e instanceof Error ? e.message : "Connection failed";
    }
    submitting = false;
  }

  function handleClose() {
    if (!submitting) {
      reset();
      onClose?.();
    }
  }
</script>

<Modal open={show} title="Connect via API Key" onclose={handleClose}>
  <form onsubmit={handleSubmit} class="space-y-4">
    <div>
      <label for="connect-provider" class="block text-xs font-medium text-muted mb-1">Provider</label>
      <input
        id="connect-provider"
        type="text"
        value={provider}
        disabled
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-muted cursor-not-allowed"
      />
    </div>

    <div>
      <label for="api-key" class="block text-xs font-medium text-muted mb-1">
        API Key <span class="text-error">*</span>
      </label>
      <input
        id="api-key"
        type="password"
        bind:value={apiKey}
        placeholder="Enter your API key"
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-white placeholder:text-muted-dark focus:outline-none focus:border-brand-500 transition-colors"
      />
    </div>

    <div>
      <label for="instance-url" class="block text-xs font-medium text-muted mb-1">
        Instance URL <span class="text-muted-dark">(optional)</span>
      </label>
      <input
        id="instance-url"
        type="text"
        bind:value={instanceUrl}
        placeholder="https://lemmy.world"
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-white placeholder:text-muted-dark focus:outline-none focus:border-brand-500 transition-colors"
      />
    </div>

    <div>
      <label for="label" class="block text-xs font-medium text-muted mb-1">
        Label <span class="text-muted-dark">(optional)</span>
      </label>
      <input
        id="label"
        type="text"
        bind:value={label}
        placeholder="e.g. My Lemmy Account"
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-white placeholder:text-muted-dark focus:outline-none focus:border-brand-500 transition-colors"
      />
    </div>

    {#if errorMsg}
      <div class="text-sm text-error bg-error/10 border border-error/20 rounded-lg px-3 py-2">
        {errorMsg}
      </div>
    {/if}

    <div class="flex justify-end gap-2 pt-2">
      <Button variant="secondary" onclick={handleClose} disabled={submitting}>Cancel</Button>
      <Button variant="primary" onclick={handleSubmit} disabled={submitting}>
        {#if submitting}
          <span class="flex items-center gap-2">
            <Spinner size="sm" /> Connecting...
          </span>
        {:else}
          Connect
        {/if}
      </Button>
    </div>
  </form>
</Modal>
