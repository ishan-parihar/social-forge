# Verdict: Your Reddit connection is NOT broken. Do NOT reconnect.

**One-line answer:** The 403 "whoa there, pardner!" page is Reddit's edge blocking
your machine's **IP address** (datacenter/VPN-hostile network policy on Reddit's
`www` endpoints), not a problem with your account, token, or switchpost
connection. Your credentials are healthy and proven working — other Reddit paths
succeed right now.

## Evidence (read-only, gathered 2026-10-24)

| Check | Command | Result |
|---|---|---|
| Provider health round-trip | `switchpost doctor` | Reddit `status: healthy`, `live_verified: true`, `publish_capable: true` — the authenticated live check **passed** (3/3 providers healthy, 0 errors) |
| Integration row | `switchpost mcp-call integrations_list --args '{}'` | Reddit `disabled: false`, `refresh_needed: false` — no flags say "reconnect" |
| Working read path (OAuth) | `switchpost mcp-call reddit_get_saved --args '{"limit": 3}'` | **Succeeded** — exit 0, returned real saved posts (`Forexstrategy` …), with a live `modhash` |
| Working read path (OAuth) | `switchpost mcp-call reddit_list_friends --args '{}'` | **Succeeded** — exit 0, `kind: UserList` |
| Blocked read paths (www) | `switchpost reddit browse rust --limit 3` | exit 2, 403 "whoa there, pardner!" — reproduced your error |
| Blocked read paths (www) | `switchpost reddit user d3vilzwrld` / `reddit search "rust lang"` / `reddit inbox` | exit 2, same 403 block |
| Read-only posture audit | `switchpost audit` | `status: clean` — no disabled integrations, no credential-source problems |
| Orientation | `switchpost agent --json`, `switchpost providers --json` | Reddit connected as `d3vilzwrld` (`internal_id: 7ev435uy`) |

## Why this happened

- switchpost's Reddit reads use two endpoint families. The **`www.reddit.com`**
  paths (`reddit browse`, `reddit search`, `reddit user`, `reddit inbox`,
  `reddit_user_info`, `reddit_get_karma`) are the ones Reddit's edge network
  blocks for datacenter IPs with its "network policy" wall. The **OAuth
  `oauth.reddit.com`** paths (`reddit_get_saved`, `reddit_list_friends`, and
  doctor's live-verify round-trip) go through and return real data.
- Reddit's own block page says it: *"Your request has been blocked due to a
  network policy"* — and suggests logging in via a browser or registering
  developer credentials, which is what an IP/edge block looks like. It is not
  an invalid-token shape: an auth failure would be a 401 or a
  `refresh_needed` flag, and you have neither.
- `doctor`'s live-verified check **succeeded at 19:08 UTC**, minutes before the
  403s, which rules out broken credentials outright: a healthy authenticated
  round-trip and a blocked www fetch cannot coexist if the account were broken.

## Should you reconnect? No.

Reconnecting would not change your IP, so it would not fix the block, and you
would risk re-consent friction for nothing. The skill's own error-recovery
table classifies this exact error as **environment, not auth**.

## What actually helps (optional, no action forced)

- **Read Reddit another way, right now:** `switchpost mcp-call reddit_get_saved
  --args '{}'` and `reddit_list_friends` work; `switchpost feed` (the unified
  local feed) and imported feeds are unaffected because they read from your
  local DB.
- **Publishing is unaffected** — `doctor` shows `publish_capable: true` and the
  publish path is not the blocked www-read path. If a scheduled Reddit post
  ever hits the same wall, the scheduler retries with exponential backoff and
  `switchpost doctor` will show the provider state.
- **If you need www-path reads back:** run from a residential IP (not this
  datacenter/VPN egress), or wait out the policy window. Nothing in switchpost
  needs to change. `switchpost connect reddit` again would only be warranted if
  `doctor` ever reports Reddit `unhealthy` or the integration row shows
  `refresh_needed: true` — neither is true today.

## Commands run (read-only; every claim above maps to one of these)

```bash
./target/release/switchpost agent --json
./target/release/switchpost providers --json
./target/release/switchpost doctor
./target/release/switchpost audit
./target/release/switchpost mcp-call integrations_list --args '{}'
./target/release/switchpost reddit browse rust --limit 3
./target/release/switchpost reddit user d3vilzwrld
./target/release/switchpost reddit search "rust lang"
./target/release/switchpost reddit inbox
./target/release/switchpost mcp-call tools_search --args '{"query":"reddit"}'
./target/release/switchpost mcp-call reddit_get_saved --args '{"limit": 3}'
./target/release/switchpost mcp-call reddit_list_friends --args '{}'
./target/release/switchpost mcp-call reddit_get_karma --args '{}'
./target/release/switchpost mcp-call reddit_user_info --args '{"username": "d3vilzwrld"}'
```

Notes on reading the runs: `reddit browse/user/search/inbox` and
`reddit_user_info` exited 2 with the 403 block on **stdout** (per switchpost's
error contract, `{error, hint}` goes to stdout for platform shims — stderr was
empty). `reddit_get_saved` and `reddit_list_friends` exited 0 with real data.
`reddit_get_karma` failed with a decode error, not a 403 — likely the same
endpoint family being blocked but failing at parse; it does not change the
verdict (the account is proven healthy by two other authenticated reads plus
doctor's live round-trip).

**Bottom line:** Not broken. Environment block. Don't reconnect. Use the
OAuth-path reads (or a different egress IP) until you need www-path reads from
a residential network.
