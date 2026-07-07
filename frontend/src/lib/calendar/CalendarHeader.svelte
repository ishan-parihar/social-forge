<script lang="ts">
  import type { CalendarView } from "./types";
  import type { Tag } from "$lib/api/tags";
  import type { Integration } from "$lib/api/integrations";
  import type { Campaign } from "$lib/api/campaigns";

  let { view, currentDate, onPrev, onNext, onToday, onViewChange, tags = [], selectedTagId = null, onTagFilter,
         // v24-2: channel + campaign filter props.
         integrations = [], selectedIntegrationId = null, onIntegrationFilter,
         campaigns = [], selectedCampaignId = null, onCampaignFilter }: {
    view: CalendarView; currentDate: Date;
    onPrev: () => void; onNext: () => void; onToday: () => void;
    onViewChange: (v: CalendarView) => void;
    tags?: Tag[];
    selectedTagId?: string | null;
    onTagFilter?: (tagId: string | null) => void;
    // v24-2: channel + campaign filter props.
    integrations?: Integration[];
    selectedIntegrationId?: string | null;
    onIntegrationFilter?: (id: string | null) => void;
    campaigns?: Campaign[];
    selectedCampaignId?: string | null;
    onCampaignFilter?: (id: string | null) => void;
  } = $props();

  const views: { key: CalendarView; label: string; icon: string }[] = [
    { key: "day", label: "Day", icon: "\u{1F4C5}" },
    { key: "week", label: "Week", icon: "\u{1F4C6}" },
    { key: "month", label: "Month", icon: "\u{1F4C5}" },
    { key: "list", label: "List", icon: "\u{1F4CB}" },
  ];

  let title = $derived.by(() => {
    const d = currentDate;
    const m = d.toLocaleDateString("en-US", { month: "long" });
    if (view === "month") return `${m} ${d.getFullYear()}`;
    if (view === "week") return `Week of ${m} ${d.getDate()}`;
    if (view === "day") return d.toLocaleDateString("en-US", { weekday: "long", month: "long", day: "numeric" });
    return "Upcoming Posts";
  });
</script>

<div class="flex items-center justify-between flex-wrap gap-2">
  <div class="flex items-center gap-2">
    <button onclick={onPrev} aria-label="Previous" class="px-2 py-1 text-sm text-muted hover:text-content rounded hover:bg-surface-hover">&larr;</button>
    <span class="text-lg font-semibold min-w-[200px] text-center">{title}</span>
    <button onclick={onNext} aria-label="Next" class="px-2 py-1 text-sm text-muted hover:text-content rounded hover:bg-surface-hover">&rarr;</button>
    <button onclick={onToday} aria-label="Go to today" class="px-2 py-1 text-xs bg-surface-hover hover:bg-line-hover rounded">Today</button>
  </div>
  <div class="flex items-center gap-2 flex-wrap">
    <!-- v24-2: channel filter -->
    {#if integrations.length > 0 && onIntegrationFilter}
      <select
        value={selectedIntegrationId ?? ""}
        onchange={(e) => onIntegrationFilter(e.currentTarget.value || null)}
        class="px-2 py-1.5 bg-background-input border border-line rounded-lg text-xs text-content focus:outline-none focus:border-brand-500"
      >
        <option value="">All channels</option>
        {#each integrations as integ (integ.id)}
          <option value={integ.id}>{integ.provider_name || integ.provider_identifier}</option>
        {/each}
      </select>
    {/if}
    <!-- v24-2: campaign filter -->
    {#if campaigns.length > 0 && onCampaignFilter}
      <select
        value={selectedCampaignId ?? ""}
        onchange={(e) => onCampaignFilter(e.currentTarget.value || null)}
        class="px-2 py-1.5 bg-background-input border border-line rounded-lg text-xs text-content focus:outline-none focus:border-brand-500"
      >
        <option value="">All campaigns</option>
        {#each campaigns as camp (camp.id)}
          <option value={camp.id}>{camp.name}</option>
        {/each}
      </select>
    {/if}
    {#if tags.length > 0 && onTagFilter}
      <select
        value={selectedTagId ?? ""}
        onchange={(e) => onTagFilter(e.currentTarget.value || null)}
        class="px-2 py-1.5 bg-background-input border border-line rounded-lg text-xs text-content focus:outline-none focus:border-brand-500"
      >
        <option value="">All tags</option>
        {#each tags as tag}
          <option value={tag.id}>{tag.name}</option>
        {/each}
      </select>
    {/if}
    <div class="flex bg-surface rounded-lg border border-line overflow-hidden">
      {#each views as v}
        <button
          onclick={() => onViewChange(v.key)}
          aria-label={`${v.label} view`}
          class="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium capitalize transition-colors
            {view === v.key ? 'bg-brand-500 text-white' : 'text-muted hover:text-content hover:bg-surface-hover'}"
        ><span aria-hidden="true">{v.icon}</span> {v.label}</button>
      {/each}
    </div>
  </div>
</div>
