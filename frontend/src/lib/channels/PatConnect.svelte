<script lang="ts">
  // v25 F4 — token-paste connect path (GitHub PAT, Telegram bot token).
  //
  // Both are "paste a long-lived secret the provider issued you" flows, so
  // they share one component. They were separate inline branches in the
  // channels page before; the shared form keeps validation and error
  // presentation identical between them.
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

  let token = $state("");
  let label = $state("");
  let submitting = $state(false);
  let errorMsg = $state("");

  const isTelegramBot = $derived(provider === "telegram-bot");

  const PAT_HELP: Record<string, { title: string; help: string; helpHref?: string; helpLabel?: string }> = {
    github: {
      title: "Connect GitHub with a token",
      help: "Create a token under Settings → Developer settings → Personal access tokens. The repo scope covers posting to a blog or repo file.",
      helpHref: "https://github.com/settings/tokens",
      helpLabel: "Open GitHub token settings",
    },
    "telegram-bot": {
      title: "Add a Telegram bot token",
      help: "Create a bot with BotFather and paste the token it gives you. If you already have a bot configured in .env, choose the configured-bot option instead.",
      helpHref: "https://t.me/BotFather",
      helpLabel: "Open BotFather",
    },
  };

  let copy = $derived(PAT_HELP[provider] ?? PAT_HELP.github);

  function reset() {
    token = "";
    label = "";
    errorMsg = "";
    submitting = false;
  }

  async function handleSubmit() {
    if (!token.trim()) {
      errorMsg = "A token is required";
      return;
    }
    submitting = true;
    errorMsg = "";
    try {
      const r = isTelegramBot
        ? await integrationsApi.connectTelegramBotToken(token.trim())
        : await integrationsApi.connectGithubPat(token.trim(), label.trim() || undefined);
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
    if (show) reset();
  });
</script>

<Modal open={show} title={copy.title} onclose={handleClose}>
  <form onsubmit={handleSubmit} class="space-y-4">
    <p class="text-sm text-muted">
      {copy.help}
      {#if copy.helpHref}
        <a href={copy.helpHref} target="_blank" rel="noreferrer" class="text-accent hover:text-accent-strong">
          {copy.helpLabel}
        </a>
      {/if}
    </p>

    <div>
      <label for="pat-token" class="block text-xs font-medium text-muted mb-1">
        {isTelegramBot ? "Bot token" : "Personal access token"} <span class="text-error">*</span>
      </label>
      <input
        id="pat-token"
        type="password"
        bind:value={token}
        required
        autocomplete="off"
        placeholder={isTelegramBot ? "123456:ABC-DEF..." : "ghp_..."}
        class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content placeholder:text-faint focus:outline-none focus:border-accent transition-colors font-mono"
      />
    </div>

    {#if !isTelegramBot}
      <div>
        <label for="pat-label" class="block text-xs font-medium text-muted mb-1">
          Label <span class="text-faint">(optional)</span>
        </label>
        <input
          id="pat-label"
          type="text"
          bind:value={label}
          placeholder="My GitHub"
          class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content placeholder:text-faint focus:outline-none focus:border-accent transition-colors"
        />
        <p class="text-xs text-faint mt-1">Shown in the channel list so you can tell multiple tokens apart.</p>
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
