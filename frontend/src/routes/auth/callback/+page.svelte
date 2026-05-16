<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';

  let status = $state<'loading' | 'success' | 'error'>('loading');
  let message = $state('');

  onMount(() => {
    const params = new URLSearchParams($page.url.search);
    const connected = params.get('connected');
    const error = params.get('error');
    const pending = params.get('pending');
    const name = params.get('name');

    if (error) {
      status = 'error';
      message = decodeURIComponent(error);
    } else if (connected) {
      status = 'success';
      message = name ? decodeURIComponent(name) : connected;
    } else if (pending) {
      // Multi-step provider: redirect to page picker
      const integrationId = params.get('integration_id');
      const token = params.get('token');
      if (token) localStorage.setItem('token', token);
      window.location.href = `/?pending=${pending}&integration_id=${integrationId}`;
      return;
    }

    // Send message to opener window (the main app window)
    if (window.opener) {
      window.opener.postMessage(
        {
          type: 'oauth-connected',
          provider: connected,
          success: status === 'success',
          error: status === 'error' ? message : undefined,
        },
        '*'
      );
      // Close the popup after a brief delay
      setTimeout(() => window.close(), 500);
    }
  });
</script>

<div class="min-h-screen bg-[#0b0e14] flex items-center justify-center">
  <div class="text-center space-y-4">
    {#if status === 'loading'}
      <div class="animate-spin h-8 w-8 border-2 border-indigo-500 border-t-transparent rounded-full mx-auto"></div>
      <p class="text-sm text-[#6b7280]">Processing OAuth callback...</p>
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
