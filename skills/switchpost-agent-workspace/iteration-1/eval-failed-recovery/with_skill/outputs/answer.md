# Failed-post recovery — workflow, failure-reason command, and discovery run

## 1. The exact switchpost workflow to diagnose and fix three failed posts

The skill's failed-post recovery loop (SKILL.md "Core routes" row *Fix a failed
post*, expanded in `references/recipes.md` § *Failed-post recovery*):

```bash
switchpost posts list --state error     # 1. find what failed (IDs + state)
switchpost posts get <POST_ID>          # 2. read error_message for the cause
switchpost posts update <ID> ...        # 3. fix the cause (content/title/media/settings)
switchpost posts publish <POST_ID>      # 4. retry immediately
# OR
switchpost posts delete <POST_ID>       # drop it if it's not worth retrying
```

Step by step, for three failed posts:

1. **List the failures** — `switchpost posts list --state error`. This returns
   every post whose state is `error` with its `id`. (The same command covers
   `draft|queued|published` if you need to cross-check.)
2. **Read each failure reason** — `switchpost posts get <POST_ID>` for each of
   the three IDs. The `error_message` field is the failure reason. This is
   read-only and safe to run on all three.
3. **Triage by cause** (per the skill's error-recovery table, and "Always run
   `switchpost doctor` first when a provider call fails unexpectedly"):
   - Run `switchpost doctor` to separate "not connected" / "token expired" /
     "provider down" / "rate limited" from a post-specific problem.
   - `Missing subreddit in settings` (or `board`, `channel`,
     `publication_id`, …) → the provider target was never set. Fix with
     `switchpost posts update <ID> --settings '{"subreddit":"rust"}'` then
     `posts publish <ID>`.
   - Content problem (too long, bad media URL) → `switchpost posts update <ID>
     --content "fixed text"` / `--media <url>`, then `posts publish <ID>`.
   - `Rate limit exceeded` / HTTP 429 → the scheduler already retried with
     exponential backoff; a post only lands in `error` after retries are
     exhausted. Wait for quota reset (`doctor` shows provider health) or
     reschedule to a later slot (`switchpost posts schedule <ID> --at
     <ISO8601>`), then retry.
   - `Token expired` → `switchpost connect <provider>` again (auto-refresh
     usually handles it), then retry.
   - Provider not connected → `switchpost connect <provider>`, then retry.
4. **Retry each fixed post** — `switchpost posts publish <POST_ID>` publishes
   a queued or errored post immediately via the provider. Fix first
   (`posts update`), then publish — re-publishing an unfixed post just burns
   another attempt.
5. **Verify** — `switchpost posts list --state error` again should no longer
   show them; `switchpost posts get <ID>` shows state `published`.

Note on what "failed" means: the scheduler retries `RateLimited`/`Network`
failures with exponential backoff automatically, so a post in `error` has
exhausted its retries — retrying by hand is the point of `posts publish`.
Every publish attempt (success or failure) is also recorded, and the
MCP side offers the same lifecycle (`mcp-call posts_get`, `posts_publish`,
`posts_delete` with `delete_from_platform: true`).

## 2. Which command shows each post's failure reason

**`switchpost posts get <POST_ID>`** — its `error_message` field is the
failure reason. `switchpost posts list --state error` finds the failed posts
(and shows their IDs), but the detailed cause per post comes from
`posts get <ID>`. Use `--json`/`--pretty` when a machine or human
respectively needs the full structured record.

## 3. Discovery I ran (read-only)

Commands run, in order (also in `commands.txt`):

| # | Command | Result |
|---|---|---|
| 1 | `./target/release/switchpost posts list --state error` | `count: 0` — **no posts are currently in `error` state**. The 3 "last night" failures were assumed per the task, not present in this DB. |
| 2 | `./target/release/switchpost doctor` | 3 connected providers, all `healthy`, 0 errors: github (publish_capable: false — content limit 0, drafts only), x (Ishan Parihar, publish_capable), reddit (d3vilzwrld, publish_capable). Media OK. 3 missing_oauth (linkedin, facebook, instagram — credentials configured but not connected). So no provider outage or token expiry is pending. |
| 3 | `./target/release/switchpost posts list --limit 10` | `total: 0` — the posts table is empty in this environment, consistent with #1. |
| 4 | `./target/release/switchpost agent --json` | Step Zero orientation: connected = github, x, reddit; command/workflow index confirmed `posts list|create|publish` lifecycle. |
| 5 | `./target/release/switchpost posts get 00000000-0000-0000-0000-000000000000` | Confirms the failure-reason read path exists and errors cleanly for a bad ID (`error: "Post … not found"`, exit 2, error object on stdout per the skill's error contract). |

Also ran `--help` on `posts list`, `posts`, `posts get`, `posts update`,
`doctor` to confirm exact flag names (`--state error`, `--limit`,
`--content`, `--settings`) before using them — the skill's "Unknown flag →
`switchpost <command> --help` — never guess" rule.

**What I did NOT run (would be needed if the 3 failures were real):**
`posts get <each error ID>` to read the actual `error_message`, `posts
update` + `posts publish` to fix and retry. Discovery was capped at listing
the error state and read-only health checks, as instructed.

## Where the workflow comes from

- `skills/switchpost-agent/SKILL.md` — "Core routes (90% of jobs)" row:
  *Fix a failed post*: `posts list --state error` → `posts get <ID>` (read
  error_message) → `posts update` + `posts publish`.
- `skills/switchpost-agent/references/recipes.md` § *Failed-post recovery*
  (lines 74–101): the command block above, the retry/backoff note, and the
  MCP extras.
- `skills/switchpost-agent/SKILL.md` § *Error recovery*: the symptom→fix
  table (`Rate limit exceeded`, `Token expired`, `Missing subreddit in
  settings`, provider-not-connected rows) and the "Always run `switchpost
  doctor` first" rule.
