<script lang="ts">
  import { onMount } from 'svelte';
  import { createEditor, EditorContent } from 'svelte-tiptap';
  import type { Editor } from 'svelte-tiptap';
  import StarterKit from '@tiptap/starter-kit';
  import Placeholder from '@tiptap/extension-placeholder';
  import Link from '@tiptap/extension-link';
  import { ImageExtension } from './extensions/ImageExtension';
  import LinkEditor from './extensions/LinkEditor.svelte';
  import EmojiPicker from './extensions/EmojiPicker.svelte';
  import MentionPicker from './extensions/MentionPicker.svelte';
  import SignatureEditor from './SignatureEditor.svelte';

  let { content = "", placeholder = "Write your post...", onUpdate, integrationId }: {
    content?: string; placeholder?: string;
    onUpdate?: (html: string) => void;
    /** v26-5: integration ID for @mention suggestions. Optional — if omitted,
     *  the MentionPicker won't activate. */
    integrationId?: string;
  } = $props();

  let editor = $state<Editor | null>(null);
  let charCount = $state(0);
  let showLinkEditor = $state(false);
  let showEmojiPicker = $state(false);
  let showImageInput = $state(false);
  let imageUrl = $state('');
  let syncing = false;

  // Note: Placeholder config is captured once at init — TipTap doesn't support reactive placeholder updates
  const extensions = [
    StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
    Placeholder.configure({ placeholder }),
    Link.configure({
      openOnClick: false,
      HTMLAttributes: {
        rel: 'noopener noreferrer nofollow',
      },
    }),
    ImageExtension,
  ];

  function toggleLinkEditor() {
    showLinkEditor = !showLinkEditor;
    showEmojiPicker = false;
    showImageInput = false;
  }

  function toggleEmojiPicker() {
    showEmojiPicker = !showEmojiPicker;
    showLinkEditor = false;
    showImageInput = false;
  }

  function toggleImageInput() {
    showImageInput = !showImageInput;
    showLinkEditor = false;
    showEmojiPicker = false;
    if (!showImageInput) {
      imageUrl = '';
    }
  }

  // Guarded by {#if ... && editor} in the template — editor is always available when UI is visible
  function insertImage() {
    if (!editor || !imageUrl.trim()) return;
    editor.chain().focus().setImage({ src: imageUrl.trim() }).run();
    imageUrl = '';
    showImageInput = false;
  }

  // Guarded by {#if ... && editor} in the template — editor is always available when UI is visible
  function insertEmoji(emoji: string) {
    if (!editor) return;
    editor.chain().focus().insertContent(emoji).run();
  }

  function closeImageInput() {
    showImageInput = false;
    imageUrl = '';
  }

  function handleImageKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      insertImage();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      closeImageInput();
    }
  }

  function handleImageBackdropClick(e: MouseEvent) {
    if ((e.target as HTMLElement).classList.contains('img-input-backdrop')) {
      closeImageInput();
    }
  }

  onMount(() => {
    const store = createEditor({
      extensions,
      content,
      onUpdate: ({ editor: ed }) => {
        if (syncing) return;
        onUpdate?.(ed.getHTML());
        charCount = ed.getText().length;
      },
    });

    const unsub = store.subscribe(val => {
      editor = val;
      if (val) charCount = val.getText().length;
    });

    return () => {
      unsub();
      editor?.destroy();
    };
  });

  // Sync external content changes into the editor (guarded to avoid loops)
  $effect(() => {
    if (editor && content && content !== editor.getHTML() && !syncing) {
      syncing = true;
      editor.commands.setContent(content);
      syncing = false;
    }
  });
</script>

