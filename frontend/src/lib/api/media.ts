import { api } from './client';

export interface MediaItem {
  id: string; original_name: string; url: string;
  mime_type: string; file_size: number; width?: number; height?: number;
  // v22 Phase 7: alt text for accessibility. Previously MediaUpload
  // saved alt text to original_name (conflating filename + alt), and
  // ComposerModal.buildPayload read m.alt (which never existed) - so
  // alt text was silently dropped on submit. Now it's a proper field.
  alt?: string;
  created_at?: string;
}

export interface ListMediaParams {
  limit?: number;
  offset?: number;
  search?: string;
}

/**
 * The mime type of a media URL, from its extension.
 *
 * The platform rules classify media by mime type - `image/*` is an
 * image, `video/*` is a video - so a wrong mime type miscounts the
 * media. The library picker used to label every non-image
 * `application/octet-stream`, which is neither: an MP4 from the
 * library was counted as neither an image nor a video, the composer's
 * video count was wrong, and X's "images and video cannot be mixed"
 * rule fired on a post that held one video.
 *
 * An unknown extension falls back to `application/octet-stream`, which
 * is the honest answer.
 */
export function mimeForUrl(url: string): string {
  const ext = (url.split('?')[0].split('#')[0].match(/\.([a-z0-9]+)$/i)?.[1] ?? '').toLowerCase();
  const table: Record<string, string> = {
    png: 'image/png',
    jpg: 'image/jpeg',
    jpeg: 'image/jpeg',
    webp: 'image/webp',
    gif: 'image/gif',
    avif: 'image/avif',
    svg: 'image/svg+xml',
    mp4: 'video/mp4',
    m4v: 'video/mp4',
    mov: 'video/quicktime',
    webm: 'video/webm',
    mkv: 'video/x-matroska',
    avi: 'video/x-msvideo',
    mp3: 'audio/mpeg',
    m4a: 'audio/mp4',
    wav: 'audio/wav',
    ogg: 'audio/ogg',
    pdf: 'application/pdf',
  };
  return table[ext] ?? 'application/octet-stream';
}

export const mediaApi = {
  /**
   * Update a media item's alt text.
   *
   * The alt text is a real column and a real field on the wire, but
   * there was no way to write it: the composer's edit box changed a
   * browser-side object only, so the alt text was lost on reload - and
   * DESTROYED by the first save of the post, which wrote the empty
   * browser value back over it.
   */
  updateAlt: (id: string, alt: string | null) =>
    api.patch<MediaItem>(`/api/media/${id}`, { alt }),
  upload: (file: File) => {
    const fd = new FormData();
    fd.append("file", file);
    return api.post<MediaItem>("/api/media", fd);
  },
  /**
   * Phase v21: upload with real progress via XMLHttpRequest.
   *
   * The default `upload()` goes through the shared ApiClient which uses
   * `fetch()` - and `fetch()` has no way to observe upload progress
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
