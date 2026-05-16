import { api } from './client';

export interface MediaItem {
  id: string; original_name: string; url: string;
  mime_type: string; file_size: number; width?: number; height?: number;
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
