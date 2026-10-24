# Verdict: Reddit 403 "whoa there, pardner" investigation

## TL;DR

**No, your Reddit connection is NOT broken. Do NOT reconnect your account.**

The 403 is **Reddit's edge/network policy block**, not an authentication failure.
Reddit is actively refusing certain unauthenticated-style listing endpoints
(`browse`, `search`, `user`, `inbox`, `post_detail`, `get_comments`, `browse`
via MCP) from your network/IP — the classic "whoa there, pardner!" block page
that Reddit serves when its WAF flags a request as bot-like. Meanwhile, the
**same credential set works fine** on other endpoints
(`reddit_get_rules`, `reddit_get_saved`, `reddit_list_friends`), and
`switchpost doctor` live-verifies the Reddit account (`d3vilzwrld`,
`7ev435uy`) as **healthy** every time.

## The evidence

### 1. Connection health: healthy, live-verified

`switchpost doctor` (run 3x, read-only) consistently reports:

```
- provider: reddit
    name: d3vilzwrld
    internal_id: 7ev435uy
    status: healthy
    draft_ok: true
    publish_capable: true
    live_verified: true
    live_verified_at: "2026-10-24T19:33:16.264719263+00:00"
```

No `refresh_needed`, no disabled integration (`audit` → `status: clean`,
`disabled_integrations: 0`), no token expiry. Per the skill's error-recovery
table, this is NOT "not connected", NOT "token expired", NOT "provider down".

### 2. The 403 is endpoint-specific, not account-wide

Same session, same credentials, same run window:

| Endpoint | Result |
|---|---|
| `reddit browse rust` (CLI) | 403 network-policy block page |
| `reddit browse algotrading` (CLI) | 403 network-policy block page |
| `reddit search "rust"` (CLI) | 403 network-policy block page |
| `reddit user d3vilzwrld` (CLI) | 403 network-policy block page |
| `reddit inbox` (CLI) | 403 network-policy block page |
| `reddit_post_detail` (MCP) | 403 network-policy block page |
| `reddit_get_comments` (MCP) | 403 network-policy block page |
| `reddit_browse` (MCP) | 403 network-policy block page |
| `reddit_user_info` (MCP) | 403 network-policy block page |
| `reddit_get_karma` (MCP) | Error result (decode failed — same class) |
| **`reddit_get_rules` (MCP)** | **SUCCESS — full r/rust rules returned** |
| **`reddit_get_saved` (MCP)** | **SUCCESS — returned the account's own saved posts, WITH a live `modhash`** |
| **`reddit_list_friends` (MCP)** | **SUCCESS — authenticated friend list returned** |

The three successes are the key evidence. `reddit_get_saved` returns the
account's private saved-posts listing **including a modhash** — that only
works if the cookie session is valid and authenticated. `reddit_list_friends`
is likewise an authenticated-only endpoint. A broken or expired session
returns 401/invalid-auth errors, not a WAF block page that says "your
request has been blocked due to a network policy" and invites you to "try
logging in or creating an account".

### 3. Not a machine-wide network failure

`x timeline` over the same network egress works perfectly (returned 105
timeline entries). The problem is not DNS, TLS, routing, or a dead uplink —
it is Reddit's WAF specifically.

### 4. Persistent, not transient

Immediate retries (`reddit browse rust` twice, ~20 min apart) both 403'd
with fresh block codes (`01a16f53-…`, `01a16f67-…`). Each request gets a NEW
block code, meaning Reddit's WAF is actively evaluating and refusing these
requests, not serving a stale cached denial.

## Diagnosis

Reddit's edge (the "whoa there, pardner!" block page) is refusing the
request pattern that `browse`/`search`/`user`/`inbox`/`post_detail`/`get_comments`
use — most likely the unauthenticated www.reddit.com listing endpoints that
Reddit aggressively rate-limits/blocks from datacenter/VPS IPs. The skill's
platform-matrices note says Reddit is "cookie auth — richest access", so
these listing reads may be hitting www.reddit.com without a full browser-grade
session fingerprint (or with a User-Agent/HTTP fingerprint Reddit's WAF has
decided to block from your IP). Notably the block page itself says: "make sure
your User-Agent is not empty and is something unique and descriptive" — a
fingerprint/UA-shaped block, exactly what Reddit serves unauthenticated
scrapers.

Which paths work vs. fail also maps to auth shape: the three that succeed
(`get_saved`, `list_friends`, `get_rules`) succeed because they present valid
session cookies; the ones that fail are the ones that present fewer/different
auth signals (or hit WAF-monitored listing routes). Either way, the
**credential** is fine — Reddit's network policy is rejecting the
**request**, not the identity.

## What to do

1. **Do NOT reconnect** — the cookies/tokens are valid and working. Reconnecting
   cannot change how Reddit's WAF treats an IP+fingerprint, and it risks
   invalidating a perfectly good session.
2. **Wait and retry later** — Reddit edge blocks like this are typically
   IP/velocity-based and clear on their own (hours). Re-run:
   `switchpost reddit browse <sub> --limit 25` periodically.
3. **If it persists**, options within the skill's toolkit (all read-only-safe):
   - Use the **local unified feed** for Reddit reads:
     `switchpost feed --provider reddit` — works from the local store
     (verified: returned 20 imported posts) and doesn't touch the Reddit WAF
     at all.
   - Use the **MCP tools that still work**: `reddit_get_rules`,
     `reddit_get_saved`, `reddit_list_friends` keep functioning.
   - If the operator wants more, `switchpost connect reddit` (or
     `connect-all`) would re-import fresh browser cookies — but per the
     evidence above, **that is not warranted here**: the existing cookies are
     demonstrably valid (modhash returned, authenticated listings succeed).
4. If it NEVER clears, then it's an IP-reputation issue (VPS/datacenter IP
   blocked by Reddit's WAF) — the fix is running from a different network
   (e.g. residential IP), not a reconnect inside switchpost.

## Commands run (all read-only)

```bash
./target/release/switchpost doctor
./target/release/switchpost reddit browse rust --sort hot --limit 5
./target/release/switchpost providers --json
./target/release/switchpost mcp-call integrations_list --args '{}'
./target/release/switchpost reddit user d3vilzwrld
./target/release/switchpost reddit search "rust"
./target/release/switchpost reddit inbox
./target/release/switchpost mcp-call reddit_get_karma --args '{}'
./target/release/switchpost mcp-call reddit_get_karma --args '{}' --json
./target/release/switchpost mcp-call reddit_user_info --args '{"username":"d3vilzwrld"}'
./target/release/switchpost audit
./target/release/switchpost mcp-call reddit_get_rules --args '{"subreddit":"rust"}'
./target/release/switchpost mcp-call reddit_get_saved --args '{}'
./target/release/switchpost mcp-call reddit_post_detail --args '{"post_id":"1e5b14e"}'
./target/release/switchpost mcp-call reddit_get_comments --args '{"post_id":"1e5b14e","limit":3}'
./target/release/switchpost mcp-call reddit_browse --args '{"subreddit":"rust","limit":3,"sort":"hot"}'
./target/release/switchpost mcp-call reddit_list_friends --args '{}'
./target/release/switchpost reddit browse algotrading --limit 3
./target/release/switchpost x timeline --count 2
./target/release/switchpost doctor   (re-run to confirm)
./target/release/switchpost reddit browse rust --limit 3   (retry)
./target/release/switchpost feed --provider reddit
```

Full transcript: `commands.txt` (same directory).
