// v25 F5 - one focus trap, shared by every dialog surface.
//
// Why a shared primitive: before F5, each modal hand-rolled its own partial
// version. `lib/ui/Modal.svelte` moved focus in and out but let Tab walk out
// of the dialog into the page behind it; `lib/components/ModalManager.svelte`
// did not move focus at all, so opening a modal dropped the caret on <body>
// the first Tab went to the top nav, not the dialog. A dialog that does not
// trap focus is not keyboard-usable, and with two independent modal systems
// the fix had to land twice or drift.
//
// Mechanics:
//   - Tab / Shift+Tab wrap at the ends of the dialog's focusable set.
//   - On enable, focus moves to the first focusable child; if the dialog has
//     none (a pure-confirm dialog, say) the dialog node itself takes focus so
//     the caret can never end up on the page behind.
//   - On DESTROY, focus returns to whatever was focused before the dialog
//     opened. On plain disable (the modal was demoted because another one
//     opened on top) focus is deliberately NOT restored: the incoming modal
//     has already taken focus, and yanking it back would fight it.
//
// The selector is the standard visible-focusable set. `getClientRects()` is
// the visibility test rather than `offsetParent` because `offsetParent` is
// null for `position: fixed` elements, which the dialog scrims and panels are.

const FOCUSABLE = [
  'a[href]',
  'area[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  'iframe',
  'audio[controls]',
  'video[controls]',
  '[contenteditable]:not([contenteditable="false"])',
  '[tabindex]:not([tabindex^="-"])',
].join(',');

/**
 * Focusable descendants of `root`, in DOM order, skipping anything hidden.
 * An element is hidden if it is inside an `aria-hidden` subtree (a decoy
 * scrim, an inactive modal) or has no layout boxes.
 */
export function focusableWithin(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (el) => !el.closest('[aria-hidden="true"]') && el.getClientRects().length > 0,
  );
}

export interface FocusTrapParams {
  /** When false the trap stands down without releasing focus. Reactive. */
  enabled?: boolean;
  /** Tab to this node when the dialog has no focusable children. Default true. */
  focusSelf?: boolean;
  /**
   * Called when the user presses Escape. OPTIONAL, and off by default.
   *
   * The trap does NOT own Escape. A dialog's Escape handler belongs on
   * the PANEL, not on a wrapper: the panel holds the focus, and a
   * keydown from the panel's children bubbles to it. A handler on the
   * wrapper only ever saw a keydown that nobody had consumed, which
   * stopped working the moment the focus moved onto a child.
   *
   * The trap also must not STOP the event: the modal manager has its
   * own window-level Escape handler, and a dialog may have its own.
   * Two handlers for one keystroke fight - the composer asked "Discard
   * this post?" and the SAME keystroke answered it, so the confirm
   * appeared and vanished in one frame.
   */
  onEscape?: () => void;
}

/**
 * Svelte action. Usage: `use:focusTrap` or `use:focusTrap={{ enabled: isTop }}`.
 * Pass the node `tabindex="-1"` so it can receive focus as the fallback target.
 */
