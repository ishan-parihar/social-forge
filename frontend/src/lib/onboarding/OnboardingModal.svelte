<script lang="ts">
  // OnboardingModal — 2-step post-signup onboarding (Phase 3, v19).
  //
  // Step 1: Connect Channels — shows already-connected channels + a grid
  //         of available providers to connect. "Continue" proceeds to
  //         step 2 even if no channels are connected.
  // Step 2: Quick Tour — a static "how to" guide showing the 4 key flows
  //         (composer modal, calendar drag-drop, posts list, analytics).
  //         "Get Started" closes the modal and routes to /calendar.
  //
  // Inspired by postiz-app's onboarding.modal.tsx (2-step: connect → tutorial).
  // Adapted for Social Forge's single-user model (no team invite step).

  import { onMount } from 'svelte';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { goto } from '$app/navigation';
  import { providerLabel, providerIcon, providerColor } from '$lib/providers';

  let { onClose } = $props();
  let step = $state(1);
  let integrations = $state<Integration[]>([]);
  let loading = $state(true);

  const ONBOARDING_KEY = 'social-forge-onboarded';

  // Available providers to show in the connect grid (Step 1).
  // Subset of the full provider list — the most popular ones.
  const AVAILABLE_PROVIDERS = [
    'x', 'reddit', 'linkedin', 'facebook', 'instagram',
    'threads', 'bluesky', 'mastodon', 'youtube', 'tiktok',
    'pinterest', 'discord', 'slack', 'telegram-bot',
  ];

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
    goto('/calendar');
  }

  function skip() {
    localStorage.setItem(ONBOARDING_KEY, 'true');
    onClose();
  }

  // Tour steps for Step 2 — the 4 key flows.
  const tourSteps = [
    {
      icon: '✨',
      title: 'Create posts in a modal',
      description: 'Press "n" or click any calendar slot to open the composer. It stays on top of the calendar so you never lose context.',
    },
    {
      icon: '📅',
      title: 'Drag to reschedule',
      description: 'Grab any post on the calendar and drag it to a new time. Published posts get a safety modal — no accidental re-publishes.',
    },
    {
      icon: '🌐',
      title: 'Multi-channel posting',
      description: 'Select multiple channels, write once globally, then switch to per-channel tabs to customize. Pink dots show which channels diverged.',
    },
    {
      icon: '📊',
      title: 'Track engagement',
      description: 'Click the 📊 icon on any post to see per-metric charts. The feed refresher pulls engagement data every 30 minutes.',
    },
  ];
</script>

<div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4">
  <div class="bg-surface border border-line rounded-2xl max-w-lg w-full max-h-[90vh] overflow-y-auto">
    <!-- Header with step indicator -->
    <div class="px-6 py-5 border-b border-line">
      <div class="flex items-center justify-between">
        <h2 class="text-xl font-bold text-brand-400">
          {#if step === 1}
            Welcome to Social Forge
          {:else}
            Quick Tour
          {/if}
        </h2>
        <button onclick={skip} class="text-muted hover:text-white text-sm">Skip</button>
      </div>
      <!-- Step indicator -->
      <div class="flex items-center gap-2 mt-4">
        <div class="flex items-center gap-1.5">
          <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold {step >= 1 ? 'bg-brand-500 text-white' : 'bg-surface-hover text-muted'}">1</div>
          <span class="text-xs {step >= 1 ? 'text-content' : 'text-muted'}">Connect Channels</span>
        </div>
        <div class="flex-1 h-px bg-line"></div>
        <div class="flex items-center gap-1.5">
          <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold {step >= 2 ? 'bg-brand-500 text-white' : 'bg-surface-hover text-muted'}">2</div>
          <span class="text-xs {step >= 2 ? 'text-content' : 'text-muted'}">Quick Tour</span>
        </div>
      </div>
    </div>

    <div class="px-6 py-6">
      {#if step === 1}
        <!-- Step 1: Connect Channels -->
        <div>
          <p class="text-sm text-muted mb-4">
            Social Forge is your solo-founder social media command center.
            Connect your channels to get started — you can add more later.
          </p>

          {#if loading}
            <div class="text-center py-8 text-muted text-sm">Loading...</div>
          {:else if integrations.length > 0}
            <div class="mb-4">
              <p class="text-xs text-muted mb-2">Connected channels ({integrations.length})</p>
              <div class="flex flex-wrap gap-2">
                {#each integrations as int (int.id)}
                  <div class="flex items-center gap-1.5 px-3 py-1.5 bg-surface-hover rounded-lg text-xs text-content">
                    <span style="color: {providerColor(int.provider_identifier)}">{providerIcon(int.provider_identifier)}</span>
                    {int.provider_name}
                  </div>
                {/each}
              </div>
            </div>
          {/if}

          <!-- Available providers grid -->
          <div class="mb-4">
            <p class="text-xs text-muted mb-2">Available channels</p>
            <div class="grid grid-cols-4 gap-2">
              {#each AVAILABLE_PROVIDERS as p}
                <a
                  href="/channels"
                  onclick={skip}
                  class="flex flex-col items-center gap-1 p-3 bg-surface-hover hover:bg-background-input rounded-lg transition-colors text-center group"
                >
                  <span class="text-lg" style="color: {providerColor(p)}">{providerIcon(p)}</span>
                  <span class="text-[10px] text-muted group-hover:text-content">{providerLabel(p)}</span>
                </a>
              {/each}
            </div>
          </div>

          <a
            href="/channels"
            onclick={skip}
            class="block w-full text-center px-4 py-3 bg-brand-600 hover:bg-brand-500 text-white rounded-lg font-medium text-sm transition-colors"
          >
            Connect a Channel
          </a>
          <button
            onclick={() => step = 2}
            class="block w-full text-center px-4 py-2 mt-2 text-muted hover:text-white text-sm"
          >
            {#if integrations.length > 0}
              Continue to Quick Tour →
            {:else}
              Continue without channels →
            {/if}
          </button>
        </div>
      {:else}
        <!-- Step 2: Quick Tour -->
        <div class="space-y-3">
          <p class="text-sm text-muted mb-4">
            Here are the 4 key flows you'll use every day:
          </p>
          {#each tourSteps as item, i}
            <div class="flex items-start gap-3 p-3 bg-surface-hover rounded-lg">
              <div class="w-10 h-10 rounded-lg bg-brand-500/20 flex items-center justify-center text-xl shrink-0">
                {item.icon}
              </div>
              <div class="flex-1">
                <div class="text-sm font-medium text-content">{item.title}</div>
                <div class="text-xs text-muted mt-0.5 leading-relaxed">{item.description}</div>
              </div>
            </div>
          {/each}
        </div>
        <button
          onclick={finish}
          class="block w-full text-center px-4 py-3 mt-5 bg-brand-600 hover:bg-brand-500 text-white rounded-lg font-medium text-sm transition-colors"
        >
          Get Started
        </button>
      {/if}
    </div>
  </div>
</div>
