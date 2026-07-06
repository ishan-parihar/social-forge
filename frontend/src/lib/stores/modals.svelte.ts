// Global modal store with stacking (Phase 0 — keystone foundation).
//
// This module provides a Svelte 5 runes-based modal manager that supports:
//   - A stack of modals (zIndex = 200 + index)
//   - Escape closes only the topmost modal
//   - Backdrop click closes the topmost (with askClose confirm if set)
//   - Body scroll lock when stack is non-empty
//   - Promise-based areYouSure() helper
//   - open(component, props, options) returns a modal id
//   - close(id), closeAll(), closeCurrent()
//
// Inspired by postiz-app's useModalStore (Zustand) but adapted for Svelte 5
// runes and Social Forge's architectural preferences (no external deps,
// no Mantine, no Headless UI — just Svelte + Tailwind).
//
// Usage:
//   import { modals } from '$lib/stores/modals.svelte';
//   import ComposerModal from '$lib/composer/ComposerModal.svelte';
//
//   // Open a modal:
//   const id = modals.open(ComposerModal, { date: '2026-07-10' }, {
//     title: 'Create Post',
//     closeOnClickOutside: false,
//     askClose: true,
//     size: '80%',
//   });
//
//   // Close it:
//   modals.close(id);
//
//   // Promise-based confirm:
//   const ok = await modals.areYouSure({
//     title: 'Delete post?',
//     message: 'This cannot be undone.',
//     confirmLabel: 'Delete',
//     danger: true,
//   });

import type { Component, Snippet } from 'svelte';

export interface ModalOptions {
  /** Optional title shown in the modal header. If empty, no header is rendered. */
  title?: string;
  /** Click outside the modal to close? Default: true */
  closeOnClickOutside?: boolean;
  /** Escape key closes the topmost modal? Default: true */
  closeOnEscape?: boolean;
  /** Show a close (×) button in the header? Default: true (only if title is set) */
  withCloseButton?: boolean;
  /** Ask for confirmation before closing? The modal's onclose callback
   *  receives a `confirmed: boolean` flag. Default: false */
  askClose?: boolean;
  /** Max width CSS class or value. Examples: 'max-w-lg', 'max-w-4xl', '80%'.
   *  Default: 'max-w-lg' */
  size?: string;
  /** Full-screen modal (no max-width, fills viewport with padding)? Default: false */
  fullScreen?: boolean;
  /** Called when the modal is about to close. If askClose is true, this
   *  receives `confirmed: false` for backdrop/escape and `confirmed: true`
   *  for explicit close-button clicks. Return false to abort the close. */
  onClose?: (confirmed: boolean) => boolean | void;
  /** Custom class for the modal panel. */
  panelClass?: string;
}

export interface ModalEntry {
  /** Unique id (auto-generated). Used by close(id). */
  id: string;
  /** The Svelte component to render as the modal body. */
  component: Component<any, any, any> | null;
  /** Props to pass to the component. The component also receives a
   *  `close` function it can call to close itself. */
  props: Record<string, any>;
  /** Options. */
  options: ModalOptions;
  /** The rendered children snippet (alternative to component). */
  snippet?: Snippet;
  /** Z-index for this modal (computed when pushed). */
  zIndex: number;
  /** Whether this is the topmost modal (computed). */
  isTop: boolean;
}

let _idCounter = 0;
function nextId(): string {
  return `modal-${++_idCounter}-${Date.now().toString(36)}`;
}

class ModalStore {
  /** The modal stack. Index 0 is the bottom; last index is the top. */
  stack = $state<ModalEntry[]>([]);

  /** Derived: is any modal open? */
  get isOpen(): boolean {
    return this.stack.length > 0;
  }

  /** Derived: the topmost modal, or null. */
  get top(): ModalEntry | null {
    return this.stack.length > 0 ? this.stack[this.stack.length - 1] : null;
  }

  /** Open a modal by passing a Svelte component + props.
   *  Returns the modal id (use with close(id)). */
  open(
    component: Component<any, any, any>,
    props: Record<string, any> = {},
    options: ModalOptions = {},
  ): string {
    const id = nextId();
    const entry: ModalEntry = {
      id,
      component,
      props,
      options,
      zIndex: 0, // recomputed below
      isTop: true,
    };
    this.stack = [...this.stack, entry];
    this._recompute();
    return id;
  }

  /** Open a modal by passing a snippet (for inline use without a component).
   *  Returns the modal id. */
  openSnippet(snippet: Snippet, options: ModalOptions = {}): string {
    const id = nextId();
    const entry: ModalEntry = {
      id,
      component: null,
      props: {},
      options,
      snippet,
      zIndex: 0,
      isTop: true,
    };
    this.stack = [...this.stack, entry];
    this._recompute();
    return id;
  }

  /** Close a specific modal by id. Calls the modal's onClose callback. */
  close(id: string, confirmed: boolean = false): void {
    const entry = this.stack.find(e => e.id === id);
    if (!entry) return;

    // Give the onClose callback a chance to abort.
    if (entry.options.onClose) {
      const result = entry.options.onClose(confirmed);
      if (result === false) return; // abort close
    }

    this.stack = this.stack.filter(e => e.id !== id);
    this._recompute();
  }

  /** Close the topmost modal. */
  closeCurrent(confirmed: boolean = false): void {
    if (this.stack.length === 0) return;
    this.close(this.stack[this.stack.length - 1].id, confirmed);
  }

  /** Close all modals (no onClose callbacks invoked — force close). */
  closeAll(): void {
    this.stack = [];
  }

  /** Promise-based confirm dialog. Renders a small confirm modal on top
   *  of the stack. Resolves to true if confirmed, false if cancelled. */
  areYouSure(opts: {
    title?: string;
    message?: string;
    confirmLabel?: string;
    cancelLabel?: string;
    danger?: boolean;
  }): Promise<boolean> {
    return new Promise<boolean>((resolve) => {
      // Inline confirm modal component — we render it via a snippet
      // in ModalManager. To keep this store framework-agnostic, we
      // stash the resolver in a pending-confirm slot that ModalManager
      // reads.
      const confirmId = `confirm-${nextId()}`;
      this._pendingConfirm = {
        id: confirmId,
        opts,
        resolve: (ok: boolean) => {
          resolve(ok);
          this._pendingConfirm = null;
        },
      };
    });
  }

  /** Internal: pending confirm dialog state (read by ModalManager). */
  _pendingConfirm: {
    id: string;
    opts: {
      title?: string;
      message?: string;
      confirmLabel?: string;
      cancelLabel?: string;
      danger?: boolean;
    };
    resolve: (ok: boolean) => void;
  } | null = null;

  /** Internal: recompute zIndex + isTop for all stack entries. */
  private _recompute(): void {
    this.stack = this.stack.map((entry, idx) => ({
      ...entry,
      zIndex: 200 + idx,
      isTop: idx === this.stack.length - 1,
    }));
  }
}

export const modals = new ModalStore();

/** Convenience helper: open a confirm modal and await the result.
 *  Usage: `if (await confirmModal({ title: 'Delete?' })) { ... }` */
export function confirmModal(opts: {
  title?: string;
  message?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}): Promise<boolean> {
  return modals.areYouSure(opts);
}
