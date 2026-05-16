<script lang="ts">
  import Modal from "$lib/ui/Modal.svelte";
  import Button from "$lib/ui/Button.svelte";
  import { integrationsApi } from "$lib/api/integrations";
  import { onMount } from "svelte";

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

  let verificationCode = $state("");
  let label = $state("");
  let userCode = $state("");
  let verifying = $state(false);
  let error = $state("");

  function generateCode() {
    const chars = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let code = "";
    for (let i = 0; i < 6; i++) {
      code += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    verificationCode = code;
  }

  onMount(() => {
    generateCode();
  });

  async function handleVerify() {
    error = "";
    if (!label.trim()) {
      error = "Please enter a label for this integration.";
      return;
    }
    if (!userCode.trim()) {
      error = "Please enter the verification code shown above.";
      return;
    }
    if (userCode.trim() !== verificationCode) {
      error = "Verification code does not match. Double-check the code displayed above.";
      return;
    }
    verifying = true;
    try {
      const r = await integrationsApi.connectApiKey({
        provider,
        api_key: verificationCode,
        label: label.trim(),
        verification_code: verificationCode,
      });
      if (r.error) {
        error = r.error;
      } else {
        onSuccess?.();
      }
    } catch (e) {
      error = "Failed to connect. Please try again.";
      console.error("Extension connect failed:", e);
    }
    verifying = false;
  }
</script>

<Modal open={show} title="Connect via Browser Extension" onclose={onClose}>
  <div class="space-y-4">
    <div class="text-sm text-[#9ca3af] leading-relaxed">
      <p>
        This provider requires a browser extension to connect your account.
      </p>
    </div>

    <a
      href="https://chrome.google.com/webstore"
      target="_blank"
      rel="noopener noreferrer"
      class="inline-flex items-center justify-center w-full gap-2 px-4 py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white text-sm font-medium rounded-lg transition-colors"
    >
      <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
      </svg>
      Install the Chrome Extension
    </a>

    {#if verificationCode}
      <div class="bg-[#0d1117] border border-[#1e2435] rounded-lg p-4 text-center space-y-2">
        <p class="text-xs text-[#6b7280] uppercase tracking-wider font-medium">Your verification code</p>
        <p class="text-2xl font-mono font-bold tracking-[0.25em] text-indigo-400 select-all">{verificationCode}</p>
        <p class="text-xs text-[#6b7280]">Enter this code below to verify the extension is installed.</p>
      </div>
    {/if}

    <div class="bg-[#0d1117] border border-[#1e2435] rounded-lg p-4 space-y-3">
      <h4 class="text-sm font-medium text-white">Complete Connection</h4>

      <div>
        <label for="ext-label" class="block text-xs text-[#6b7280] mb-1">Integration Label</label>
        <input
          id="ext-label"
          type="text"
          bind:value={label}
          placeholder="e.g. My Skool Community"
          class="w-full bg-[#030712] border border-[#1e2435] rounded-lg px-3 py-2 text-sm text-white placeholder-[#4b5563] focus:outline-none focus:ring-1 focus:ring-indigo-500"
        />
      </div>

      <div>
        <label for="ext-code" class="block text-xs text-[#6b7280] mb-1">Verification Code</label>
        <input
          id="ext-code"
          type="text"
          bind:value={userCode}
          placeholder="Enter the 6-character code"
          maxlength="6"
          class="w-full bg-[#030712] border border-[#1e2435] rounded-lg px-3 py-2 text-sm text-white placeholder-[#4b5563] font-mono tracking-widest text-center focus:outline-none focus:ring-1 focus:ring-indigo-500 uppercase"
          style="text-transform: uppercase;"
        />
      </div>

      {#if error}
        <p class="text-xs text-red-400">{error}</p>
      {/if}

      <div class="flex justify-end gap-2 pt-1">
        <Button variant="secondary" onclick={onClose}>Cancel</Button>
        <Button variant="primary" onclick={handleVerify} disabled={verifying}>
          {verifying ? "Verifying..." : "Verify & Connect"}
        </Button>
      </div>
    </div>
  </div>
</Modal>
