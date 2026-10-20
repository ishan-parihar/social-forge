// Campaigns API - CRUD for campaign entities + post stage management.
// Phase 7, v20. v22 Phase 6: expanded with status, progress_metric,
// audience_persona, content_pillars, budget_cents, kpi_targets.

import { api } from './client';

export interface Campaign {
  id: string;
  user_id: string;
  name: string;
  description: string | null;
  color: string;
  start_date: string | null;
  end_date: string | null;
  goal: string | null;
  // v22 Phase 6: expanded fields.
  status: 'active' | 'paused' | 'archived' | 'completed';
  progress_metric: 'posts' | 'engagement' | 'reach' | 'followers' | 'custom' | null;
  progress_target: number | null;
  audience_persona: Record<string, unknown> | null;
  content_pillars: Array<{ title: string; description?: string; tags?: string[] }> | null;
  budget_cents: number | null;
  kpi_targets: Record<string, unknown> | null;
  sort_order: number;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
  post_count: number | null;
  /**
   * The posts in this campaign that FAILED to publish.
   *
   * Reported beside `post_count` rather than inside it: a failed post
   * is work the founder did, and the progress bar said nothing about
   * it - so the card read "4 / 24 posts" while the detail page's
   * "Posts (5)" tab listed five, on the same screen.
   */
  failed_count: number | null;
  /**
   * Total engagement across the campaign's published posts, from
   * `analytics_cache`. The progress bar measures `progress_metric`,
   * which can be "engagement" - so it needs this number, not the post
   * count. Absent when the campaign has no measured posts.
   */
  engagement_total?: number | null;
}

export interface CreateCampaignInput {
  name: string;
  description?: string;
  color?: string;
  start_date?: string;
  end_date?: string;
  goal?: string;
  // v22 Phase 6: optional expanded fields.
  status?: 'active' | 'paused' | 'archived' | 'completed';
  progress_metric?: 'posts' | 'engagement' | 'reach' | 'followers' | 'custom';
  progress_target?: number;
  audience_persona?: Record<string, unknown>;
  content_pillars?: Array<{ title: string; description?: string; tags?: string[] }>;
  budget_cents?: number;
  kpi_targets?: Record<string, unknown>;
}

export interface UpdateCampaignInput {
  name?: string;
  /**
   * The clearable fields are THREE-state:
   *   omitted -> leave the stored value alone
   *   `null`  -> clear it
   *   a value -> replace it
   *
   * The settings page used to send `|| undefined` for an emptied
   * field, which is "omit", so the server kept the old value and the
   * save reported success while nothing changed.
   */
  description?: string | null;
  color?: string;
  start_date?: string | null;
  end_date?: string | null;
  goal?: string | null;
  // v22 Phase 6: expanded fields.
  status?: 'active' | 'paused' | 'archived' | 'completed';
  progress_metric?: 'posts' | 'engagement' | 'reach' | 'followers' | 'custom' | null;
  progress_target?: number | null;
  audience_persona?: Record<string, unknown> | null;
  content_pillars?: Array<{ title: string; description?: string; tags?: string[] }> | null;
  budget_cents?: number | null;
  kpi_targets?: Record<string, unknown> | null;
}

/**
 * Format a `YYYY-MM-DD` date for display.
 *
 * A bare `YYYY-MM-DD` is a CALENDAR DATE, not an instant. `new
 * Date("2026-11-15")` parses it as UTC midnight, and rendering that in
 * a negative-offset zone gives the PREVIOUS day: a campaign ending
 * 15 November read as "11/14/2026". The string is split and formatted
 * directly, so the date the founder typed is the date they see.
 */
export function formatDateOnly(date: string | null | undefined, locale = 'en-US'): string {
  if (!date) return '';
  const m = /^(\d{4})-(\d{2})-(\d{2})/.exec(date);
  if (!m) return date;
  const [, y, mo, d] = m;
  // Construct in UTC so the formatter does not shift it, then read the
  // UTC parts back.
  const dt = new Date(Date.UTC(Number(y), Number(mo) - 1, Number(d)));
  return dt.toLocaleDateString(locale, {
    year: 'numeric',
    month: 'numeric',
    day: 'numeric',
    timeZone: 'UTC',
  });
}

export const campaignsApi = {
  list: () => api.get<Campaign[]>('/api/campaigns'),
  create: (data: CreateCampaignInput) => api.post<Campaign>('/api/campaigns', data),
  update: (id: string, data: UpdateCampaignInput) => api.put<Campaign>(`/api/campaigns/${id}`, data),
  delete: (id: string) => api.del<{ deleted: boolean }>(`/api/campaigns/${id}`),
  updateStage: (
    postId: string,
    state: string,
    /**
     * The campaign to put the post in. Three states:
     *   undefined -> leave the post's campaign alone
     *   null      -> unassign the post from its campaign
     *   a uuid     -> assign the post to that campaign
     *
     * The backend reads all three. Omitting the key used to NULL the
     * column, which silently dropped a post out of its campaign on every
     * kanban drag with no campaign filter active.
     */
    campaignId?: string | null,
    // v25-3: optional kanban metadata. When omitted, the existing value is
    // preserved. Pass an empty string to `due_date` to explicitly clear
    // it.
    //
    // `kanban_substate` is THREE-state, and the three states have to
    // survive the wire:
    //   undefined -> leave it alone (the key is not sent)
    //   null      -> CLEAR it
    //   a value   -> set it
    //
    // A plain `string | undefined` could not express the middle one,
    // and the backend's `COALESCE(?, kanban_substate)` could not
    // either - so a card tagged "Blocked" was "Blocked" for ever.
    kanbanMeta?: {
      kanban_substate?: string | null;
      priority?: string;
      due_date?: string;
    },
  ) =>
    api.patch<{ updated: boolean }>(`/api/posts/${postId}/stage`, {
      state,
      ...(campaignId === undefined ? {} : { campaign_id: campaignId }),
      ...(kanbanMeta && 'kanban_substate' in kanbanMeta
        ? { kanban_substate: kanbanMeta.kanban_substate }
        : {}),
      priority: kanbanMeta?.priority,
      due_date: kanbanMeta?.due_date,
    }),
};
