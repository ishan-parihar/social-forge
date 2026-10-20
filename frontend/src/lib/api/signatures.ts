import { api } from './client';

export interface Signature {
  id: string;
  name: string;
  content: string;
  provider?: string;
  /** Phase v21/v22: TRUE if this signature is auto-appended to new posts. */
  is_default?: boolean;
  created_at: string;
  updated_at: string;
}

export const signaturesApi = {
  list: () => api.get<Signature[]>('/api/signatures'),
  create: (d: { name: string; content: string; provider?: string }) => api.post<Signature>('/api/signatures', d),
  update: (id: string, d: { name?: string; content?: string; provider?: string }) => api.put<Signature>(`/api/signatures/${id}`, d),
  delete: (id: string) => api.del(`/api/signatures/${id}`),
  /** Phase v21/v22: set a signature as the default for its provider. */
  setDefault: (id: string) => api.post<Signature>(`/api/signatures/${id}/set-default`, {}),
  /**
   * Get the default signature for a provider (or global if no provider).
   *
   * `null` when no default is set. It used to be a 404, which put an
   * error in every user's console on every page - the composer calls
   * this on mount.
   */
  getDefault: (provider?: string) => {
    const q = provider ? `?provider=${encodeURIComponent(provider)}` : '';
    return api.get<Signature | null>(`/api/signatures/default${q}`);
  },
};
