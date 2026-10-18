// Vite proxy: relative URL avoids CORS
const API_BASE = "";

class RealtimeClient {
        private es: EventSource | null = null;
        private connected = false;
        private listeners = new Map<string, Set<(data: unknown) => void>>();
        private errorCount = 0;
        private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

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
                                this.scheduleReconnect();
                        };
                        [
                                "post_created",
                                "post_scheduled",
                                "post_published",
                                "post_failed",
                                // Post published, but its first comment failed.
                                "first_comment_failed",
                                "post_deleted",
                                // Post edited after creation (content, media,
                                // schedule edits via the services layer).
                                "post_updated",
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
                                // v22 Phase 6: campaign CRUD - the campaigns pages
                                // and the kanban campaign filter were already
                                // subscribing to these; they only went dead because
                                // the names were never registered here.
                                "campaign_created",
                                "campaign_updated",
                                "campaign_deleted",
                                // Background feed refresher (src/feed/mod.rs):
                                // new external posts pulled in, and engagement /
                                // comment counters refreshed on its own cadence.
                                "feed:new_posts",
                                "feed:engagement",
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
                if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
                this.reconnectTimer = null;
        }

        // A dropped stream is not a logout: a backend restart can outlast any
        // fixed retry budget. Only an explicit 401 from /api/auth/me sends the
        // user to /login. Every other failure retries with capped backoff.
        private scheduleReconnect() {
                if (this.reconnectTimer) return;
                const delay = Math.min(30_000, 2_000 * 2 ** Math.min(this.errorCount - 1, 4));
                this.reconnectTimer = setTimeout(async () => {
                        this.reconnectTimer = null;
                        if (await this.sessionExpired()) {
                                if (typeof window !== "undefined") window.location.href = "/login";
                                return;
                        }
                        this.connect();
                }, delay);
        }

        private async sessionExpired(): Promise<boolean> {
                try {
                        const res = await fetch(`${API_BASE}/api/auth/me`, { credentials: "same-origin" });
                        return res.status === 401;
                } catch {
                        return false;
                }
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
