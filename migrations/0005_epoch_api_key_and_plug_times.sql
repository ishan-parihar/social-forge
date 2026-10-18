-- `api_keys.expires_at` and `post_plugs.next_run_at` are declared INTEGER epoch,
-- but their writers bound `DateTime<Utc>`, which sqlx stores as RFC 3339 TEXT.
-- SQLite orders every INTEGER before TEXT, so `expires_at > unixepoch()` held for
-- every expired key and `next_run_at <= unixepoch()` never held for a plug.
--
-- The write path is fixed in Rust (`EpochUtc` bind in api::developer and
-- services::plugs); this rewrites the rows already on disk. Same technique and
-- same caveat as 0002: `strftime('%s', …)` parses the ISO-8601 text, and rows that
-- do not parse are left as they are rather than zeroed.

UPDATE api_keys
   SET expires_at = CAST(strftime('%s', expires_at) AS INTEGER)
 WHERE typeof(expires_at) = 'text'
   AND strftime('%s', expires_at) IS NOT NULL;

UPDATE post_plugs
   SET next_run_at = CAST(strftime('%s', next_run_at) AS INTEGER)
 WHERE typeof(next_run_at) = 'text'
   AND strftime('%s', next_run_at) IS NOT NULL;
