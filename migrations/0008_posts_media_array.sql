-- `posts.media` is an ARRAY: the column's default is '[]' and every
-- reader treats it as one.
--
-- The RSS autopost path wrote a JSON OBJECT there instead:
--
--     create_post(db, ..., &serde_json::json!({}), &serde_json::json!({"rss_auto": true}), ...)
--
-- `create_post`'s sixth parameter is `media`, so every RSS draft was
-- stored with `media = {}`.
--
-- The frontend assumes an array. The kanban's `coverImage` calls
-- `post.media.find(...)` directly, and `{}` has no `find`, so the whole
-- ROUTE crashed with "i.find is not a function" - a pageerror, not just
-- one bad card. The calendar's `(post.media || []).map(...)` failed the
-- same way, because `{}` is truthy so the `||` fallback never fired.
--
-- The writer is fixed in the same change (both RSS paths now pass
-- `json!([])`), so new rows are arrays from now on. This migration
-- repairs the rows that are already there.
--
-- `json_type(media) = 'object'` catches the broken shape exactly: a
-- row whose media is a single ATTACHMENT OBJECT would also match, but
-- that shape has never been written - the writers pass either an array
-- or `{}`, and an attachment object always arrives inside an array.
-- To be safe, an object that actually looks like an attachment (it
-- carries a `url`) is kept as a single-element array rather than
-- flattened away.

UPDATE posts
   SET media = '[]'
 WHERE json_type(media) = 'object'
   AND json_type(media, '$.url') IS NULL;

-- An object that DOES carry a url is one attachment: wrap it, do not
-- drop it.
UPDATE posts
   SET media = json_array(media)
 WHERE json_type(media) = 'object'
   AND json_type(media, '$.url') IS NOT NULL;
