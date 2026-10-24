# Failed-post recovery — the switchpost workflow

Eval: `failed-post-recovery` (old_skill run). Task: three scheduled posts
failed last night; describe the exact workflow to diagnose and fix them, run
the DISCOVERY part read-only, and name the command that shows each post's
failure reason.

All commands below were run against `./target/release/switchpost`.

---

## 0. One flag to know first: `--state error`, not `errored`

`posts list --help` advertises the states as "draft, queued, published,
errored", but the filter's vocabulary is `error`. The proof, from the scratch
DB used for this run (3 `error` posts + 1 `draft` post):

```
posts list --state errored --json  →  total: 4 | states: ['Error','Error','Error','Draft']
posts list --state error   --json  →  total: 3 | states: ['Error','Error','Error']
```

An unknown state value falls through to the **unfiltered** list, so
`--state errored` silently shows drafts and queued posts too — it looks like
it worked and lies about the count. Use `--state error`. (The same vocabulary
is confirmed by the `posts_list` MCP tool description: `draft|queued|published|error`.)

## 1. The exact workflow

### Step 1 — Find the failed posts (discovery)

```bash
switchpost posts list --state error            # ids, content, state, error_message, integration_id
switchpost posts list --state error --json    # when parsing
```

This is the discovery command. Each row already carries `error_message` (the
upstream failure text), plus `integration_id` (which account it was bound
for), `scheduled_at`, and `platform_post_url` (empty when nothing went out).
`total` is the count of live errored posts; use `--offset` to page if
`hint` says more exist.

### Step 2 — Read each post's failure reason

**`switchpost posts get <POST_ID>`** is the command that shows each post's
failure reason — the full `error_message` field, the complete `settings`
JSON (so you can see a missing `subreddit` / `board` / `channel` target with
your own eyes), `media`, `first_comment`, and the timestamps. `posts list`
truncates long strings; `posts get` does not, so use it for the rows you are
about to act on:

```bash
switchpost posts get <POST_ID>
# → error_message: "API error: Authorization: You have reached your daily limit ... (344)"
#   settings: {}          ← e.g. Reddit post with NO subreddit
```

The list view and `posts get` both carry `error_message`; `posts get <ID>` is
the per-post failure-reason read. (The MCP equivalent is
`switchpost mcp-call posts_get --args '{"id":"<UUID>"}'`.)

### Step 3 — Triage the failure (classify, don't guess)

Read the `error_message` and sort each post into a bucket. The skill's error
contract maps them to fixes:

| error_message looks like | Bucket | Fix |
|---|---|---|
| `Missing subreddit in settings` / `Missing ... in settings` | provider target not set | fix settings, then retry (Step 4) |
| `Token expired` / `invalid_grant` / 401-ish auth text | credential problem | `switchpost doctor` → `switchpost connect <provider>` again |
| `Rate limit exceeded` / HTTP 429 / `X API rate limit` / X daily-limit (344) | provider quota | wait for reset; scheduler retries with exponential backoff; reschedule for later |
| `No <provider> integration found` | account not connected | `switchpost connect <provider>` |
| `Invalid integration_id format` | ID systems swapped | `mcp-call integrations_list` for UUIDs |

Two read-only triage helpers before touching anything:

```bash
switchpost doctor                            # per-provider health: healthy / token expired / not connected
switchpost mcp-call analytics_summary --args '{}'   # total_posts + failed count vs published/queued/draft
switchpost mcp-call integrations_list --args '{}'   # map integration UUID → provider, confirm none disabled
switchpost notifications list --limit 20            # post_failed events with the reason in the body
```

`doctor` first is the skill's own rule for any unexpected provider failure —
it distinguishes "not connected" from "token expired" from "provider down",
which is exactly the fork this workflow depends on.

### Step 4 — Fix the underlying cause, per post

- **Missing provider target** (the classic: post created, queued, then failed
  at publish with `Missing subreddit in settings`): add the key the provider
  reads, then retry:
  ```bash
  switchpost posts update <POST_ID> --settings '{"subreddit":"rust"}'
  # same shape for Pinterest board, Discord channel, Hashnode publication_id,
  # Telegram-user target — the key is named in the error_message itself
  ```
  Note: `--settings` **replaces** the settings object (`media`/`settings` are
  written wholesale on update), so pass the complete object you want — read
  the current one with `posts get` first.
- **Content problem** (over-limit, rejected): edit then retry:
  `switchpost posts update <POST_ID> --content "…"`
- **Credential problem**: `switchpost connect <provider>` (prints OAuth URL or
  imports cookies), or `switchpost doctor` to watch it go healthy again.
