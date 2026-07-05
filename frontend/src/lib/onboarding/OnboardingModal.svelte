<script lang="ts">
  import { onMount } from 'svelte';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import Icon from '$lib/ui/Icon.svelte';

  let { onClose } = $props();
  let step = $state(1);
  let integrations = $state<Integration[]>([]);
  let loading = $state(true);

  const ONBOARDING_KEY = 'social-forge-onboarded';

  onMount(async () => {
    const res = await integrationsApi.list();
    if (res.data) {
      integrations = res.data.integrations.filter(i => !i.disabled);
    }
    loading = false;
  });

  function finish() {
    localStorage.setItem(ONBOARDING_KEY, 'true');
    onClose();
  }

  function skip() {
    localStorage.setItem(ONBOARDING_KEY, 'true');
    onClose();
  }

  let checklist = $derived([
    { label: 'Connect a channel', done: integrations.length > 0, href: '/channels' },
    { label: 'Create a post', done: false, href: '/posts/new' },
    { label: 'Schedule it on the calendar', done: false, href: '/calendar' },
    { label: 'View your dashboard', done: false, href: '/' },
  ]);
</script>

<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
  <div class="bg-surface border border-line rounded-2xl max-w-lg w-full mx-4 overflow-hidden">
    <div class="px-6 py-5 border-b border-line">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold text-indigo-400">
          {#if step === 1}
            Welcome to Social Forge
          {:else}
            Quick Start
          {/if}
        </h2>
        <button onclick={skip} class="text-muted hover:text-white text-sm">Skip</button>
      </div>
      <div class="flex gap-1.5 mt-3">
        <div class="h-1 flex-1 rounded-full {step >= 1 ? 'bg-indigo-500' : 'bg-line'}"></div>
        <div class="h-1 flex-1 rounded-full {step >= 2 ? 'bg-indigo-500' : 'bg-line'}"></div>
      </div>
    </div>

    <div class="px-6 py-6">
      {#if step === 1}
        <div>
          <p class="text-sm text-muted mb-4">
            Social Forge is your solo-founder social media command center. Connect your social channels to get started.
          </p>
          {#if loading}
            <div class="text-center py-8 text-muted text-sm">Loading...</div>
          {:else if integrations.length > 0}
            <div class="mb-4">
              <p class="text-xs text-muted mb-2">Connected channels ({integrations.length})</p>
              <div class="flex flex-wrap gap-2">
                {#each integrations.slice(0, 6) as int}
                  <div class="px-3 py-1.5 bg-surface-hover rounded-lg text-xs text-content">
                    {int.provider_name}
                  </div>
                {/each}
              </div>
            </div>
          {/if}
          <a
            href="/channels"
            onclick={finish}
            class="block w-full text-center px-4 py-3 bg-brand-500 hover:bg-brand-600 text-white rounded-lg font-medium text-sm transition-colors"
          >
            Connect a Channel
          </a>
          <button
            onclick={() => step = 2}
            class="block w-full text-center px-4 py-2 mt-2 text-muted hover:text-white text-sm"
          >
            {#if integrations.length > 0}
              Continue to Quick Start
            {:else}
              Continue without channels
            {/if}
          </button>
        </div>
      {:else}
        <div class="space-y-3">
          <p class="text-sm text-muted mb-4">
            Here's how to get started in 4 steps:
          </p>
          {#each checklist as item, i}
            <a
              href={item.href}
              onclick={finish}
              class="flex items-center gap-3 p-3 bg-surface-hover rounded-lg hover:bg-background-input transition-colors cursor-pointer"
            >
              <div class="w-7 h-7 rounded-full bg-brand-500/20 text-brand-400 flex items-center justify-center text-xs font-bold shrink-0">
                {i + 1}
              </div>
              <span class="text-sm text-content flex-1">{item.label}</span>
              <span class="text-muted text-xs">→</span>
            </a>
          {/each}
        </div>
        <button
          onclick={finish}
          class="block w-full text-center px-4 py-3 mt-5 bg-brand-500 hover:bg-brand-600 text-white rounded-lg font-medium text-sm transition-colors"
        >
          Get Started
        </button>
      {/if}
    </div>
  </div>
</div>
