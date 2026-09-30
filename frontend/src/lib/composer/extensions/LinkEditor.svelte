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
  // v25 F5: the rejection below used to be a bare `return`.
  let urlError = $state('');

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
      urlError = 'Enter a full URL starting with http:// or https://';
      urlInput?.focus();
      return; // reject non-http(s) URLs, but say so
    }
    urlError = '';
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
    aria-modal="true"
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
    <!-- v25 F5: type="url" (was "text") so the browser validates and mobile
         keyboards show the right layout, plus an error region. `applyLink`
         used to `return` silently on a non-http URL, so a keyboard user could
         press Apply on garbage and get no feedback whatsoever. -->
    <input
      id="link-url"
      type="url"
      placeholder="https://example.com"
      bind:this={urlInput}
      bind:value={url}
      aria-invalid={urlError ? "true" : undefined}
      aria-describedby={urlError ? "link-url-error" : undefined}
      oninput={() => (urlError = "")}
      class="w-full px-3 py-2 rounded text-sm bg-background-input border text-content-secondary placeholder:text-faint outline-none focus:border-accent transition-colors {urlError ? 'border-error' : 'border-line'}"
    />
    {#if urlError}
      <p id="link-url-error" role="alert" class="text-[11px] text-error mt-1">{urlError}</p>
    {/if}

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
        class="flex-1 px-3 py-1.5 rounded text-xs font-medium bg-accent-fill text-accent-fg hover:bg-accent-fill-hover transition-colors"
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
    /* v25 F5: bare `18rem` (288px) inside a card with p-4 padding either side
       left ~264px to fit it. See RichTextEditor .img-input-popover. */
    width: min(18rem, calc(100vw - 2rem));
    box-shadow: var(--shadow-lg);
    z-index: 50;
  }

  .link-editor-backdrop {
    background: var(--overlay);
  }
</style>
