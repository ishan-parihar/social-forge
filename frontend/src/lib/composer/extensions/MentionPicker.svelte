<script lang="ts">
  // v26-5: MentionPicker — a custom @mention suggestion popup for TipTap.
  //
  // No new deps. Instead of installing @tiptap/extension-mention, this
  // component listens to editor updates, detects the `@query` pattern at
  // the cursor position, fetches suggestions from the backend
  // /api/integrations/{id}/mentions endpoint, and shows a floating
  // dropdown. On select, it replaces the `@query` text with the
  // mention's `formatted` string (e.g. `@username`).
  //
  // This is a pragmatic approach: the mention is inserted as plain text,
  // not as a decorated node. The platform API will parse `@username` on
  // its end. This avoids the complexity of a custom TipTap node type
  // while still giving the user the suggestion UX.
  //
  // Keyboard: ↑/↓ to navigate, Enter to select, Escape to close.

  import { onMount, onDestroy } from 'svelte';
  import type { Editor } from 'svelte-tiptap';
  import { integrationsApi } from '$lib/api/integrations';

  let { editor, integrationId }: { editor: Editor | null; integrationId?: string } = $props();

  let open = $state(false);
  let query = $state('');
  let results = $state<Array<{ id: string; label: string; formatted: string; image?: string | null; provider: string }>>([]);
  let selectedIndex = $state(0);
  let popupX = $state(0);
  let popupY = $state(0);
  let loading = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  // Range to replace when a mention is selected: { from, to } in prosemirror positions.
  let replaceRange: { from: number; to: number } | null = null;

  // Detect @mention pattern at the cursor position.
  // The pattern: `@` followed by 1-20 word chars, not preceded by another word char
  // (so `email@test` doesn't trigger).
  function detectMention(ed: Editor): { query: string; from: number; to: number } | null {
    const { to } = ed.state.selection;
    const textBefore = ed.state.doc.textBetween(Math.max(0, to - 30), to, '\n', '\0');
    // Match @word at the end of the text. The @ must be at a word boundary
    // (preceded by whitespace, start of line, or nothing).
    const match = textBefore.match(/(?:^|\s)@(\w{1,20})$/);
    if (!match) return null;
    const queryStr = match[1];
    // Calculate the absolute position of the @ in the document.
    // textBefore.length - match[0].length + match[0].indexOf('@') gives the
    // offset of @ within the last 30 chars. Add the start position to get absolute.
    const atOffsetInText = match[0].indexOf('@');
    const atAbsolutePos = to - (textBefore.length - atOffsetInText);
    return { query: queryStr, from: atAbsolutePos, to };
  }

  // Position the popup at the cursor's screen coordinates.
  function positionAtCursor(ed: Editor) {
    try {
      const coords = ed.view.coordsAtPos(ed.state.selection.to);
      popupX = coords.left;
      popupY = coords.bottom + 4;
    } catch {
      // If coords fails (e.g. editor not focused), don't show popup.
      open = false;
    }
  }

  // Fetch suggestions from the backend (debounced).
  function fetchSuggestions(integId: string, q: string) {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(async () => {
      loading = true;
      try {
        const r = await integrationsApi.searchMentions(integId, q);
        if (r.data) {
          results = r.data.results;
          selectedIndex = 0;
          open = results.length > 0;
        } else {
          results = [];
          open = false;
        }
      } catch {
        results = [];
        open = false;
      } finally {
        loading = false;
      }
    }, 250);
  }

  // Listen to editor updates.
  $effect(() => {
    if (!editor) return;
    const checkMention = () => {
      if (!editor || !editor.isFocused) { open = false; return; }
      if (!integrationId) { open = false; return; }
      const detected = detectMention(editor);
      if (!detected) { open = false; return; }
      query = detected.query;
      replaceRange = { from: detected.from, to: detected.to };
      positionAtCursor(editor);
      fetchSuggestions(integrationId, detected.query);
    };
    // TipTap emits 'transaction' on every content/selection change.
    const handler = () => checkMention();
    editor.on('transaction', handler);
    editor.on('blur', () => { setTimeout(() => { open = false; }, 200); });
    return () => {
      editor.off('transaction', handler);
    };
  });

  function selectMention(idx: number) {
    if (!editor || !replaceRange || idx < 0 || idx >= results.length) return;
    const mention = results[idx];
    // Insert the formatted mention text (e.g. "@username") as plain text.
    // The platform API will parse it on publish.
    editor.chain()
      .focus()
      .deleteRange({ from: replaceRange.from, to: replaceRange.to })
      .insertContent(mention.formatted + ' ')
      .run();
    open = false;
    results = [];
    replaceRange = null;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open || results.length === 0) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % results.length;
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = selectedIndex <= 0 ? results.length - 1 : selectedIndex - 1;
    } else if (e.key === 'Enter') {
      e.preventDefault();
      selectMention(selectedIndex);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      open = false;
    }
  }

  // Global keydown listener (the editor captures keys, so we listen at document level
  // and only act when the popup is open).
  $effect(() => {
    if (!open) return;
    document.addEventListener('keydown', handleKeydown, true);
    return () => document.removeEventListener('keydown', handleKeydown, true);
  });

  // v25 F5: aria-activedescendant on the editor.
  // The mention list is the textbook activedescendant pattern: focus MUST stay
  // in the caret or typing breaks, so the options can never receive it. But
  // nothing announced which option the arrow keys had moved to — the popup
  // updated silently and Enter picked something the user could not see
  // highlighted. Pointing the editor's own aria-activedescendant at the
  // selected option is what makes the existing key handling perceivable.
  // Attributes are removed on close so the editor is not left claiming an
  // expanded combobox when no popup is showing.
  $effect(() => {
    const dom = editor?.view?.dom as HTMLElement | undefined;
    if (!dom) return;
    const active = open ? results[selectedIndex] : undefined;
    if (active) {
      dom.setAttribute('role', 'combobox');
      dom.setAttribute('aria-autocomplete', 'list');
      dom.setAttribute('aria-expanded', 'true');
      dom.setAttribute('aria-controls', 'mention-listbox');
      dom.setAttribute('aria-activedescendant', `mention-opt-${active.id}`);
    } else {
      for (const a of ['role', 'aria-autocomplete', 'aria-expanded', 'aria-controls', 'aria-activedescendant']) {
        dom.removeAttribute(a);
      }
    }
  });
