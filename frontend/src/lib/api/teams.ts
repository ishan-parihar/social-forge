import { api } from './client';

export interface Team {
  id: string;
  name: string;
  slug: string;
  owner_id: string;
  created_at: string;
  updated_at: string;
  member_count: number;
}

export interface TeamMember {
  id: string;
  team_id: string;
  user_id: string;
  email: string;
  name: string;
  role: string;
  joined_at: string;
}

export interface TeamInvitation {
  id: string;
  team_id: string;
  email: string;
  role: string;
  token: string;
  expires_at: string;
  created_at: string;
}

export const teamsApi = {
  list: () => api.get<Team[]>('/api/teams'),
  create: (d: { name: string; slug: string }) => api.post<Team>('/api/teams', d),
  get: (id: string) => api.get<Team>(`/api/teams/${id}`),
  update: (id: string, d: { name?: string; slug?: string }) => api.put<Team>(`/api/teams/${id}`, d),
  delete: (id: string) => api.del(`/api/teams/${id}`),
  invite: (id: string, d: { email: string; role: string }) => api.post<TeamInvitation>(`/api/teams/${id}/invite`, d),
  acceptInvite: (token: string) => api.post<TeamMember>('/api/teams/accept', { token }),
  members: (id: string) => api.get<TeamMember[]>(`/api/teams/${id}/members`),
  removeMember: (id: string, userId: string) => api.del(`/api/teams/${id}/members/${userId}`),
};
