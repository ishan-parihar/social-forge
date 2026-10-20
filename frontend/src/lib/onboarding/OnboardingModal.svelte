<script lang="ts">
  // OnboardingModal - 2-step post-signup onboarding (Phase 3, v19).
  //
  // Step 1: Connect Channels - shows already-connected channels + a grid
  //         of available providers to connect. "Continue" proceeds to
  //         step 2 even if no channels are connected.
  // Step 2: Quick Tour - a static "how to" guide showing the 4 key flows
  //         (composer modal, calendar drag-drop, posts list, analytics).
  //         "Get Started" closes the modal and routes to /calendar.
  //
  // Inspired by postiz-app's onboarding.modal.tsx (2-step: connect → tutorial).
  // Adapted for Social Forge's single-user model (no team invite step).

  import { onMount } from 'svelte';
  import { integrationsApi, type Integration } from '$lib/api/integrations';
  import { goto } from '$app/navigation';
  import { providerLabel, providerIcon, providerColor, providerInkOnChip } from '$lib/providers';
  import { focusTrap } from '$lib/ui/focus-trap';

  let { onClose } = $props();
  let step = $state(1);
  let integrations = $state<Integration[]>([]);
  let loading = $state(true);

  const ONBOARDING_KEY = 'social-forge-onboarded';

  // Available providers to show in the connect grid (Step 1).
  // Subset of the full provider list - the most popular ones.
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

  // Tour steps for Step 2 - the 4 key flows.
  const tourSteps = [
    {
      icon: '✨',
      title: 'Create posts in a modal',
      description: 'Press "n" or click any calendar slot to open the composer. It stays on top of the calendar so you never lose context.',
    },
    {
      icon: '📅',
      title: 'Drag to reschedule',
      description: 'Grab any post on the calendar and drag it to a new time. Published posts get a safety modal - no accidental re-publishes.',
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

<!--
  The scrim is the `overlay` TOKEN, not `bg-black/70`.

  A pure black scrim does not retheme: in the light theme it is a hole
  in the page. The token carries the theme's own scrim tint.

  The PANEL carries the dialog role and the name, and it - not the
  scrim - is the focus trap's node. The scrim is a sibling, marked
  aria-hidden because it is decoration: putting the panel inside an
  aria-hidden subtree makes it invisible to a screen reader AND stops
  the trap from finding a focusable child, so the modal never takes
  the focus.
-->
<div class="fixed inset-0 bg-overlay flex items-center justify-center z-50 p-4" aria-hidden="true"></div>
<div class="fixed inset-0 flex items-center justify-center z-50 p-4 pointer-events-none">
  <!--
    The PANEL is the dialog: it carries the role, the name, the focus
    trap and the Escape handler. The wrapper is positioning only.

    The panel holds the focus, so a keydown from the panel's children
    bubbles up to it - which is how Escape reaches the handler. A
    handler on the wrapper only ever saw a keydown that nobody had
    consumed.

    `tabindex="-1"` is the trap's fallback: with no focusable child it
    focuses the panel itself.
  -->
  <div
    role="dialog"
    aria-modal="true"
    aria-labelledby="onboarding-title"
    tabindex="-1"
    class="relative bg-surface border border-line rounded-2xl max-w-lg w-full max-h-[90vh] overflow-y-auto pointer-events-auto"
    use:focusTrap={{ enabled: true }}
    onkeydown={(e) => { if (e.key === 'Escape') skip(); }}
  >
    <!-- Header with step indicator -->
    <div class="px-6 py-5 border-b border-line">
      <div class="flex items-center justify-between">
        <h2 id="onboarding-title" class="text-xl font-bold text-accent">
          {#if step === 1}
            Welcome to Social Forge
          {:else}
            Quick Tour
          {/if}
        </h2>
        <button onclick={skip} class="text-muted hover:text-content text-sm min-h-11 min-w-11 px-2">Skip</button>
      </div>
      <!-- Step indicator -->
      <div class="flex items-center gap-2 mt-4">
        <div class="flex items-center gap-1.5">
          <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold {step >= 1 ? 'bg-accent-fill text-accent-fg' : 'bg-surface-hover text-muted'}">1</div>
          <span class="text-xs {step >= 1 ? 'text-content' : 'text-muted'}">Connect Channels</span>
        </div>
        <div class="flex-1 h-px bg-line"></div>
        <div class="flex items-center gap-1.5">
          <div class="w-6 h-6 rounded-full flex items-center justify-center text-xs font-bold {step >= 2 ? 'bg-accent-fill text-accent-fg' : 'bg-surface-hover text-muted'}">2</div>
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
            Connect your channels to get started - you can add more later.
          </p>

          {#if loading}
            <div class="text-center py-8 text-muted text-sm">Loading...</div>
          {:else if integrations.length > 0}
            <div class="mb-4">
              <p class="text-xs text-muted mb-2">Connected channels ({integrations.length})</p>
              <div class="flex flex-wrap gap-2">
                {#each integrations as int (int.id)}
                  <div class="flex items-center gap-1.5 px-3 py-1.5 bg-surface-hover rounded-lg text-xs text-content">
                    <span style="color: {providerInkOnChip(int.provider_identifier)}">{providerIcon(int.provider_identifier)}</span>
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
                  <span class="text-lg" style="color: {providerInkOnChip(p)}">{providerIcon(p)}</span>
                  <span class="text-[10px] text-muted group-hover:text-content">{providerLabel(p)}</span>
                </a>
              {/each}
            </div>
          </div>

          <a
            href="/channels"
            onclick={skip}
            class="block w-full text-center px-4 py-3 bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg font-medium text-sm transition-colors"
          >
            Connect a Channel
          </a>
          <button
            onclick={() => step = 2}
            class="block w-full text-center px-4 py-2 min-h-11 mt-2 text-muted hover:text-content text-sm"
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
              <div class="w-10 h-10 rounded-lg bg-accent-fill/20 flex items-center justify-center text-xl shrink-0">
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
          class="block w-full text-center px-4 py-3 mt-5 bg-accent-fill hover:bg-accent-fill-hover text-accent-fg rounded-lg font-medium text-sm transition-colors"
        >
          Get Started
        </button>
      {/if}
    </div>
  </div>
</div>
