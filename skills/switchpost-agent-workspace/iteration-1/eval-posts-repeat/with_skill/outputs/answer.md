# Task: Repeat post every 7 days until Nov 30, 2026 (DRY-RUN)

Post id (hypothetical): `550e8400-e29b-41d4-a716-446655440000`

## The exact command I would run

Recurring/evergreen repeat is an **MCP-extras tool**, not a native CLI subcommand
(`switchpost posts --help` shows no `repeat` subcommand). Per the skill
(SKILL.md "Core routes" + recipes.md "MCP extras"), the shell path is the
in-process MCP bridge:

```bash
switchpost mcp-call posts_repeat --args '{"post_id":"550e8400-e29b-41d4-a716-446655440000","interval_days":7,"end_date":"2026-11-30T23:59:59Z"}'
```

## The exact JSON args

```json
{
  "post_id": "550e8400-e29b-41d4-a716-446655440000",
  "interval_days": 7,
  "end_date": "2026-11-30T23:59:59Z"
}
```

Key contract points (from the skill's recipes.md):

- `end_date` must be a **FULL RFC3339 timestamp** — a date-only string
  (e.g. `"2026-11-30"`) is **rejected**. So "until Nov 30 2026" is encoded as
  the last instant of that day: `2026-11-30T23:59:59Z`.
- The scheduler creates a new queued copy of the post every
  `interval_days` (7) days until `end_date` is reached.
- Returns `group_id` + `post_ids` + `scheduled_dates` — capture these to
  manage or cancel the recurring set later.

## Optional pre-flight (safe, read-only)

```bash
switchpost posts get 550e8400-e29b-41d4-a716-446655440000
```

(For the real post, this would confirm the post exists and its current state.
The tool wraps `POST /api/posts/{id}/repeat`, so the post must already exist.)
Skipped here: the post id is hypothetical, so `posts get` would return an
error, and the task forbids creating the repeat.

## What was NOT run (dry-run discipline)

- No `mcp-call posts_repeat` call was executed — the post id is hypothetical.
- Only help/discovery commands were run (see `commands.txt`).

## How the answer was derived

1. SKILL.md "Core routes" table → Thread/recurring row points to
   `posts_repeat` (interval_days + FULL-RFC3339 end_date), exact params in
   recipes.md.
2. recipes.md (line 88-92) gives the exact shape:
   `{"post_id":"…","interval_days":7,"end_date":"2026-11-08T23:59:59Z"}` —
   `end_date` is a full RFC3339 timestamp, date-only rejected.
3. Verified via discovery (allowed):
   - `switchpost mcp-tools` → `posts_repeat` exists.
   - `switchpost mcp-call tools_search --args '{"query":"repeat"}'` →
     `posts_repeat,core,true` — "Set up a recurring/evergreen post. The
     scheduler creates a new queued copy every interval_days days until
     end_date. Wraps POST /api/posts/{id}/repeat."
   - `switchpost posts --help` → no native `repeat` subcommand, confirming
     the MCP bridge is the correct path.
