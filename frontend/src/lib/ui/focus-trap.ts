// v25 F5 — one focus trap, shared by every dialog surface.
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
    if (!active || e.key !== 'Tab') return;
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
    // One frame of delay so the dialog's children have layout boxes —
    // focusableWithin() filters on getClientRects().
    requestAnimationFrame(() => {
      if (!active) return;
      const target = focusableWithin(node)[0] ?? (opts.focusSelf === false ? null : node);
      target?.focus();
    });
  }

  function disable() {
    if (!active) return;
    active = false;
    document.removeEventListener('keydown', onKeydown, true);
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
