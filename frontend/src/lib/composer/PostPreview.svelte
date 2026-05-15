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
  <div class="bg-[#131720] border border-[#1e2435] rounded-xl p-4">
    <h4 class="text-xs font-semibold text-[#6b7280] uppercase mb-3">Preview ({selectedIntegrations.length} platforms)</h4>
    <div class="space-y-2">
      {#each selectedIntegrations as intId}
        {@const providerId = integrationProviders.get(intId) || intId}
        <div class="flex items-start gap-3 p-3 bg-[#0d1117] rounded-lg">
          <ProviderIcon provider={providerId} size="sm" />
          <div class="flex-1 min-w-0">
            <div class="text-xs text-[#6b7280] capitalize mb-1">{providerId}</div>
            <div class="text-sm text-[#d1d5db]">{plainText || "No content"}</div>
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}
