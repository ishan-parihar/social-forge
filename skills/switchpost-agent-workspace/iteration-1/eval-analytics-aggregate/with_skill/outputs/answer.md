# X Account Status — @ishanparihar (read-only)

**Account:** Ishan Parihar, @_ishanparihar (X user id 3301263462) — healthy per `doctor` (live-verified, cookie auth).

## Commands run

All commands used the local CLI at `./target/release/switchpost` and were **strictly read-only** (no post was created, scheduled, published, or deleted; no like/reply/DM was sent). `import x` and `feed` only pull copies of already-existing posts into the local store — they write nothing to X and nothing to the posts lifecycle.

```bash
./target/release/switchpost agent --json
./target/release/switchpost providers --json
./target/release/switchpost doctor
./target/release/switchpost analytics get x --days 30
./target/release/switchpost analytics get x --days 90 --json
./target/release/switchpost analytics get x --days 365 --json
./target/release/switchpost mcp-call x_get_analytics --args '{}'
./target/release/switchpost mcp-call x_get_me --args '{}'
./target/release/switchpost posts list --state published --limit 10
./target/release/switchpost posts list --limit 30
./target/release/switchpost import x --count 20
./target/release/switchpost feed --provider x --json
./target/release/switchpost mcp-call x_user_tweets --args '{"user_id": "3301263462", "count": 10}' --json
./target/release/switchpost mcp-call x_search_tweets --args '{"query": "from:_ishanparihar", "count": 10}' --json
./target/release/switchpost mcp-call x_get_post_analytics --args '{"tweet_id": "<latest-id>"}'        # 403, see caveat
./target/release/switchpost mcp-call x_tweet_detail --args '{"tweet_id": "<id>"}'                      # empty results, see caveat
./target/release/switchpost analytics post 94b0dc93-c939-44f7-9dec-6502fe6d37fa                        # Post not found (not an internal posts row)
./target/release/switchpost mcp-call analytics_get_post --args '{"post_id": "94b0dc93-c939-44f7-9dec-6502fe6d37fa"}'
./target/release/switchpost mcp-call analytics_summary --args '{}'
./target/release/switchpost mcp-call feed_analytics --args '{}'
./target/release/switchpost mcp-call feed_analytics --args '{"provider": "x"}'
./target/release/switchpost streak --tz Asia/Kolkata
./target/release/switchpost mcp-call tools_search --args '{"query":"analytics"}'
./target/release/switchpost mcp-call tools_search --args '{"query":"engagement"}'
./target/release/switchpost mcp-call tools_search --args '{"query":"tweet"}'
./target/release/switchpost mcp-call tools_search --args '{"query":"timeline"}'
./target/release/switchpost mcp-tools --toolset x --pretty
./target/release/switchpost mcp-tools --toolset x --json
# plus jq post-processing on captured JSON (feed / x_user_tweets / x_search_tweets)
```

## Findings

### Follower trend
- `analytics get x` (30/90/365-day windows all identical) and `x_get_analytics` return a **single snapshot**: **Followers 9** and **Following 93**, `percentage_change 0`.
- **No historical follower trend exists in the analytics series** — the series carries one data point (2026-10-24), so the follower "trend" is flat/no-change: 9 followers today, 0% reported change.
- Live profile read (`x_get_me`) confirms the snapshot: 9 followers, 93 following, 80 lifetime tweets (statuses), account created July 2015, not verified.

### Latest post engagement
- Latest post live on X (verified via the live `x_user_tweets` timeline; also the newest in `from:_ishanparihar` search), posted **2026-10-19 13:59:35 UTC**: *"The research tab ranks what actually got engagement, not what you hoped would."*
  - **Views: 3** · **Likes: 0** · **Reposts: 0** · **Replies: 0** · **Quotes: 0** · **Bookmarks: 0**
- Batch context: this was one of 8 posts published back-to-back that day (13:59:10–13:59:35 UTC); across all eight the combined engagement is **11 views, 0 likes, 0 reposts, 1 reply, 0 quotes**. The single reply landed on *"The worst part of social media management is not the writing. It is the tab."*
- Data-hygiene note: the local store's newest X row is a "CLI audit smoke test" tweet dated 2026-10-24 13:06:59 UTC, but it **no longer exists on X** (absent from the live user timeline and from `from:` search; live statuses_count is 80 vs that row's 81) — it was deleted outside this session, so the Oct 19 post is the real "latest post". Nothing in this session deleted it or anything else.
- Aggregate scope note: the 30-day `feed_analytics` headline (1,068 likes / 52 comments / 32,040 views) spans **all** imported feed posts (X + Reddit + LinkedIn + Bluesky rows in the local store); the X-only portion of those aggregates is the 0-likes / 1-reply / 11-views total above.

### Caveats (tool-path failures encountered, none blocking)
- `x_get_post_analytics` (the impressions/metrics endpoint) returns **HTTP 403** on this account's cookie-auth path — X requires OAuth 1.0a/2.0 User Context for that endpoint. `doctor` confirms the account itself is healthy; per-post numbers above come from the live GraphQL timeline payloads instead.
- `x_tweet_detail` returns an empty `tweet_results` shape for every tweet tried (including known-live ones) — the skill's documented empty-shape trap; treated as unreliable and not used as a source.
- `analytics post <uuid>` / `analytics_get_post` only accept internal `posts`-table UUIDs; imported feed rows (external_posts) are out of range, hence "Post not found" — per recipes.md's UUID warning.
- Publishing streak: 0 days (`streak --tz Asia/Kolkata`).

## Summary

Your X account (@_ishanparihar) is healthy but very early-stage and effectively flat: you have 9 followers (against 93 following), 80 lifetime tweets, and the analytics series holds only a single snapshot — 2026-10-24 with 0% change — so there is no measurable follower trend yet, just a flat line at nine. Your latest post actually live on X, "The research tab ranks what actually got engagement, not what you hoped would." (2026-10-19 13:59:35 UTC), has earned 3 views with zero likes, reposts, replies, quotes, or bookmarks, and it's not an outlier — the eight-post batch it belongs to has managed just 11 views and a single reply in total (that one landing on "The worst part of social media management is not the writing. It is the tab."), with a 0-day publishing streak since; the headline feed numbers (1,068 likes / 32,040 views over 30 days) are cross-platform store aggregates and not X's, whose own contribution is that same near-zero engagement. One measurement caveat: X's dedicated per-tweet analytics endpoint refused the cookie-auth path (HTTP 403 — it requires OAuth user-context scopes), so the per-post figures above come from the live timeline payloads, which agree across both the user-timeline and search reads. Net: a small, flat account whose most recent content is getting single-digit impressions — the number to move first is views per post, before follower growth can happen at all.
