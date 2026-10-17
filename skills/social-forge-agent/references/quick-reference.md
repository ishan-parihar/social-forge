# Social Forge — Quick Reference

Routing rules, ID-discovery chains, error recovery and the v11 MCP contracts.
Per-platform command and tool listings live in `providers.md` (installed) and
in `social-forge <command> --help` / `social-forge mcp-tools --pretty`; they are
not repeated here.

---

## Output Format

All commands output JSON. Use `--pretty` for human-readable or `--json` for machine-parseable.

**Exit codes**: `0` = success, `1` = error. Error details are in stderr as JSON.

---

## Finding IDs

Before many commands, you need integration IDs. Here's how to find them:

```bash
social-forge providers --pretty                    # All integration UUIDs + names
social-forge linkedin-page list --pretty           # LinkedIn page IDs
social-forge instagram media <IG_ID> --limit 1     # Verify Instagram account
social-forge reddit browse <sub> --limit 1         # Verify Reddit connection
social-forge facebook pages --pretty               # Facebook page IDs
# YouTube: no list command — pass a known channel ID to `youtube stats <CHANNEL_ID>`
```

---

## Discovery

```bash
social-forge --help                    # All commands
social-forge <cmd> --help              # Args for a command
social-forge providers --pretty        # Connected accounts
social-forge doctor                    # Provider health
social-forge connect <provider>        # Connect a new account
social-forge mcp-tools --toolset x --pretty   # Tools in one toolset (sizing)
```

---

## Routing

- Native CLI command exists → use it (`providers.md` lists each provider's).
- No native command → `social-forge mcp-call <tool> --args '<json>'`. It runs in-process and works while the `serve` daemon holds the writer lock.
- Unknown tool name → `social-forge mcp-tools --pretty`, then `--toolset <set>` to narrow it.

---

## Auth Priority

1. DB-stored cookie tokens
2. Browser cookie extraction (Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB
4. Environment variables (`X_AUTH_TOKEN`, `X_CT0`, etc.)

---

## Errors

```bash
social-forge doctor                    # Check all providers
social-forge providers --pretty        # See what's connected
social-forge connect <provider>        # Fix missing provider
social-forge <cmd> --help              # Fix wrong flags
```

---

## MCP tool contracts (v11 — current)

- **`mcp-call`** is real: `social-forge mcp-call <tool> --args '<json>'`. It runs in-process — works while the `serve` daemon holds the writer lock (thin MCP mode).
- **Remote/HTTP MCP (F8)**: with the `serve` daemon running, the same tool surface is served at `POST {APP_URL}/mcp` (MCP streamable-HTTP). Connect an MCP client by URL with `Authorization: Bearer sf_<api-key>` (create one in the WebUI's developer page) or the `sf_session` cookie. Zero processes spawned; toolsets behave as in stdio mode (`SOCIAL_FORGE_TOOLSETS` at daemon boot).
- **Multi-account**: most `*_create_*` tools accept `integration` (UUID or `provider:Name`). Empty → first connected account.
- **Toolsets**: boot default advertises `core` only. `SOCIAL_FORGE_TOOLSETS=x,reddit` pre-enables; at runtime `tools_search {"query":"…"}` finds any tool and reports its set + enabled flag; `tools_enable {"sets":["x"]}` adds sets for the session.
- **Stage v2**: `posts_stage` takes `overrides` (per-provider content/settings), `dry_run` (readiness only, writes nothing), `force` (stage despite readiness errors), `group_id` (attach to a campaign), `scheduled_at: "next-slot"` (each account's next free slot). Readiness errors REJECT that integration unless forced.
- **Lifecycle**: one campaign `group_id` works with `posts_reschedule` (`move_group: true` shifts the whole campaign), `posts_unschedule`, `posts_publish`, `posts_delete`, `posts_group`.
- **Repeat**: `posts_repeat` requires `scheduled_at` + `end_date` (RFC3339) and returns the copies (`group_id`, `post_ids`, `scheduled_dates`).
- **UTM**: put `"utm_campaign": "…"` in a post's settings — every plain URL in its content gets `utm_source=<platform>&utm_medium=social&utm_campaign=<campaign>` at publish time.
