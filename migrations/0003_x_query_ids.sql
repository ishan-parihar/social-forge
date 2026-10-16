-- F4: discovered X GraphQL queryIds. Refreshed from x.com JS bundles by
-- x_queryids::refresh_if_stale with a TTL; the static table in
-- social/x_queryids.rs is the final fallback when discovery fails.

CREATE TABLE IF NOT EXISTS x_query_ids (
    op_name    TEXT PRIMARY KEY NOT NULL,
    query_id   TEXT NOT NULL,
    fetched_at INTEGER NOT NULL
);
