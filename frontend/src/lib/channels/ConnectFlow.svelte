<script lang="ts">
  import { getAuthType } from "./auth-types";
  import ApiKeyConnect from "./ApiKeyConnect.svelte";
  import Web3Connect from "./Web3Connect.svelte";
  import ChromeExtensionConnect from "./ChromeExtensionConnect.svelte";

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

  let authType = $derived(getAuthType(provider));
</script>

{#if authType === "api_key"}
  <ApiKeyConnect {provider} {show} {onClose} {onSuccess} />
{:else if authType === "web3"}
  <Web3Connect {provider} {show} {onClose} {onSuccess} />
{:else if authType === "extension"}
  <ChromeExtensionConnect {provider} {show} {onClose} {onSuccess} />
{/if}
