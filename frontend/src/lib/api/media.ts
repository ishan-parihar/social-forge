import { api } from './client';

export interface MediaItem {
  id: string; original_name: string; url: string;
  mime_type: string; file_size: number; width?: number; height?: number;
}

export const mediaApi = {
  upload: (file: File) => {
    const fd = new FormData();
    fd.append("file", file);
    return api.post<MediaItem>("/api/media", fd);
  },
};
