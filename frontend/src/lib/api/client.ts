type Interceptor = (req: RequestInit & { url: string }) => RequestInit & { url: string };
type ResponseInterceptor = (res: Response) => Response | Promise<Response>;

class ApiClient {
  private base: string;
  private reqInterceptors: Interceptor[] = [];
  private resInterceptors: ResponseInterceptor[] = [];

  constructor(base = "") {
    this.base = base;
  }

  addRequestInterceptor(fn: Interceptor) { this.reqInterceptors.push(fn); }
  addResponseInterceptor(fn: ResponseInterceptor) { this.resInterceptors.push(fn); }

  async request<T>(method: string, path: string, body?: unknown, timeoutMs = 10000): Promise<{ data?: T; error?: string; status: number }> {
    let headers: Record<string, string> = {};
    let reqBody: BodyInit | undefined;
    if (body && !(body instanceof FormData)) {
      headers["Content-Type"] = "application/json";
      reqBody = JSON.stringify(body);
    } else if (body instanceof FormData) {
      reqBody = body;
    }

    let reqInit: RequestInit & { url: string } = {
      url: `${this.base}${path}`,
      method,
      headers,
      body: reqBody,
    };

    for (const interceptor of this.reqInterceptors) {
      reqInit = interceptor(reqInit);
    }

    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), timeoutMs);
      const res = await fetch(reqInit.url, { ...reqInit, signal: controller.signal });
      clearTimeout(timeout);

      let processedRes = res;
      for (const interceptor of this.resInterceptors) {
        processedRes = await interceptor(processedRes);
      }

      const text = await processedRes.text();
      const data = text ? JSON.parse(text) : {};
      if (!processedRes.ok) return { error: data.error || `HTTP ${processedRes.status}`, status: processedRes.status };
      return { data, status: processedRes.status };
    } catch (e: any) {
      if (e.name === "AbortError") return { error: "Request timed out", status: 0 };
      return { error: e.message, status: 0 };
    }
  }

  get<T>(path: string) { return this.request<T>("GET", path); }
  post<T>(path: string, body?: unknown) { return this.request<T>("POST", path, body); }
  put<T>(path: string, body?: unknown) { return this.request<T>("PUT", path, body); }
  del<T>(path: string) { return this.request<T>("DELETE", path); }
}

export const api = new ApiClient("");

// Auth interceptor: attach Bearer token from localStorage
api.addRequestInterceptor((req) => {
  const token = typeof window !== "undefined" ? localStorage.getItem("token") : null;
  if (token) {
    (req.headers as Record<string, string>)["Authorization"] = `Bearer ${token}`;
  }
  return req;
});

// Response interceptor: detect 401 and trigger token refresh
api.addResponseInterceptor(async (res) => {
  if (res.status === 401) {
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent("auth:unauthorized"));
    }
  }
  return res;
});
