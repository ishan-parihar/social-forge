<script lang="ts">
  import { onMount } from 'svelte';
  import { integrations, type Integration } from '$lib/api';
  import { toast } from '$lib/stores/toast';

  let channels = $state<Integration[]>([]);
  let connecting = $state('');

  const providers = [
    { id: 'x', name: 'X / Twitter', icon: '𝕏', color: '#000' },
    { id: 'linkedin', name: 'LinkedIn', icon: 'in', color: '#0a66c2' },
    { id: 'bluesky', name: 'Bluesky', icon: '☁️', color: '#0085ff' },
    { id: 'facebook', name: 'Facebook', icon: 'f', color: '#1877f2' },
    { id: 'instagram', name: 'Instagram', icon: '📷', color: '#e4405f' },
  ];

  onMount(async () => { const r = await integrations.list(); if (r.data) channels = r.data.integrations; });

  async function connect(provider: string) {
    connecting = provider;
    const r = await integrations.connect(provider);
    connecting = '';
    if (r.data?.url) {
      window.open(r.data.url, '_blank', 'width=600,height=700');
      toast('Authorization window opened. Complete the OAuth flow to connect.', 'info');
      setTimeout(async () => {
        const r2 = await integrations.list();
        if (r2.data) channels = r2.data.integrations;
        toast('Channels refreshed', 'success');
      }, 5000);
    } else {
      toast(r.error || 'Connection failed', 'error');
    }
  }

  async function disconnect(id: string) {
    if (!confirm('Remove this channel?')) return;
    await integrations.delete(id);
    channels = channels.filter(c => c.id !== id);
    toast('Channel removed', 'success');
  }
</script>

<div class="space-y-6">
  <div>
    <h2 class="text-xl font-semibold">Connected Channels</h2>
    <p class="text-sm text-[#6b7280] mt-1">Manage your social media accounts</p>
  </div>

  {#if channels.length > 0}
    <div class="grid gap-3 md:grid-cols-2">
      {#each channels as ch}
        <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4 flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg flex items-center justify-center text-lg font-bold" style="background: {providers.find(p => p.id === ch.provider_identifier)?.color || '#333'}20; color: {providers.find(p => p.id === ch.provider_identifier)?.color || '#fff'}">
            {providers.find(p => p.id === ch.provider_identifier)?.icon || ch.provider_identifier[0].toUpperCase()}
          </div>
          <div class="flex-1 min-w-0">
            <p class="text-sm font-medium truncate">{ch.provider_name}</p>
            <p class="text-xs text-[#6b7280] truncate">{ch.profile_name || ch.provider_identifier}</p>
          </div>
          {#if ch.refresh_needed}<span class="text-[10px] px-2 py-0.5 rounded badge-error">Refresh needed</span>{/if}
          <button onclick={() => disconnect(ch.id)} class="text-xs text-red-400 hover:text-red-300">Remove</button>
        </div>
      {/each}
    </div>
  {:else}
    <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-8 text-center">
      <p class="text-sm text-[#6b7280]">No channels connected yet.</p>
      <p class="text-xs text-[#6b7280] mt-1">Connect a social media account below to start posting.</p>
    </div>
  {/if}

  <div>
    <h3 class="text-sm font-medium mb-3">Available Providers</h3>
    <div class="grid gap-2 md:grid-cols-3">
      {#each providers as p}
        <button onclick={() => connect(p.id)} disabled={!!connecting}
          class="bg-[#131720] hover:bg-[#1a1f2e] border border-[#1e2435] rounded-xl p-4 text-left transition-colors disabled:opacity-50"
        >
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-lg flex items-center justify-center font-bold" style="background: {p.color}20; color: {p.color}">{p.icon}</div>
            <div>
              <div class="text-sm font-medium">{p.name}</div>
              <div class="text-xs text-[#6b7280]">{connecting === p.id ? 'Connecting...' : 'Connect account'}</div>
            </div>
          </div>
        </button>
      {/each}
    </div>
  </div>
</div>
