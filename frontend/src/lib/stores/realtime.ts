// Vite proxy: relative URL avoids CORS
const API_BASE = "";

class RealtimeClient {
        private es: EventSource | null = null;
        private connected = false;
        private listeners = new Map<string, Set<(data: unknown) => void>>();
        private errorCount = 0;
        private readonly MAX_ERRORS = 5;

        get isConnected() {
                return this.connected;
        }

        connect() {
                if (this.es) return;
                try {
                        this.es = new EventSource(`${API_BASE}/api/events`);
                        this.es.onopen = () => {
                                this.connected = true;
                                this.errorCount = 0;
                        };
                        this.es.onerror = () => {
                                this.connected = false;
                                this.es?.close();
                                this.es = null;
                                this.errorCount++;
                                // After repeated failures, stop retrying and
                                // bounce to /login — session likely expired.
                                // (v22 Phase 1: SSE is now auth-gated, so a 401
                                // here means the session cookie is gone.)
                                if (this.errorCount >= this.MAX_ERRORS) {
                                        if (typeof window !== "undefined") {
                                                window.location.href = "/login";
                                        }
                                        return;
                                }
                                setTimeout(() => this.connect(), 5000);
                        };
                        [
                                "post_created",
                                "post_scheduled",
                                "post_published",
                                "post_failed",
                                "post_deleted",
                                // v22 Phase 1 (BUG #7): kanban stage changes now
                                // broadcast so multi-tab sync works.
                                "post_stage_changed",
                                // v22 Phase 1 (BUG #18): synthetic event emitted
                                // by the server when the SSE client lagged behind
                                // >1024 events. Listeners should refetch stale views.
                                "lagged",
                                "integration_connected",
                                "integration_disconnected",
                                "notification_new",
                                "comment_received",
                                "dm_received",
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
                this.errorCount = 0;
        }

        on(event: string, cb: (data: unknown) => void) {
                if (!this.listeners.has(event)) this.listeners.set(event, new Set());
                this.listeners.get(event)!.add(cb);
                return () => this.listeners.get(event)?.delete(cb);
        }

        private emit(event: string, data: unknown) {
                this.listeners.get(event)?.forEach((cb) => cb(data));
        }
}

export const realtime = new RealtimeClient();
