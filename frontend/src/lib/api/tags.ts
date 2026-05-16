import { api } from './client';

export interface Tag {
  id: string;
  name: string;
  color: string;
  created_at: string;
  updated_at: string;
}

export const tagsApi = {
  list: () => api.get<Tag[]>('/api/tags'),
  create: (d: { name: string; color?: string }) => api.post<Tag>('/api/tags', d),
  update: (id: string, d: { name?: string; color?: string }) => api.put<Tag>(`/api/tags/${id}`, d),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/tags/${id}`),
};
