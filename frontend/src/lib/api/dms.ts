import { api } from './client';

export interface Conversation {
  id: string;
  platform: string;
  participant: string;
  participant_name: string | null;
  participant_avatar: string | null;
  last_message: string | null;
  last_message_at: string | null;
  unread_count: number;
}

export interface DmMessage {
  id: string;
  conversation_id: string;
  sender: string;
  sender_name: string | null;
  content: string;
  created_at: string;
  read: boolean;
}

export const dmsApi = {
  listConversations: (integrationId: string, limit?: number) =>
    api.get<{ conversations: Conversation[]; total: number }>(
      `/api/dms/conversations?integration_id=${integrationId}${limit ? `&limit=${limit}` : ''}`
    ),
  getMessages: (conversationId: string, integrationId: string, limit?: number) => {
    const params = new URLSearchParams({ integration_id: integrationId });
    if (limit) params.set('limit', String(limit));
    return api.get<{ messages: DmMessage[]; total: number }>(
      `/api/dms/${conversationId}/messages?${params.toString()}`
    );
  },
  send: (integrationId: string, recipient: string, content: string) =>
    api.post<{ message_id: string; status: string }>(`/api/dms/send`, {
      integration_id: integrationId,
      recipient,
      content,
      media: [],
    }),
};