export function focusTrap(node: HTMLElement, params: FocusTrapParams | boolean = {}) {
  const opts: FocusTrapParams = typeof params === 'boolean' ? { enabled: params } : params;
  let active = false;
  let prevFocus: HTMLElement | null = null;

  function onKeydown(e: KeyboardEvent) {
    if (!active) return;
    if (e.key !== 'Tab') return;
    const items = focusableWithin(node);
    if (items.length === 0) {
      // Nothing to move to. Hold focus here rather than letting it escape.
      if (opts.focusSelf !== false) {
        e.preventDefault();
        node.focus();
      }
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const current = document.activeElement as HTMLElement | null;
    // `!node.contains(current)` catches focus that already escaped (e.g. the
    // user clicked the page behind): pull it back to the correct end.
    if (e.shiftKey) {
      if (current === first || !node.contains(current)) {
        e.preventDefault();
        last.focus();
      }
    } else if (current === last || !node.contains(current)) {
      e.preventDefault();
      first.focus();
    }
  }

  function enable() {
    if (active) return;
    active = true;
    prevFocus = (document.activeElement as HTMLElement | null) ?? null;
    // Capture phase: a dialog that stops propagation (the media carousel
    // carousel does, for arrow keys) must not be able to swallow Tab.
    document.addEventListener('keydown', onKeydown, true);
    focusFirst();
    watchContent();
  }

  /**
   * Keep the focus inside the dialog when its CONTENT is replaced.
   *
   * A dialog that loads its content asynchronously re-renders when the
   * data lands, and the re-render DESTROYS the element that had the
   * focus. The browser then puts the focus on `<body>` - which is
   * OUTSIDE the dialog.
   *
   * The composer is one: its `onMount` fetches the integrations, the
   * tags and the brand profile. The trap focused the first control, and
   * a moment later the content was replaced and the focus was gone. The
   * founder's first keystroke went to the page BEHIND the dialog.
   *
   * So the trap watches for the focused element being removed, and
   * puts the focus back on the dialog's first control. This is the
   * same reason a dialog needs a focus trap at all: the focus must
   * never be able to leave.
   */
  let observer: MutationObserver | null = null;

  function watchContent() {
    observer?.disconnect();
    observer = new MutationObserver(() => {
      if (!active) return;
      const current = document.activeElement as HTMLElement | null;
      // The focus is still inside the dialog - nothing to do.
      if (current && current !== document.body && node.contains(current)) return;
      const items = focusableWithin(node);
      if (items.length > 0) items[0].focus();
      else if (opts.focusSelf !== false) node.focus();
    });
    observer.observe(node, { childList: true, subtree: true });
  }

  /**
   * Move the focus to the dialog's first focusable child.
   *
   * A dialog that renders its content ASYNCHRONOUSLY has no focusable
   * child at the moment the trap is enabled. The composer is one: its
   * `onMount` fetches the integrations, the tags and the brand
   * profile, and until those land it renders a spinner - so a single
   * frame after enabling there was nothing to focus, and the focus
   * stayed on `<body>`.
   *
   * The consequence was the worst kind: the dialog was open, it looked
   * right, and the founder's first keystroke went to the PAGE BEHIND
   * it. The caret was invisible and the typing went nowhere.
   *
   * So the trap waits for a focusable child, polling a few frames. It
   * gives up after the window and falls back to the dialog node itself,
   * which still contains the focus.
   */
  function focusFirst() {
    // Hold the focus on the dialog node FIRST, so the focus is never on
    // the page behind it. A dialog that is still loading has no
    // focusable child, and the focus sitting on `<body>` means the
    // founder's first keystroke goes to the page behind the dialog.
    if (opts.focusSelf !== false) node.focus();

    // Then move to the first focusable child, waiting for it if the
    // dialog renders its content asynchronously.
    const deadline = 60;
    let tries = 0;
    const attempt = () => {
      if (!active) return;
      const items = focusableWithin(node);
      if (items.length > 0) {
        items[0].focus();
        return;
      }
      tries += 1;
      if (tries >= deadline) return; // the focus stays on the dialog node
      requestAnimationFrame(attempt);
    };
    requestAnimationFrame(attempt);
  }

  function disable() {
    if (!active) return;
    active = false;
    document.removeEventListener('keydown', onKeydown, true);
    observer?.disconnect();
    observer = null;
    prevFocus = null;
  }

  if (opts.enabled !== false) enable();

  return {
    update(next: FocusTrapParams | boolean) {
      const o: FocusTrapParams = typeof next === 'boolean' ? { enabled: next } : next;
      Object.assign(opts, o);
      if (opts.enabled === false) disable();
      else enable();
    },
    destroy() {
      const back = prevFocus;
      disable();
      // Restore only on teardown (see header): a demotion must not steal focus
      // back from the modal that replaced it.
      if (back && document.contains(back)) {
        requestAnimationFrame(() => {
          if (document.contains(back)) back.focus();
        });
      }
    },
  };
}
