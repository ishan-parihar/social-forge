export type ToastType = "success" | "error" | "warning" | "info";
export interface Toast {
	id: number;
	message: string;
	type: ToastType;
}

let _id = 0;
let _listeners: Array<(t: Toast) => void> = [];

export function toast(message: string, type: ToastType = "info") {
	const t: Toast = { id: ++_id, message, type };
	_listeners.forEach((fn) => fn(t));
	setTimeout(() => _listeners.forEach((fn) => fn({ ...t, message: "" })), 4000);
}

export function onToast(fn: (t: Toast) => void) {
	_listeners.push(fn);
	return () => {
		_listeners = _listeners.filter((f) => f !== fn);
	};
}
