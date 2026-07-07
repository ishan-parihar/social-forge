<script lang="ts">
  import type { Editor } from 'svelte-tiptap';

  interface Props {
    editor: Editor;
    onClose: () => void;
  }

  let { editor, onClose }: Props = $props();

  let url = $state('');
  let newTab = $state(false);
  let linkText = $state('');
  let urlInput = $state<HTMLInputElement | null>(null);

  // Populate fields when the editor already has a link selected
  $effect(() => {
    const attrs = editor.getAttributes('link');
    url = attrs.href || '';
    newTab = attrs.target === '_blank';
    linkText = editor.state.doc.textBetween(
      editor.state.selection.from,
      editor.state.selection.to,
      ' ',
    );
  });

  $effect(() => {
    urlInput?.focus();
  });

  function applyLink() {
    if (!editor) return;
    const href = url.trim();
    if (href && !/^https?:\/\//i.test(href)) {
      return; // silently reject non-http(s) URLs
    }
    const chain = editor.chain().focus().extendMarkRange('link');
    if (href) {
      chain.setLink({
        href,
        target: newTab ? '_blank' : null,
        rel: 'noopener noreferrer nofollow',
      });
    }
    chain.run();
    onClose();
  }

  function removeLink() {
    editor.chain().focus().unsetLink().run();
    onClose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  }

  function handleBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('link-editor-backdrop')) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="link-editor-backdrop fixed inset-0 z-40" role="none" onclick={handleBackdropClick}>
  <div
    class="link-editor-popover"
    role="dialog"
    aria-label="Edit link"
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
  >
    {#if linkText}
      <p class="text-xs text-muted mb-2 truncate">
        Text: <span class="text-content-secondary">{linkText}</span>
      </p>
    {/if}

    <label for="link-url" class="block text-xs text-muted mb-1">URL</label>
    <input
      id="link-url"
      type="text"
      placeholder="https://example.com"
      bind:this={urlInput}
      bind:value={url}
      class="w-full px-3 py-2 rounded text-sm bg-background-input border border-line text-content-secondary placeholder:text-muted-dark outline-none focus:border-brand-500 transition-colors"
    />

    <label class="flex items-center gap-2 mt-2 cursor-pointer select-none">
      <input
        type="checkbox"
        bind:checked={newTab}
        class="accent-brand-500 w-4 h-4"
      />
      <span class="text-xs text-muted">Open in new tab</span>
    </label>

    <div class="flex items-center gap-2 mt-3">
      <button
        onclick={applyLink}
        class="flex-1 px-3 py-1.5 rounded text-xs font-medium bg-brand-500 text-white hover:bg-brand-600 transition-colors"
        aria-label="Apply link"
      >
        Apply
      </button>
      <button
        onclick={removeLink}
        class="px-3 py-1.5 rounded text-xs font-medium border border-line text-content-secondary hover:bg-surface-hover transition-colors"
        aria-label="Remove link"
      >
        Remove
      </button>
    </div>
  </div>
</div>

<style>
  .link-editor-popover {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.875rem;
    width: 18rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 50;
  }

  .link-editor-backdrop {
    background: rgba(0, 0, 0, 0.3);
  }
</style>
