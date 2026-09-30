<script lang="ts">
  // v25 F4 — the single entry point for connecting a channel.
  //
  // Before F4 there were four separate implementations of "connect a channel"
  // across two files: an inline OAuth popup branch, a hand-rolled
  // `connectChoice` dialog and a `credDialog` in routes/channels/+page.svelte,
  // and this component (which only handled api_key / web3 / extension). A
  // provider's connect path therefore depended on which code path reached it,
  // and three of the four bypassed the shared Modal primitive: no Escape, no
  // focus trap, no aria-modal, and a hardcoded `bg-black/60` scrim that the F1
  // `overlay` token exists to replace.
  //
  // This is now the only path. `connectMethodsFor()` decides the list,
  // MethodPicker asks the user when there is a real choice, and each method
  // has exactly one component.
  import { connectMethodsFor, type AuthType } from "./auth-types";
  import MethodPicker from "./MethodPicker.svelte";
  import OAuthConnect from "./OAuthConnect.svelte";
  import CookieConnect from "./CookieConnect.svelte";
  import ApiKeyConnect from "./ApiKeyConnect.svelte";
  import PatConnect from "./PatConnect.svelte";
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

  let methods = $derived(connectMethodsFor(provider));
  let method = $state<AuthType | null>(null);

  // Single-method providers skip the picker and land straight on the form.
  // `null` means "not chosen yet", which only ever happens for multi-method
  // providers.
  let active = $derived(method ?? (methods.length === 1 ? methods[0] : null));

  function handleSelect(m: AuthType) {
    method = m;
  }

  function handleSuccess() {
    onSuccess?.();
  }

  function handleClose() {
    method = null;
    onClose?.();
  }

  $effect(() => {
    // A new provider is a new flow; never carry the previous method over.
    if (show) method = methods.length === 1 ? methods[0] : null;
  });
</script>

<MethodPicker
  {provider}
  {methods}
  show={show && active === null}
  onSelect={handleSelect}
  onClose={handleClose}
/>

{#if active === "oauth"}
  <OAuthConnect {provider} show={show && active !== null} onClose={handleClose} onSuccess={handleSuccess} />
{:else if active === "cookie"}
  <CookieConnect {provider} show={show && active !== null} onClose={handleClose} onSuccess={handleSuccess} />
{:else if active === "api_key"}
  <ApiKeyConnect {provider} show={show && active !== null} onClose={handleClose} onSuccess={handleSuccess} />
{:else if active === "pat"}
  <PatConnect {provider} show={show && active !== null} onClose={handleClose} onSuccess={handleSuccess} />
{:else if active === "web3"}
  <Web3Connect {provider} show={show && active !== null} onClose={handleClose} onSuccess={handleSuccess} />
{:else if active === "extension"}
  <ChromeExtensionConnect {provider} show={show && active !== null} onClose={handleClose} onSuccess={handleSuccess} />
{/if}
