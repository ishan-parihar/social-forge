import { api } from './client';

export interface MediaItem {
  id: string; original_name: string; url: string;
  mime_type: string; file_size: number; width?: number; height?: number;
  // v22 Phase 7: alt text for accessibility. Previously MediaUpload
  // saved alt text to original_name (conflating filename + alt), and
  // ComposerModal.buildPayload read m.alt (which never existed) — so
  // alt text was silently dropped on submit. Now it's a proper field.
  alt?: string;
  created_at?: string;
}

export interface ListMediaParams {
  limit?: number;
  offset?: number;
  search?: string;
}

export const mediaApi = {
  upload: (file: File) => {
    const fd = new FormData();
    fd.append("file", file);
    return api.post<MediaItem>("/api/media", fd);
  },
  /**
   * Phase v21: upload with real progress via XMLHttpRequest.
   *
   * The default `upload()` goes through the shared ApiClient which uses
   * `fetch()` — and `fetch()` has no way to observe upload progress
   * (the body is consumed as a stream, but there's no `onprogress` for
   * the upload phase). XHR's `upload.onprogress` gives us real byte-level
   * progress, which is what users expect for video uploads especially.
   *
   * Returns a Promise that resolves to the same envelope as `upload()`:
   * `{ data?: MediaItem; error?: string; status: number }`. The optional
   * `onProgress` callback fires with a 0-100 integer on each progress event.
   *
   * Mirrors the ApiClient's auth (credentials: 'include') + 401 redirect
   * + JSON envelope behavior so callers can use it as a drop-in.
   */
  uploadWithProgress: (
    file: File,
    onProgress?: (percent: number) => void,
  ): Promise<{ data?: MediaItem; error?: string; status: number }> => {
    const fd = new FormData();
    fd.append("file", file);

    return new Promise((resolve) => {
      const xhr = new XMLHttpRequest();
      xhr.open("POST", "/api/media");
      xhr.withCredentials = true;

      // Real upload progress (bytes uploaded).
      if (onProgress && xhr.upload) {
        xhr.upload.onprogress = (e: ProgressEvent) => {
          if (e.lengthComputable) {
            onProgress(Math.round((e.loaded / e.total) * 100));
          }
        };
        // When the upload finishes but the response hasn't come back yet,
        // hold the bar at 100% so the user sees "processing…" rather than
        // a frozen bar.
        xhr.upload.onload = () => onProgress?.(100);
      }

      xhr.onerror = () => {
        resolve({ error: "Network error during upload", status: 0 });
      };

      xhr.onload = () => {
        let data: unknown = null;
        try { data = JSON.parse(xhr.responseText); } catch { /* non-JSON */ }
        // 401 → redirect to /login (mirror ApiClient behavior)
        if (xhr.status === 401 && !window.location.pathname.startsWith('/login')) {
          window.location.href = '/login';
          return;
        }
        if (xhr.status >= 200 && xhr.status < 300) {
          resolve({ data: data as MediaItem, status: xhr.status });
        } else {
          const errMsg = (data && typeof data === 'object' && 'error' in data && typeof (data as Record<string, unknown>).error === 'string')
            ? (data as Record<string, string>).error
            : `HTTP ${xhr.status}`;
          resolve({ error: errMsg, status: xhr.status });
        }
      };

      xhr.send(fd);
    });
  },
  list: (params?: ListMediaParams) => {
    const searchParams = new URLSearchParams();
    if (params?.limit) searchParams.set("limit", String(params.limit));
    if (params?.offset) searchParams.set("offset", String(params.offset));
    if (params?.search) searchParams.set("search", params.search);
    const qs = searchParams.toString();
    return api.get<MediaItem[]>(`/api/media${qs ? `?${qs}` : ""}`);
  },
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/media/${id}`),
};
