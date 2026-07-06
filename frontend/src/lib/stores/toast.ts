export type ToastType = "success" | "error" | "warning" | "info";
export interface Toast {
	id: number;
	message: string;
	type: ToastType;
}

// R-12 fix: toast race condition.
//
// Before this fix, `toast()` broadcast to `_listeners` immediately. If
// `toast()` was called before `Toast.svelte` mounted its listener (which
// happens during initial page load — e.g., a `+page.svelte` `onMount`
// that fires before `+layout.svelte`'s `Toast.svelte` `onMount`), the
// toast was silently lost.
//
// Fix: buffer toasts in `_pending` when there are no listeners. When the
// first listener subscribes, flush the buffer to it. Each pending toast
// also schedules its own 4-second dismissal, so even if no listener ever
// subscribes (e.g., server-side render), the pending buffer doesn't
// grow unboundedly.

let _id = 0;
let _listeners: Array<(t: Toast) => void> = [];
let _pending: Toast[] = [];

function scheduleDismissal(t: Toast) {
	setTimeout(() => {
		_listeners.forEach((fn) => fn({ ...t, message: "" }));
		// Also remove from _pending in case it hasn't been flushed yet
		// (no listener ever subscribed).
		_pending = _pending.filter((x) => x.id !== t.id);
	}, 4000);
}

export function toast(message: string, type: ToastType = "info") {
	const t: Toast = { id: ++_id, message, type };

	if (_listeners.length === 0) {
		// No listener yet — buffer the toast. It'll be flushed when the
		// first listener subscribes. Schedule the dismissal now so the
		// buffered toast doesn't live forever if no listener ever shows up.
		_pending.push(t);
		scheduleDismissal(t);
		return;
	}

	// Listeners exist — broadcast immediately.
	_listeners.forEach((fn) => fn(t));
	scheduleDismissal(t);
}

export function onToast(fn: (t: Toast) => void) {
	_listeners.push(fn);

	// Flush any pending toasts that were fired before this listener
	// subscribed. Use a microtask so the listener's first render
	// (if it's a Svelte component) has a chance to set up its state
	// before we start pushing toasts at it.
	if (_pending.length > 0) {
		const pending = _pending;
		_pending = [];
		queueMicrotask(() => {
			for (const t of pending) {
				fn(t);
			}
		});
	}

	return () => {
		_listeners = _listeners.filter((f) => f !== fn);
	};
}
