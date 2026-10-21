class ApiClient {
  private base: string;

  constructor(base = "") {
    this.base = base;
  }

  async request<T>(method: string, path: string, body?: unknown, timeoutMs = 30000, signal?: AbortSignal): Promise<{ data?: T; error?: string; status: number }> {
    let headers: Record<string, string> = {};
    let reqBody: BodyInit | undefined;
    if (body && !(body instanceof FormData)) {
      headers["Content-Type"] = "application/json";
      reqBody = JSON.stringify(body);
    } else if (body instanceof FormData) {
      reqBody = body;
    }

    const reqInit: RequestInit = {
      method,
      headers,
      body: reqBody,
      // Always send the sf_session cookie - the backend validates it
      // via auth_middleware. Without this, fetch() omits cookies on
      // cross-origin requests and we'd get 401 on every call.
      credentials: "include",
    };

    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), timeoutMs);
      const combinedSignal = signal ? AbortSignal.any([controller.signal, signal]) : controller.signal;
      const res = await fetch(`${this.base}${path}`, { ...reqInit, signal: combinedSignal });
      clearTimeout(timeout);

      // A SESSION 401 bounces to /login: the session never existed,
      // expired, or the cookie was cleared. But NOT every 401 is the
      // session: a provider token that expired mid-fetch answers 401
      // with code "token_expired" (the analytics and publish paths),
      // and bouncing on THAT destroys the modal the founder just
      // opened and dumps them on the login page while still logged
      // in - the stats button on a calendar list row did exactly
      // that. Parse the body: only the session 401 ("no_session")
      // navigates; every other 401 returns to the caller as the
      // error it is, where the UI can say what actually happened.
      if (res.status === 401 && !path.startsWith("/api/auth/")) {
        const text = await res.text();
        let code: string | undefined;
        try {
          code = (JSON.parse(text) as Record<string, unknown>).code as string | undefined;
        } catch {
          code = undefined;
        }
        if (code === "no_session") {
          if (typeof window !== "undefined") {
            window.location.href = "/login";
          }
          return { error: "Not authenticated", status: 401 };
        }
        // Unwrap the error envelope so the UI shows a message, not
        // raw JSON ("{\"error\":\"Token expired\"...}" in the stats
        // modal). The envelope's `error` field is the human text.
        let msg = text || "Unauthorized";
        try {
          const env = JSON.parse(text) as Record<string, unknown>;
          if (env.error) msg = String(env.error);
        } catch {
          /* non-JSON body: keep the raw text */
        }
        return { error: msg, status: 401 };
      }

      const text = await res.text();
      let data: Record<string, unknown>;
      try {
        data = text ? JSON.parse(text) : {};
      } catch {
        return { error: `Invalid JSON response from ${path}`, status: res.status };
      }
      if (!res.ok) {
        const errorMsg = typeof data.error === 'string' ? data.error : `HTTP ${res.status}`;
        return { error: errorMsg, status: res.status };
      }
      return { data: data as T, status: res.status };
    } catch (e: unknown) {
      if (e instanceof Error && e.name === "AbortError") return { error: "Request timed out", status: 0 };
      return { error: (e instanceof Error ? e.message : String(e)), status: 0 };
    }
  }

  get<T>(path: string, signal?: AbortSignal) { return this.request<T>("GET", path, undefined, undefined, signal); }
  post<T>(path: string, body?: unknown) { return this.request<T>("POST", path, body); }
  put<T>(path: string, body?: unknown) { return this.request<T>("PUT", path, body); }
  patch<T>(path: string, body?: unknown) { return this.request<T>("PATCH", path, body); }
  del<T>(path: string) { return this.request<T>("DELETE", path); }
}

export const api = new ApiClient("");
