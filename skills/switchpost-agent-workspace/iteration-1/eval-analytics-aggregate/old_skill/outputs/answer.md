# How is my X account doing? (read-only)

**Account**: @_ishanparihar (Ishan Parihar, X user id 3301263462) — provider healthy per `switchpost doctor` (live-verified round trip).

## Follower trend

- `x_get_analytics` (and the CLI `analytics get x`, tried `--days 7` and `--days 30`) returns the cached analytics series for X. The cache holds a **single data point**: **9 followers on 2026-10-24**, with `percentage_change: 0`. There is no day-over-day follower history recorded yet, so no upward/downward trend can be shown — the recorded trend is flat at the one snapshot.
- Live profile (`x_get_me`, `x user _ishanparihar`) confirms the same numbers right now: **9 followers, 93 following, 80 lifetime tweets (statuses_count)**, 35 media, not verified, account created Jul 30 2015.

## Engagement on the latest post

Latest own post (most recent, excluding retweets):

- **Text**: "The research tab ranks what actually got engagement, not what you hoped would."
- **Posted**: Mon Oct 19 13:59:35 +0000 2026
- **Engagement**: **3 views/impressions, 0 likes, 0 retweets, 0 replies, 0 quotes, 0 bookmarks**

For context, that post was the last of an 8-post burst on Oct 19 (all within ~25 seconds): across the burst, the posts with view counts sum to **11 views and 1 reply total**, with 0 likes/RTs/quotes anywhere.

Note: `x_get_post_analytics` (the OAuth metrics endpoint) returned HTTP 403 for this cookie-authenticated account ("Supported authentication types are [OAuth 1.0a User Context, OAuth 2.0 User Context...]"), so per-post numbers were read via `x_tweet_detail` instead, which returns the same engagement fields.

Local store context (`analytics_summary`, 30 days): 25 posts tracked — 1 published, 15 queued, 7 draft, 2 failed.

## Summary

Your X account is essentially dormant at a very small scale: you have 9 followers (following 93, 80 lifetime tweets), and the analytics cache holds only a single follower snapshot — 9 as of 2026-10-24 with 0% change — so there is no recorded follower trend to speak of, just a flat line at today's count. Your latest post ("The research tab ranks what actually got engagement, not what you hoped would.", posted Oct 19, 2026 at 13:59 UTC) has 3 views with zero likes, retweets, replies, quotes or bookmarks, and the eight-post burst it belongs to managed just 11 views and a single reply in total, so recent content is reaching almost no one. Within Switchpost you have 25 posts tracked over the last 30 days (1 published, 15 queued, 7 draft, 2 failed), meaning the pipeline has inventory queued but very little has actually gone live; the account's connection itself is healthy and live-verified, so the constraint is audience and publishing cadence, not access.

## Commands run (read-only, via ./target/release/switchpost)

```
./target/release/switchpost providers --json
./target/release/switchpost mcp-call integrations_list --args '{}'
./target/release/switchpost mcp-call tools_search --args '{"query":"x_get_analytics"}'
./target/release/switchpost mcp-call x_get_analytics --args '{}'
./target/release/switchpost analytics get x --days 7 --json
./target/release/switchpost analytics get x --days 30 --json
./target/release/switchpost mcp-call x_get_me --args '{}'
./target/release/switchpost x user _ishanparihar --json
./target/release/switchpost mcp-call tools_search --args '{"query":"x_user_tweets"}'
./target/release/switchpost mcp-call x_user_tweets --args '{"user_id":"3301263462"}'
./target/release/switchpost mcp-call x_user_tweets --args '{"user_id":"3301263462"}' --json
./target/release/switchpost mcp-call tools_search --args '{"query":"x_get_post_analytics"}'
./target/release/switchpost mcp-call x_get_post_analytics --args '{"tweet_id":"<latest own tweet id>"}'
./target/release/switchpost mcp-call tools_search --args '{"query":"x_tweet_detail"}'
./target/release/switchpost mcp-call x_tweet_detail --args '{"tweet_id":"<latest own tweet id>"}' --json
./target/release/switchpost mcp-call analytics_get --args '{"provider":"x","days":30}'
./target/release/switchpost mcp-call analytics_summary --args '{}'
./target/release/switchpost posts list --state published --limit 10 --json
./target/release/switchpost posts list --limit 30 --json
./target/release/switchpost mcp-call feed_analytics --args '{}'
./target/release/switchpost doctor
```

No create/schedule/publish/delete commands were run. Outputs saved to:
- `skills/switchpost-agent-workspace/iteration-1/eval-analytics-aggregate/old_skill/outputs/answer.md` (this file)
- `skills/switchpost-agent-workspace/iteration-1/eval-analytics-aggregate/old_skill/outputs/commands.txt`
