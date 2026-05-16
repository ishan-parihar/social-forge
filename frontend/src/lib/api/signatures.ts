import { api } from './client';

export interface Signature {
  id: string;
  name: string;
  content: string;
  provider?: string;
  created_at: string;
  updated_at: string;
}

export const signaturesApi = {
  list: () => api.get<Signature[]>('/api/signatures'),
  create: (d: { name: string; content: string; provider?: string }) => api.post<Signature>('/api/signatures', d),
  update: (id: string, d: { name?: string; content?: string; provider?: string }) => api.put<Signature>(`/api/signatures/${id}`, d),
  delete: (id: string) => api.del(`/api/signatures/${id}`),
};
