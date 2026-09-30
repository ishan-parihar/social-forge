<script lang="ts">
  // v25 F4 — method chooser for providers that offer more than one way in.
  //
  // Replaces the channels page's bespoke `connectChoice` dialog, which was a
  // hardcoded if/else on provider id: adding a provider meant editing a
  // 30-branch block of JSX, and a provider with a method nobody had listed
  // silently offered nothing. This renders from a list instead.
  import Modal from "$lib/ui/Modal.svelte";
  import Button from "$lib/ui/Button.svelte";
  import Icon from "$lib/ui/Icon.svelte";
  import ProviderIcon from "./ProviderIcon.svelte";
  import { METHOD_LABELS, type AuthType } from "./auth-types";
  import { providerLabel } from "$lib/providers";

  let {
    provider = "",
    methods = [] as AuthType[],
    show = false,
    onSelect,
    onClose,
  }: {
    provider?: string;
    methods?: AuthType[];
    show?: boolean;
    onSelect?: (method: AuthType) => void;
    onClose?: () => void;
  } = $props();

  /** Per-method explanation. Why the choice exists matters: OAuth and cookies
   *  are not two spellings of the same thing, they grant different access. */
  const METHOD_HINTS: Record<AuthType, string> = {
    oauth: "Standard sign-in. Limited to the scopes the app requested.",
    cookie: "Full access. Includes DMs, analytics and features the OAuth scopes do not cover.",
    api_key: "Paste a key from the provider's developer console.",
    pat: "Create a token in the provider's settings. No OAuth app required.",
    web3: "Sign in with a wallet address.",
    extension: "Install the companion browser extension; it passes credentials to this instance.",
  };

  const METHOD_ICONS: Record<AuthType, string> = {
    oauth: "externalLink",
    cookie: "settings",
    api_key: "developer",
    pat: "check",
    web3: "profile",
    extension: "automation",
  };

  // v25 F5: `aria-checked` was hardcoded to "false" on every option, so the
  // radiogroup announced "none selected" no matter what — and nothing moved a
  // selection without the pointer. Now the selection is real state and the
  // ARIA radio keyboard contract (arrows move AND select, roving tabindex) is
  // implemented, because a radiogroup that only answers to clicks is a
  // listbox in disguise.
  let selected = $state<AuthType | null>(null);
  $effect(() => {
    // Reset when the dialog reopens for a different provider.
    if (show) selected = methods.length === 1 ? methods[0] : null;
  });

  function select(method: AuthType) {
    selected = method;
    onSelect?.(method);
  }

  function onKeydown(e: KeyboardEvent) {
    const i = methods.indexOf(selected ?? methods[0]);
    if (i < 0) return;
    let next = -1;
    if (e.key === "ArrowDown" || e.key === "ArrowRight") next = (i + 1) % methods.length;
    else if (e.key === "ArrowUp" || e.key === "ArrowLeft") next = (i - 1 + methods.length) % methods.length;
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = methods.length - 1;
    if (next < 0) return;
    e.preventDefault();
    select(methods[next]);
    queueMicrotask(() => {
      document.getElementById(`method-${methods[next]}`)?.focus();
    });
  }
</script>

<Modal open={show} title="Connect {providerLabel(provider)}" onclose={onClose}>
  <div class="space-y-3">
    <p class="text-sm text-muted">
      {providerLabel(provider)} can be connected {methods.length === 1 ? "one way" : "more than one way"}.
      Pick the one that fits the access you need.
    </p>

    <div class="space-y-2" role="radiogroup" aria-label="Connection method" tabindex="-1" onkeydown={onKeydown}>
      {#each methods as method (method)}
        <button
          type="button"
          id="method-{method}"
          role="radio"
          aria-checked={selected === method}
          tabindex={selected === method ? 0 : -1}
          onclick={() => select(method)}
          class="w-full text-left px-4 py-3 bg-background-input border rounded-lg hover:border-accent/50 hover:bg-surface-hover transition-colors flex items-start gap-3 {selected === method ? 'border-accent' : 'border-line'}"
        >
          <ProviderIcon {provider} size="sm" />
          <span class="flex-1 min-w-0">
            <span class="flex items-center gap-1.5 text-sm font-medium">
              <Icon name={METHOD_ICONS[method]} class="w-3.5 h-3.5 text-accent" />
              {METHOD_LABELS[method]}
            </span>
            <span class="block text-xs text-muted mt-0.5">{METHOD_HINTS[method]}</span>
          </span>
        </button>
      {/each}
    </div>

    <div class="flex justify-end pt-2">
      <Button variant="secondary" onclick={onClose}>Cancel</Button>
    </div>
  </div>
</Modal>
