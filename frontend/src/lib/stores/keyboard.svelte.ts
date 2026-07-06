// Global keyboard shortcuts for social-forge.
// Implements a simple sequence matcher for `g <key>` (go-to) shortcuts
// and single-key shortcuts for common actions.
//
// Shortcuts:
//   n       → new post (/posts/new)
//   /       → focus search (or go to /search if not on a page with search)
//   g c     → go to calendar
//   g p     → go to posts
//   g f     → go to feed
//   g a     → go to analytics
//   g m     → go to media
//   ?       → show shortcut cheat-sheet
//   Escape  → close any open modal/panel (handled by individual components)
//
// Shortcuts are disabled when the user is typing in an input/textarea/contenteditable.

import { goto } from '$app/navigation';
import { browser } from '$app/environment';

let gPressed = false;
let gTimer: ReturnType<typeof setTimeout> | null = null;
let showHelpCallback: (() => void) | null = null;

export function initKeyboardShortcuts(showHelp: () => void) {
  if (!browser) return;
  showHelpCallback = showHelp;

  window.addEventListener('keydown', handleKeydown);
}

export function destroyKeyboardShortcuts() {
  if (!browser) return;
  window.removeEventListener('keydown', handleKeydown);
  if (gTimer) clearTimeout(gTimer);
}

function handleKeydown(e: KeyboardEvent) {
  // Don't intercept if typing in an input/textarea/contenteditable
  const target = e.target as HTMLElement;
  if (target && (
    target.tagName === 'INPUT' ||
    target.tagName === 'TEXTAREA' ||
    target.isContentEditable ||
    target.tagName === 'SELECT'
  )) {
    return;
  }

  // Don't intercept if any modifier key is held (Ctrl/Cmd/Alt)
  if (e.ctrlKey || e.metaKey || e.altKey) {
    return;
  }

  const key = e.key.toLowerCase();

  // Handle `g` prefix sequences
  if (key === 'g' && !gPressed) {
    gPressed = true;
    // Reset after 1s if no second key
    if (gTimer) clearTimeout(gTimer);
    gTimer = setTimeout(() => { gPressed = false; }, 1000);
    e.preventDefault();
    return;
  }

  if (gPressed) {
    gPressed = false;
    if (gTimer) { clearTimeout(gTimer); gTimer = null; }
    switch (key) {
      case 'c': goto('/calendar'); e.preventDefault(); return;
      case 'p': goto('/posts'); e.preventDefault(); return;
      case 'f': goto('/feed'); e.preventDefault(); return;
      case 'a': goto('/analytics'); e.preventDefault(); return;
      case 'm': goto('/media'); e.preventDefault(); return;
      case 'd': goto('/'); e.preventDefault(); return;
      default: return;
    }
  }

  // Single-key shortcuts
  switch (key) {
    case 'n':
      goto('/posts/new');
      e.preventDefault();
      return;
    case '/':
      goto('/search');
      e.preventDefault();
      return;
    case '?':
      if (showHelpCallback) {
        showHelpCallback();
        e.preventDefault();
      }
      return;
  }
}
