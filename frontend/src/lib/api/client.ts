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
      // Always send the sf_session cookie — the backend validates it
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

      // 401 → bounce to /login. The session either never existed,
      // expired, or the cookie was cleared. Don't try to parse the
      // body — just redirect.
      if (res.status === 401 && !path.startsWith("/api/auth/")) {
        if (typeof window !== "undefined") {
          window.location.href = "/login";
        }
        return { error: "Not authenticated", status: 401 };
      }

      const text = await res.text();
      let data: any;
      try {
        data = text ? JSON.parse(text) : {};
      } catch {
        return { error: `Invalid JSON response from ${path}`, status: res.status };
      }
      if (!res.ok) return { error: data.error || `HTTP ${res.status}`, status: res.status };
      return { data, status: res.status };
    } catch (e: any) {
      if (e.name === "AbortError") return { error: "Request timed out", status: 0 };
      return { error: e.message, status: 0 };
    }
  }

  get<T>(path: string, signal?: AbortSignal) { return this.request<T>("GET", path, undefined, undefined, signal); }
  post<T>(path: string, body?: unknown) { return this.request<T>("POST", path, body); }
  put<T>(path: string, body?: unknown) { return this.request<T>("PUT", path, body); }
  del<T>(path: string) { return this.request<T>("DELETE", path); }
}

export const api = new ApiClient("");
