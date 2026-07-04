<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import PagePicker from '$lib/channels/PagePicker.svelte';

  let status = $state<'loading' | 'success' | 'error' | 'pending'>('loading');
  let message = $state('');
  let pendingProvider = $state('');
  let pendingIntegrationId = $state('');

  onMount(() => {
    const params = new URLSearchParams($page.url.search);
    const connected = params.get('connected');
    const error = params.get('error');
    const pending = params.get('pending');
    const integrationId = params.get('integration_id');
    const name = params.get('name');

    if (error) {
      status = 'error';
      message = decodeURIComponent(error);
    } else if (connected) {
      status = 'success';
      message = name ? decodeURIComponent(name) : connected;
    } else if (pending && integrationId) {
      status = 'pending';
      pendingProvider = pending;
      pendingIntegrationId = integrationId;
    }

    if (window.opener && status !== 'pending') {
      window.opener.postMessage(
        {
          type: 'oauth-connected',
          provider: connected,
          success: status === 'success',
          error: status === 'error' ? message : undefined,
        },
        '*'
      );
      setTimeout(() => window.close(), 500);
    }
  });

  function handlePagePickerSuccess() {
    if (window.opener) {
      window.opener.postMessage({ type: 'oauth-connected', provider: pendingProvider, success: true }, '*');
    }
  }

  function handlePagePickerClose() {
    if (window.opener) {
      // Popup context: notify opener and close the popup.
      window.opener.postMessage({ type: 'oauth-connected', provider: pendingProvider, success: false }, '*');
      window.close();
    } else {
      // Direct navigation: go to channels page.
      window.location.href = '/channels';
    }
  }
</script>

<div class="min-h-screen bg-[#0b0e14] flex items-center justify-center">
  <div class="text-center space-y-4">
    {#if status === 'loading'}
      <div class="animate-spin h-8 w-8 border-2 border-indigo-500 border-t-transparent rounded-full mx-auto"></div>
      <p class="text-sm text-[#6b7280]">Processing OAuth callback...</p>
    {:else if status === 'pending'}
      <PagePicker
        provider={pendingProvider}
        integrationId={pendingIntegrationId}
        show={true}
        onSuccess={handlePagePickerSuccess}
        onClose={handlePagePickerClose}
      />
    {:else if status === 'success'}
      <div class="text-4xl">✅</div>
      <p class="text-sm text-green-400">Connected as {message}</p>
      <p class="text-xs text-[#6b7280]">This window will close automatically.</p>
    {:else}
      <div class="text-4xl">❌</div>
      <p class="text-sm text-red-400">Connection failed</p>
      <p class="text-xs text-[#6b7280]">{message}</p>
      <p class="text-xs text-[#6b7280]">This window will close automatically.</p>
    {/if}
  </div>
</div>
