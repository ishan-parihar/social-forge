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

  let address = $state("");
  let label = $state("");
  let submitting = $state(false);
  let errorMsg = $state("");

  const providerInstructions: Record<string, string> = {
    farcaster: "Enter your Farcaster custody address or signer public key.",
    nostr: "Enter your Nostr npub (public key) to connect your identity.",
  };

  const placeholderText: Record<string, string> = {
    farcaster: "0x... or FID",
    nostr: "npub...",
  };

  let instructions = $derived(providerInstructions[provider] ?? "Enter your Web3 address or public key.");
  let placeholder = $derived(placeholderText[provider] ?? "Address or public key");

  function reset() {
    address = "";
    label = "";
    errorMsg = "";
    submitting = false;
  }

  async function handleSubmit() {
    if (!address.trim()) {
      errorMsg = "Address is required";
      return;
    }
    submitting = true;
    errorMsg = "";
    try {
      const body: import("$lib/api/integrations").ConnectWeb3Request = {
        provider,
        address: address.trim(),
        label: label.trim() || undefined,
      };
      const r = await integrationsApi.connectWeb3(body);
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

<Modal open={show} title="Connect via Web3" onclose={handleClose}>
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

    <div class="text-xs text-muted bg-background-input rounded-lg px-3 py-2 border border-line">
      {instructions}
    </div>

    <div>
      <label for="address" class="block text-xs font-medium text-muted mb-1">
        Address / Public Key <span class="text-error">*</span>
      </label>
      <input
        id="address"
        type="text"
        bind:value={address}
        {placeholder}
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
        placeholder="e.g. My {provider} identity"
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
