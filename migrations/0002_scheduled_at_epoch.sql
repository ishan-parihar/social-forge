-- G-02: `posts.scheduled_at` is declared INTEGER epoch, but sqlx encodes
-- `DateTime<Utc>` as RFC3339 TEXT. SQLite orders INTEGER before TEXT, so
-- `scheduled_at <= unixepoch()` was always FALSE for those rows and the
-- scheduler never claimed them.
--
-- The write path is fixed in Rust (`EpochUtc` bind, see db::queries); this
-- rewrites the rows already on disk.
--
-- `CAST(x AS INTEGER)` alone is NOT usable here: for
-- '2030-01-01T09:00:00+00:00' it yields 2030 (the leading numeric prefix),
-- which would mark a 2030 post as long-past-due and fire it immediately.
-- `strftime('%s', …)` parses the ISO-8601 text properly. Rows whose value
-- does not parse are left untouched rather than zeroed, so they still read
-- back as their original DateTime instead of silently becoming 1970.

UPDATE posts
   SET scheduled_at = CAST(strftime('%s', scheduled_at) AS INTEGER)
 WHERE typeof(scheduled_at) = 'text'
   AND strftime('%s', scheduled_at) IS NOT NULL;