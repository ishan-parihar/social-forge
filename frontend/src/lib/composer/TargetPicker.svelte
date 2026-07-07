<script lang="ts">
  import type { TargetInfo } from "$lib/api/integrations";

  let {
    targets = [],
    selectedTargets = [],
    onToggle,
    placeholder = "Search targets...",
    maxHeight = "240px",
  }: {
    targets?: TargetInfo[];
    selectedTargets?: string[];
    onToggle?: (id: string) => void;
    placeholder?: string;
    maxHeight?: string;
  } = $props();

  let searchQuery = $state("");

  const filteredTargets = $derived(
    searchQuery.length > 0
      ? targets.filter(t =>
          t.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          t.target_type.toLowerCase().includes(searchQuery.toLowerCase())
        )
      : targets
  );

  const selectedCount = $derived(selectedTargets.length);

  function toggleTarget(id: string) {
    onToggle?.(id);
  }

  // Target type icon mapping
  function targetTypeIcon(type: string): string {
    switch (type.toLowerCase()) {
      case "subreddit": return "📢";
      case "channel": return "📺";
      case "group": return "👥";
      case "peer": return "👤";
      case "blog": return "📝";
      case "community": return "🌐";
      default: return "🎯";
    }
  }

  // Target type badge color
  function targetTypeColor(type: string): string {
    switch (type.toLowerCase()) {
      case "subreddit": return "bg-orange-500/20 text-orange-400 border-orange-500/30";
      case "channel": return "bg-blue-500/20 text-blue-400 border-blue-500/30";
      case "group": return "bg-purple-500/20 text-purple-400 border-purple-500/30";
      case "peer": return "bg-green-500/20 text-green-400 border-green-500/30";
      case "blog": return "bg-yellow-500/20 text-yellow-400 border-yellow-500/30";
      case "community": return "bg-teal-500/20 text-teal-400 border-teal-500/30";
      default: return "bg-surface-hover text-muted border-line";
    }
  }
</script>

<div>
  <!-- Search + Selected Count Header -->
  <div class="flex items-center gap-2 mb-2">
    <div class="relative flex-1">
      <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted text-sm">&#128269;</span>
      <input
        type="text"
        bind:value={searchQuery}
        placeholder={placeholder}
        class="w-full pl-8 pr-3 py-1.5 bg-background-input border border-line rounded-lg text-sm focus:border-indigo-500 outline-none placeholder:text-muted-dark"
      />
    </div>
    {#if selectedCount > 0}
      <span class="inline-flex items-center gap-1 px-2 py-1 bg-indigo-500/20 text-indigo-400 border border-indigo-500/30 rounded-full text-xs font-medium whitespace-nowrap">
        {selectedCount} selected
      </span>
    {/if}
  </div>

  <!-- Target List -->
  <div
    class="overflow-y-auto rounded-lg border border-line"
    style="max-height: {maxHeight};"
  >
    {#if filteredTargets.length === 0}
      <div class="text-sm text-muted py-6 text-center">
        {searchQuery ? "No targets match your search" : "No targets found"}
      </div>
    {:else}
      <div class="divide-y divide-[#1e2435]">
        {#each filteredTargets as target (target.id)}
          {@const isSelected = selectedTargets.includes(target.id)}
          <button
            onclick={() => toggleTarget(target.id)}
            class="w-full flex items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-surface-hover {isSelected ? 'bg-indigo-500/5' : ''}"
            aria-label="{isSelected ? 'Deselect' : 'Select'} {target.name}"
          >
            <!-- Checkbox -->
            <span class="flex-shrink-0 w-4 h-4 rounded border flex items-center justify-center transition-colors
              {isSelected
                ? 'border-indigo-500 bg-indigo-500 text-white'
                : 'border-line-hover bg-transparent'}">
              {#if isSelected}
                <svg class="w-3 h-3" viewBox="0 0 12 12" fill="none">
                  <path d="M2 6L5 9L10 3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
              {/if}
            </span>

            <!-- Picture/Icon -->
            {#if target.picture}
              <img src={target.picture} alt="" class="w-6 h-6 rounded-full flex-shrink-0 object-cover" />
            {:else}
              <span class="w-6 h-6 rounded-full bg-surface-hover flex items-center justify-center text-xs flex-shrink-0">
                {targetTypeIcon(target.target_type)}
              </span>
            {/if}

            <!-- Name + Type Badge -->
            <div class="flex-1 min-w-0">
              <div class="text-sm truncate">{target.name}</div>
            </div>
            <span class="flex-shrink-0 px-1.5 py-0.5 text-[10px] font-medium rounded border {targetTypeColor(target.target_type)}">
              {target.target_type}
            </span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>
