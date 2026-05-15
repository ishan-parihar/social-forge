<script lang="ts">
  let { open = false, title = "", onclose, children }: {
    open?: boolean; title?: string; onclose?: () => void;
    children?: import("svelte").Snippet;
  } = $props();
</script>

{#if open}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 flex items-center justify-center" role="dialog">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="absolute inset-0 bg-black/60" onclick={onclose}></div>
    <div class="relative bg-[#131720] border border-[#1e2435] rounded-xl p-6 max-w-lg w-full mx-4 shadow-2xl">
      {#if title}
        <div class="flex items-center justify-between mb-4">
          <h3 class="text-lg font-semibold">{title}</h3>
          <button onclick={onclose} class="text-[#6b7280] hover:text-white">&times;</button>
        </div>
      {/if}
      {#if children}{@render children()}{/if}
    </div>
  </div>
{/if}