</script>

{#if open && (results.length > 0 || loading)}
  <div
    class="fixed z-[100] bg-surface border border-line rounded-lg shadow-lg max-h-60 overflow-y-auto w-[min(16rem,calc(100vw-2rem))]"
    style="left: {popupX}px; top: {popupY}px;"
    role="listbox"
    id="mention-listbox"
    aria-label="Mention suggestions"
    aria-busy={loading}
  >
    {#if loading}
      <!-- v25 F5: role=status. The roving selection below is driven from the
           editor (focus must stay in the caret to keep typing), so the whole
           popup's state has to be announced — this was the one piece of the
           interaction with no channel to AT at all. -->
      <div class="px-3 py-2 text-xs text-muted" role="status">Searching...</div>
    {:else}
      {#each results as result, i (result.id + i)}
        <button
          type="button"
          onclick={() => selectMention(i)}
          onmouseenter={() => selectedIndex = i}
          class="w-full flex items-center gap-2 px-3 py-2 text-left text-sm transition-colors {i === selectedIndex ? 'bg-surface-hover' : ''}"
          role="option"
          id={`mention-opt-${result.id}`}
          aria-selected={i === selectedIndex}
        >
          {#if result.image}
            <img src={result.image} alt="" class="w-5 h-5 rounded-full shrink-0" />
          {:else}
            <div class="w-5 h-5 rounded-full bg-accent-fill/20 text-accent text-xs flex items-center justify-center shrink-0 font-medium">
              {result.label.charAt(0).toUpperCase()}
            </div>
          {/if}
          <div class="min-w-0 flex-1">
            <div class="text-content truncate">{result.label}</div>
            <div class="text-[10px] text-muted truncate">{result.formatted}</div>
          </div>
          <span class="text-[10px] text-faint uppercase">{result.provider}</span>
        </button>
      {/each}
    {/if}
  </div>
{/if}
