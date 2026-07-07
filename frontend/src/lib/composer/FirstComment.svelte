<script lang="ts">
  let { selectedIntegrations = [], integrationProviders = new Map(), firstComment = "", onFirstCommentChange }: {
    selectedIntegrations?: string[];
    integrationProviders?: Map<string, string>;
    firstComment?: string;
    onFirstCommentChange?: (text: string) => void;
  } = $props();

  let supportsFirstComment = $derived(
    selectedIntegrations.some(id => {
      const provider = integrationProviders.get(id);
      return provider === 'linkedin' || provider === 'facebook';
    })
  );
</script>

{#if supportsFirstComment}
  <div class="bg-surface border border-line rounded-xl p-4 space-y-3">
    <h3 class="text-sm font-semibold flex items-center gap-2">
      <span class="text-brand-400">💬</span>
      First Comment
    </h3>
    <p class="text-xs text-muted">A comment will be posted right after publishing (LinkedIn/Facebook only).</p>
    <textarea
      value={firstComment}
      oninput={(e) => onFirstCommentChange?.((e.target as HTMLTextAreaElement).value)}
      placeholder="Write a first comment..."
      rows="3"
      class="w-full px-3 py-2 bg-background-input border border-line rounded-lg text-sm text-content-secondary focus:border-brand-500 outline-none resize-none"
    ></textarea>
    <div class="text-xs text-muted text-right">{firstComment.length} chars</div>
  </div>
{/if}