<div class="rich-editor border border-line rounded-lg overflow-hidden bg-background-input relative">
  <div class="flex items-center gap-1 p-2 border-b border-line flex-wrap">
    <button aria-label="Bold" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleBold().run(); }} class="toolbar-btn" class:active={editor?.isActive("bold")}>B</button>
    <button aria-label="Italic" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleItalic().run(); }} class="toolbar-btn italic" class:active={editor?.isActive("italic")}>I</button>
    <!-- v24-6: Underline button (TipTap 3.x includes Underline in StarterKit) -->
    <button aria-label="Underline" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleUnderline().run(); }} class="toolbar-btn underline" class:active={editor?.isActive("underline")}>U</button>
    <button aria-label="Heading 2" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleHeading({ level: 2 }).run(); }} class="toolbar-btn" class:active={editor?.isActive("heading", { level: 2 })}>H2</button>
    <button aria-label="Bullet list" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleBulletList().run(); }} class="toolbar-btn" class:active={editor?.isActive("bulletList")}>• List</button>
    <button aria-label="Ordered list" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleOrderedList().run(); }} class="toolbar-btn" class:active={editor?.isActive("orderedList")}>1. List</button>
    <span class="text-line-hover mx-1">|</span>
    <button aria-label="Insert image" onmousedown={(e) => { e.preventDefault(); toggleImageInput(); }} class="toolbar-btn" class:active={showImageInput}>🖼️</button>
    <button aria-label="Insert or edit link" onmousedown={(e) => { e.preventDefault(); toggleLinkEditor(); }} class="toolbar-btn" class:active={showLinkEditor || editor?.isActive("link")}>🔗</button>
    <button aria-label="Insert emoji" onmousedown={(e) => { e.preventDefault(); toggleEmojiPicker(); }} class="toolbar-btn" class:active={showEmojiPicker}>😊</button>
    <span class="text-line-hover mx-1">|</span>
    <SignatureEditor onInsert={(content) => editor?.chain().focus().insertContent(content).run()} />

    <span class="text-xs text-muted ml-auto">{charCount} chars</span>
  </div>

  {#if showImageInput}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="img-input-backdrop fixed inset-0 z-40" role="none" onclick={handleImageBackdropClick}>
      <div
        class="img-input-popover"
        role="dialog"
        aria-label="Insert image URL"
        tabindex="-1"
        onclick={(e) => e.stopPropagation()}
      >
        <label for="img-url" class="block text-xs text-muted mb-1">Image URL</label>
        <div class="flex items-center gap-2">
          <input
            id="img-url"
            type="text"
            placeholder="https://example.com/image.png"
            bind:value={imageUrl}
            onkeydown={handleImageKeydown}
            class="flex-1 px-3 py-2 rounded text-sm bg-background-input border border-line text-content-secondary placeholder:text-muted-dark outline-none focus:border-brand-500 transition-colors"
          />
          <button
            onclick={insertImage}
            disabled={!imageUrl.trim()}
            class="px-3 py-2 rounded text-xs font-medium bg-brand-500 text-white hover:bg-brand-600 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            aria-label="Insert image"
          >
            Insert
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if showLinkEditor && editor}
    <LinkEditor {editor} onClose={() => showLinkEditor = false} />
  {/if}

  {#if showEmojiPicker}
    <EmojiPicker onSelect={insertEmoji} onClose={() => showEmojiPicker = false} />
  {/if}

  <!-- v26-5: @mention suggestion popup. Renders above everything; positions
       itself at the cursor via fixed positioning. Only active when
       integrationId is provided. -->
  <MentionPicker {editor} {integrationId} />

  <div class="p-3 min-h-[200px] prose prose-invert max-w-none">
    {#if editor}
      <EditorContent {editor} />
    {/if}
  </div>
</div>

<style>
  /* v24-6: use CSS variables so the editor rethemes in light mode. */
  .toolbar-btn {
    padding: 0.25rem 0.5rem; font-size: 0.8rem; background: transparent;
    border: 1px solid transparent; border-radius: 0.25rem; cursor: pointer;
    color: var(--text-muted); font-weight: 500;
  }
  .toolbar-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .toolbar-btn.active { background: var(--brand); color: white; border-color: var(--brand); }
  .italic { font-style: italic; }
  .underline { text-decoration: underline; }

  .img-input-popover {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 0.875rem;
    width: 24rem;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
    z-index: 50;
  }

  .img-input-backdrop {
    background: rgba(0, 0, 0, 0.3);
  }
</style>
