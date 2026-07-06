// Touch-drag fallback for calendar DnD (Phase 10, v19).
//
// HTML5 native drag-and-drop doesn't fire on most mobile browsers.
// This module provides a pointer-events-based fallback so calendar
// events can be drag-rescheduled on touch devices.
//
// Usage (in WeekView or DayView):
//   import { makeTouchDragHandler } from './touch-drag';
//
//   const touchDrag = makeTouchDragHandler({
//     getEventId: (e: TouchEvent) => {
//       // Extract the event ID from the touched element
//       const target = e.target as HTMLElement;
//       return target.closest('[data-event-id]')?.getAttribute('data-event-id') || null;
//     },
//     getDropTarget: (x: number, y: number) => {
//       // Use elementFromPoint to find the cell under the finger
//       const el = document.elementFromPoint(x, y);
//       const cell = el?.closest('[data-drop-date]') as HTMLElement | null;
//       if (!cell) return null;
//       return {
//         date: cell.dataset.dropDate!,
//         hour: cell.dataset.dropHour || undefined,
//       };
//     },
//     onDrop: (eventId: string, date: string, hour?: string) => {
//       onDrop?.(eventId, date, hour);
//     },
//   });
//
// The handler uses a 200ms long-press threshold before starting the drag
// to avoid interfering with scroll. If the finger moves >10px before the
// threshold, the drag is cancelled (treated as a scroll attempt).

export interface TouchDropTarget {
  date: string;
  hour?: string;
}

export interface TouchDragConfig {
  getEventId: (e: TouchEvent) => string | null;
  getDropTarget: (x: number, y: number) => TouchDropTarget | null;
  onDrop: (eventId: string, date: string, hour?: string) => void;
  onHighlight?: (target: TouchDropTarget | null) => void;
}

const LONG_PRESS_THRESHOLD_MS = 200;
const MOVE_CANCEL_THRESHOLD_PX = 10;

export function makeTouchDragHandler(config: TouchDragConfig) {
  let dragEventId: string | null = null;
  let startX = 0;
  let startY = 0;
  let longPressTimer: ReturnType<typeof setTimeout> | null = null;
  let isDragging = false;
  let ghostEl: HTMLElement | null = null;

  function onTouchStart(e: TouchEvent) {
    if (e.touches.length !== 1) return;
    const touch = e.touches[0];
    startX = touch.clientX;
    startY = touch.clientY;

    const eventId = config.getEventId(e);
    if (!eventId) return;

    // Start long-press timer. If the finger doesn't move >10px before
    // the timer fires, we start the drag.
    longPressTimer = setTimeout(() => {
      dragEventId = eventId;
      isDragging = true;
      // Create a floating ghost element.
      createGhost(touch.clientX, touch.clientY, e.target as HTMLElement);
      // Prevent scroll while dragging.
      e.preventDefault?.();
    }, LONG_PRESS_THRESHOLD_MS);
  }

  function onTouchMove(e: TouchEvent) {
    if (e.touches.length !== 1) return;
    const touch = e.touches[0];

    if (!isDragging && longPressTimer) {
      // Check if moved > threshold — cancel long-press (treat as scroll).
      const dx = touch.clientX - startX;
      const dy = touch.clientY - startY;
      if (Math.abs(dx) > MOVE_CANCEL_THRESHOLD_PX || Math.abs(dy) > MOVE_CANCEL_THRESHOLD_PX) {
        clearTimeout(longPressTimer);
        longPressTimer = null;
      }
      return;
    }

    if (!isDragging || !dragEventId) return;

    // Prevent scroll while dragging.
    e.preventDefault();

    // Move the ghost.
    if (ghostEl) {
      ghostEl.style.left = `${touch.clientX}px`;
      ghostEl.style.top = `${touch.clientY}px`;
    }

    // Highlight the drop target under the finger.
    const target = config.getDropTarget(touch.clientX, touch.clientY);
    config.onHighlight?.(target);
  }

  function onTouchEnd(e: TouchEvent) {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      longPressTimer = null;
    }

    if (!isDragging || !dragEventId) {
      isDragging = false;
      dragEventId = null;
      return;
    }

    // Get the last touch position.
    const touch = e.changedTouches[0];
    if (touch) {
      const target = config.getDropTarget(touch.clientX, touch.clientY);
      if (target) {
        config.onDrop(dragEventId, target.date, target.hour);
      }
    }

    // Cleanup.
    config.onHighlight?.(null);
    removeGhost();
    isDragging = false;
    dragEventId = null;
  }

  function createGhost(x: number, y: number, sourceEl: HTMLElement) {
    ghostEl = document.createElement('div');
    ghostEl.style.position = 'fixed';
    ghostEl.style.left = `${x}px`;
    ghostEl.style.top = `${y}px`;
    ghostEl.style.transform = 'translate(-50%, -50%)';
    ghostEl.style.zIndex = '9999';
    ghostEl.style.pointerEvents = 'none';
    ghostEl.style.opacity = '0.8';
    ghostEl.style.background = 'var(--bg-card, #131720)';
    ghostEl.style.border = '1px solid var(--border, #1e2435)';
    ghostEl.style.borderRadius = '4px';
    ghostEl.style.padding = '4px 8px';
    ghostEl.style.fontSize = '11px';
    ghostEl.style.color = 'var(--text, #e8edf5)';
    ghostEl.style.maxWidth = '200px';
    ghostEl.style.whiteSpace = 'nowrap';
    ghostEl.style.overflow = 'hidden';
    ghostEl.style.textOverflow = 'ellipsis';
    ghostEl.textContent = sourceEl.textContent?.trim().slice(0, 40) || 'Drag...';
    document.body.appendChild(ghostEl);
  }

  function removeGhost() {
    if (ghostEl) {
      ghostEl.remove();
      ghostEl = null;
    }
  }

  return { onTouchStart, onTouchMove, onTouchEnd };
}
