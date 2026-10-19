-- `external_posts.created_at` is declared INTEGER epoch, but the writer
-- bound a `DateTime<Utc>`, which sqlx stores as RFC 3339 TEXT. SQLite
-- keeps TEXT in an INTEGER-affinity column, so the column held
-- "2026-09-16T13:34:30+00:00" while every reader compared it against
-- an INTEGER epoch cutoff.
--
-- SQLite orders every INTEGER before every TEXT, so `created_at >= ?`
-- (with an INTEGER bound) was ALWAYS TRUE: the 90-day window on the
-- research page, the feed engagement summary and the posting-time
-- histogram were all no-ops, and posts of any age passed the filter.
--
-- Same technique and same caveat as 0002 and 0005: `strftime('%s', ...)`
-- parses the ISO-8601 text, and rows that do not parse are left as they
-- are rather than zeroed. The Rust write path is fixed in the same
-- change (the `EpochUtc` seam), so new rows are INTEGER from now on.

UPDATE external_posts
   SET created_at = CAST(strftime('%s', created_at) AS INTEGER)
 WHERE typeof(created_at) = 'text'
   AND strftime('%s', created_at) IS NOT NULL;

-- `imported_at` has the same writer and the same readers.
UPDATE external_posts
   SET imported_at = CAST(strftime('%s', imported_at) AS INTEGER)
 WHERE typeof(imported_at) = 'text'
   AND strftime('%s', imported_at) IS NOT NULL;
