<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { createEditor, EditorContent } from 'svelte-tiptap';
  import type { Editor } from 'svelte-tiptap';
  import StarterKit from '@tiptap/starter-kit';
  import Placeholder from '@tiptap/extension-placeholder';
  import Link from '@tiptap/extension-link';

  let { content = "", placeholder = "Write your post...", onUpdate }: {
    content?: string; placeholder?: string;
    onUpdate?: (html: string) => void;
  } = $props();

  let editor = $state<Editor | null>(null);
  let charCount = $state(0);

  const extensions = [
    StarterKit.configure({ heading: { levels: [1, 2, 3] } }),
    Placeholder.configure({ placeholder }),
    Link.configure({ openOnClick: false }),
  ];

  onMount(() => {
    const store = createEditor({
      extensions,
      content,
      onUpdate: ({ editor: ed }) => {
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
</script>

<div class="rich-editor border border-[#1e2435] rounded-lg overflow-hidden bg-[#0d1117]">
  <!-- Toolbar -->
  <div class="flex items-center gap-1 p-2 border-b border-[#1e2435] flex-wrap">
    <button aria-label="Bold" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleBold().run(); }} class="toolbar-btn" class:active={editor?.isActive("bold")}>B</button>
    <button aria-label="Italic" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleItalic().run(); }} class="toolbar-btn italic" class:active={editor?.isActive("italic")}>I</button>
    <button aria-label="Heading 2" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleHeading({ level: 2 }).run(); }} class="toolbar-btn" class:active={editor?.isActive("heading", { level: 2 })}>H2</button>
    <button aria-label="Bullet list" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleBulletList().run(); }} class="toolbar-btn" class:active={editor?.isActive("bulletList")}>• List</button>
    <button aria-label="Ordered list" onmousedown={(e) => { e.preventDefault(); editor?.chain().focus().toggleOrderedList().run(); }} class="toolbar-btn" class:active={editor?.isActive("orderedList")}>1. List</button>
    <span class="text-[#2a3045] mx-1">|</span>
    <span class="text-xs text-[#6b7280] ml-auto">{charCount} chars</span>
  </div>

  <!-- Editor content -->
  <div class="p-3 min-h-[200px] prose prose-invert max-w-none">
    {#if editor}
      <EditorContent {editor} />
    {/if}
  </div>
</div>

<style>
  .toolbar-btn {
    padding: 0.25rem 0.5rem; font-size: 0.8rem; background: transparent;
    border: 1px solid transparent; border-radius: 0.25rem; cursor: pointer;
    color: #9ca3af; font-weight: 500;
  }
  .toolbar-btn:hover { background: #1a1f2e; color: #e5e7eb; }
  .toolbar-btn.active { background: #6366f1; color: white; border-color: #6366f1; }
  .italic { font-style: italic; }
</style>
