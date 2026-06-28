import { api } from './client';

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
  invite: (id: string, d: { email: string; role: string }) => api.post<TeamInvitation>(`/api/teams/${id}/invite`, d),
};
