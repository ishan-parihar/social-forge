# Answer: repeat a post every 7 days until Nov 30, 2026

## The exact command

Per the skill (`SKILL.md` "MCP tools from the shell" + `references/recipes.md` "MCP extras"), recurring/evergreen repeats are the `posts_repeat` MCP tool, reached from the shell through `mcp-call`:

```bash
./target/release/switchpost mcp-call posts_repeat --args '{"post_id":"550e8400-e29b-41d4-a716-446655440000","interval_days":7,"end_date":"2026-11-30"}'
```

## The exact JSON args

```json
{
  "post_id": "550e8400-e29b-41d4-a716-446655440000",
  "interval_days": 7,
  "end_date": "2026-11-30"
}
```

## Why these args (all from the skill snapshot)

- `switchpost mcp-call <tool> --args '<json>'` is the skill's documented path for reaching ANY MCP tool from the shell (SKILL.md "MCP tools from the shell"). `posts_repeat` has no native CLI subcommand — `switchpost posts --help` lists no `repeat` action, so the MCP bridge is the route.
- Tool discovery (run live, dry-run): `switchpost mcp-call tools_search --args '{"query":"repeat"}'` returns exactly one tool:
  > `posts_repeat` (toolset `core`, enabled): "Set up a recurring/evergreen post. The scheduler creates a new queued copy every interval_days days until end_date. Wraps POST /api/posts/{id}/repeat."
- `references/recipes.md` describes it as: "posts_repeat (copies a scheduled post every N days until an end date — returns group_id + post_ids)".
- Parameter names were confirmed against the live binary's own error contract (validation happens BEFORE any write, so these probes created nothing):
  - `--args '{"post_id":"nonexistent-schema-probe",...}'` → `Invalid post_id UUID` (names the param, proves UUID parsing is first)
  - `--args '{"post_id":"00000000-0000-0000-0000-000000000000",...}'` → `Post not found` (proves the format check passes, lookup happens next, still no write)
- `interval_days: 7` = "every 7 days". `end_date: "2026-11-30"` = "until Nov 30 2026" (ISO date). If a full timestamp is preferred, `"2026-11-30T23:59:59Z` is the ISO8601 form the skill uses for repeat-until values elsewhere (autopost `--repeat-until`).

## Expected result

The tool returns `group_id` + `post_ids` (per recipes.md): the repeat is registered and the scheduler creates a new queued copy every 7 days, stopping after the end date. The post must be a SCHEDULED post ("copies a scheduled post") — if the hypothetical post were still a draft, schedule it first:

```bash
./target/release/switchpost posts schedule 550e8400-e29b-41d4-a716-446655440000 "2026-11-02T09:00:00Z"
```

## NOT executed (dry-run honored)

The repeat was deliberately NOT created — the post id is hypothetical. Only help/discovery commands and two pre-write validation probes (both of which failed before touching the database) were run.