- **Rate limit / quota** (e.g. X's daily send limit, code 344): nothing to
  edit — pick a later slot:
  ```bash
  switchpost posts schedule <POST_ID> 2026-10-25T09:00:00Z
  # → state flips back to queued and the scheduler picks it up
  ```

### Step 5 — Retry / re-queue the post

`posts publish` explicitly accepts a **queued or errored** post ("Publish a
queued or errored post immediately via the provider"), so both retry shapes
are valid:

```bash
switchpost posts publish <POST_ID>     # retry NOW (errored → provider, immediately)
switchpost posts schedule <POST_ID> <ISO8601>   # or: back to queued for the scheduler
```

- Retry now when the cause is fixed and the slot is right (missing subreddit
  fixed, token refreshed).
- `posts schedule` when the platform quota is the cause (X daily limit): it
  resets state to `queued` with the new `scheduled_at` and wakes the
  scheduler.
- Give up on one: `switchpost posts delete <POST_ID>` (cancel), or edit the
  content down and retry.

### Step 6 — Verify

```bash
switchpost posts get <POST_ID>          # state should be published; platform_post_url filled in
switchpost posts list --state error     # the errored set should shrink
switchpost mcp-call analytics_summary --args '{}'   # failed count drops
```

A successful publish sets `state: Published` and fills `platform_post_id` /
`platform_post_url`, so `posts get` is the single verification read. If it
re-errors, the new `error_message` tells you the next bucket.

## 2. Which command shows each post's failure reason?

**`switchpost posts get <POST_ID>`** — it prints the post's full
`error_message` (plus `settings`, `media`, `first_comment`, timestamps).
The `posts list --state error` discovery output also shows a (possibly
truncated) `error_message` per row, which is enough to triage; `posts get`
gives the untruncated per-post detail before you act. For the retry-history
audit trail behind that message, the `publish_attempts` table (via the REST
`/api/analytics` surface) records every attempt with its error — but the CLI
read for the failure reason is `posts get <ID>`.

## 3. What discovery I actually ran (read-only)

Against the real DB first — the live store currently has **0 live posts in
error state** (`posts list --state error` → `total: 0`; every historical row
in this DB, including its 2 error-state rows, is soft-deleted), so to make
the discovery verifiable I reproduced the "three failed last night" scenario
in an isolated scratch DB (`target/sf-eval-scratch/eval.db`, seeded via
sqlite3, never touching the real store):

1. `switchpost posts list --state error` → **3 posts, each with its
   `error_message` in the row**: X daily-limit (344) on x,
   `Missing subreddit in settings` on reddit, `Token expired; refresh failed
   (invalid_grant)` on linkedin. Full TOON rows included `id`, `state`,
   `integration_id`, `scheduled_at`, `error_message`.
2. `switchpost posts get 10000000-0000-0000-0000-000000000001` → the same
   post in full detail, `error_message` untruncated, `settings: {}` visible
   (confirming the Reddit-style missing-target shape).
3. `switchpost posts list --state errored` vs `--state error` (with a draft
   row present) → proved the `errored` fallthrough trap documented in §0.
4. Read-only context on the real store: `switchpost doctor` (3 providers
   healthy), `switchpost providers --json`, `switchpost agent --json`,
   `switchpost audit` (clean; writer posture OK),
   `mcp-call integrations_list`, `mcp-call analytics_summary` (live totals),
   `notifications list` (shows `post_failed` events with the reason in the
   body), `mcp-call posts_list --state error`, and sqlite3 SELECTs against
   the real `posts`/`publish_attempts`/`notifications` tables (all read-only).

No state-changing command was run: no `posts update`, `posts publish`,
`posts schedule`, `posts delete`, `connect`, or `media` write. The only
writes anywhere were sqlite INSERT/UPDATE **inside the throwaway scratch DB**
under `target/`, used to make the discovery observable.

## 4. Assumed three failures — the workflow applied

With the three seeded failures as the worked example, the recovery would be:

| Post (integration) | error_message | Diagnosis step | Fix | Retry |
|---|---|---|---|---|
| `10000000-…-01` (x) | `API error: Authorization: You have reached your daily limit ... (344)` | rate-limit bucket | `posts find-slot` for a later slot | `posts schedule <ID> <tomorrow 09:00Z>` → queued |
| `10000000-…-02` (reddit) | `Missing subreddit in settings` | settings bucket (`posts get` shows `settings: {}`) | `posts update <ID> --settings '{"subreddit":"rust"}'` | `posts publish <ID>` now, or `posts schedule` |
| `10000000-…-03` (linkedin) | `Token expired; refresh failed (invalid_grant)` | credential bucket | `switchpost doctor` → `switchpost connect linkedin` | `posts publish <ID>` after doctor shows healthy |

then `posts get <ID>` on each until every one shows `state: Published`.
