<script lang="ts">
  import ProviderIcon from "$lib/channels/ProviderIcon.svelte";
  let { content = "", selectedIntegrations = [], integrationProviders = new Map() }: {
    content?: string;
    selectedIntegrations?: string[];
    integrationProviders?: Map<string, string>;
  } = $props();
  let plainText = $derived(content.replace(/<[^>]*>/g, "").slice(0, 120));
</script>
{#if selectedIntegrations.length > 0}
  <div class="bg-surface border border-line rounded-xl p-4">
    <h4 class="text-xs font-semibold text-muted uppercase mb-3">Preview ({selectedIntegrations.length} platforms)</h4>
    <div class="space-y-2">
      {#each selectedIntegrations as intId}
        {@const providerId = integrationProviders.get(intId) || intId}
        <div class="flex items-start gap-3 p-3 bg-background-input rounded-lg">
          <ProviderIcon provider={providerId} size="sm" />
          <div class="flex-1 min-w-0">
            <div class="text-xs text-muted capitalize mb-1">{providerId}</div>
            <div class="text-sm text-content-secondary">{plainText || "No content"}</div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
