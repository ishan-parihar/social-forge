// Vite proxy: relative URL avoids CORS
const API_BASE = "";

class RealtimeClient {
	private es: EventSource | null = null;
	private connected = false;
	private listeners = new Map<string, Set<(data: any) => void>>();

	get isConnected() {
		return this.connected;
	}

	connect() {
		if (this.es) return;
		try {
			this.es = new EventSource(`${API_BASE}/api/events`);
			this.es.onopen = () => {
				this.connected = true;
			};
			this.es.onerror = () => {
				this.connected = false;
				this.es?.close();
				this.es = null;
				setTimeout(() => this.connect(), 5000);
			};
			[
				"post_created",
				"post_scheduled",
				"post_published",
				"post_failed",
				"post_deleted",
				"integration_connected",
				"integration_disconnected",
			].forEach((type) => {
				this.es!.addEventListener(type, (e: MessageEvent) => {
					try {
						this.emit(type, JSON.parse(e.data));
					} catch {
						this.emit(type, e.data);
					}
				});
			});
		} catch {
			/* ignore */
		}
	}

	disconnect() {
		this.es?.close();
		this.es = null;
		this.connected = false;
	}

	on(event: string, cb: (data: any) => void) {
		if (!this.listeners.has(event)) this.listeners.set(event, new Set());
		this.listeners.get(event)!.add(cb);
		return () => this.listeners.get(event)?.delete(cb);
	}

	private emit(event: string, data: any) {
		this.listeners.get(event)?.forEach((cb) => cb(data));
	}
}

export const realtime = new RealtimeClient();
