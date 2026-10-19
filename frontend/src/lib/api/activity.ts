import { api } from "./client";

/** Mirrors src/api/activity.rs ActivityRow. */
export interface ActivityRow {
  /** `<kind>:<source-id>` — stable across pages, safe as a list key. */
  id: string;
  kind: "event" | "automation" | "publish";
  /** Epoch seconds (UTC). */
  at: number;
  summary: string;
  post_id?: string;
  rule_id?: string;
  integration_id?: string;
  provider?: string;
  ok: boolean;
}

/** Mirrors src/api/activity.rs ActivityResponse. */
export interface ActivityFeed {
  rows: ActivityRow[];
  /** Epoch seconds to pass as `before` for the next page; null when done. */
  next_before: number | null;
}

export type ActivityKind = "all" | "event" | "automation" | "publish";
export type ActivityRange = "24h" | "7d" | "30d";

export interface ActivityParams {
  kind?: ActivityKind;
  range?: ActivityRange;
  provider?: string;
  /** Cursor: only rows strictly older than this epoch second. */
  before?: number | null;
}

export const activityApi = {
  /** One page of the merged activity feed, newest first. */
  list: (params: ActivityParams = {}, signal?: AbortSignal) => {
    const q = new URLSearchParams();
    if (params.kind && params.kind !== "all") q.set("kind", params.kind);
    if (params.range) q.set("range", params.range);
    if (params.provider && params.provider !== "all") q.set("provider", params.provider);
    // Omitted entirely when null: a `before=` of `null` would serialise to
    // the string "null" and fail to decode server-side.
    if (params.before != null) q.set("before", String(params.before));
    const qs = q.toString();
    return api.get<ActivityFeed>(`/api/activity${qs ? `?${qs}` : ""}`, signal);
  },
};
