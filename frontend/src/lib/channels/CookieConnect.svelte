<script lang="ts">
  // v25 F4 — cookie-paste connect path (X, Reddit).
  //
  // Moved out of routes/channels/+page.svelte, where it was an inline
  // `credDialog` with a hand-rolled `bg-black/60` overlay, no Escape handler,
  // no focus trap and no aria-modal. Using the shared Modal primitive fixes
  // all four at once and keeps the connect flow in one place.
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

  let authToken = $state("");
  let ct0 = $state("");
  let cookieString = $state("");
  let submitting = $state(false);
  let errorMsg = $state("");

  // X needs two named cookies; Reddit needs the whole header string.
  const isX = $derived(provider === "x");

  const COOKIE_HELP: Record<string, string> = {
    x: "Open X while signed in, then copy these two values from your browser's developer tools (Application → Cookies → https://x.com).",
    reddit: "Sign in to Reddit, then copy the value of the cookie header for www.reddit.com from developer tools (Application → Cookies).",
  };

  function reset() {
    authToken = "";
    ct0 = "";
    cookieString = "";
    errorMsg = "";
    submitting = false;
  }

  async function handleSubmit() {
    submitting = true;
    errorMsg = "";
    try {
      const r = isX
        ? await integrationsApi.connectXCookie(authToken.trim(), ct0.trim())
        : await integrationsApi.connectRedditCookie(cookieString.trim());
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

  $effect(() => {
    // Reopening the dialog must not carry the previous attempt's secrets.
    if (show) reset();
  });
</script>

<Modal
  open={show}
  title={isX ? "Connect X with cookies" : "Connect Reddit with cookies"}
  onclose={handleClose}
>
  <form onsubmit={handleSubmit} class="space-y-4">
    <p class="text-sm text-muted">{COOKIE_HELP[provider] ?? "Paste the session cookies from your signed-in browser session."}</p>

    {#if isX}
      <div>
        <label for="x-auth-token" class="block text-xs font-medium text-muted mb-1">
          auth_token <span class="text-error">*</span>
        </label>
        <input
          id="x-auth-token"
          type="password"
          bind:value={authToken}
          required
          autocomplete="off"
          placeholder="Paste the auth_token cookie value"
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content placeholder:text-faint focus:outline-none focus:border-accent transition-colors"
        />
      </div>
      <div>
        <label for="x-ct0" class="block text-xs font-medium text-muted mb-1">
          ct0 <span class="text-error">*</span>
        </label>
        <input
          id="x-ct0"
          type="password"
          bind:value={ct0}
          required
          autocomplete="off"
          placeholder="Paste the ct0 cookie value"
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content placeholder:text-faint focus:outline-none focus:border-accent transition-colors"
        />
      </div>
    {:else}
      <div>
        <label for="reddit-cookie" class="block text-xs font-medium text-muted mb-1">
          Cookie header <span class="text-error">*</span>
        </label>
        <textarea
          id="reddit-cookie"
          bind:value={cookieString}
          required
          rows="4"
          autocomplete="off"
          placeholder="reddit_session=...; token_v2=...; csv=..."
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content placeholder:text-faint focus:outline-none focus:border-accent transition-colors font-mono"
        ></textarea>
      </div>
    {/if}

    {#if errorMsg}
      <div class="text-sm text-error bg-error/10 border border-error/20 rounded-lg px-3 py-2" role="alert">
        {errorMsg}
      </div>
    {/if}

    <div class="flex justify-end gap-2 pt-2">
      <Button variant="secondary" onclick={handleClose} disabled={submitting}>Cancel</Button>
      <Button variant="primary" onclick={handleSubmit} disabled={submitting}>
        {#if submitting}
          <span class="flex items-center gap-2"><Spinner size="sm" /> Connecting...</span>
        {:else}
          Connect
        {/if}
      </Button>
    </div>
  </form>
</Modal>
