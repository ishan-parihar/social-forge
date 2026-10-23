# Switchpost — Quick Reference

Routing rules, ID-discovery chains, error recovery and the v11 MCP contracts.
Per-platform command and tool listings live in `providers.md` (installed) and
in `switchpost <command> --help` / `switchpost mcp-tools --pretty`; they are
not repeated here.

---

## Output Format

All commands output JSON. Use `--pretty` for human-readable or `--json` for machine-parseable.

**Exit codes**: `0` = success, `1` = error. Error details are in stderr as JSON.

---

## Finding IDs

Before many commands, you need integration IDs. Here's how to find them:

```bash
switchpost providers --pretty                    # All integration UUIDs + names
switchpost linkedin-page list --pretty           # LinkedIn page IDs
switchpost instagram posts <IG_ID>                # Verify Instagram account
switchpost reddit browse <sub> --limit 1         # Verify Reddit connection
switchpost providers --pretty                     # Facebook page IDs (internal_id)
# YouTube: no list command — pass a known channel ID to `youtube stats <CHANNEL_ID>`
```

---

## Discovery

```bash
switchpost --help                    # All commands
switchpost <cmd> --help              # Args for a command
switchpost providers --pretty        # Connected accounts
switchpost doctor                    # Provider health
switchpost connect <provider>        # Connect a new account
switchpost connect <provider> --wait # OAuth: prints auth_url, polls until the login lands
switchpost mcp-tools --toolset x --pretty   # Tools in one toolset (sizing)
```

### Logging a provider in (agent loop)

```bash
switchpost connect linkedin --wait --json &   # prints {status, auth_url, redirect_uri}
# drive a browser to auth_url, complete the login
# -> the CLI prints {status: connected, accounts: [...]} and exits 0
```

The `auth_url` is the PROVIDER's own consent page — no session cookie,
no `?token=` JWT, no popup. The provider redirects to `redirect_uri`
(`/api/auth/callback`), an auth-exempt route, and the server exchanges
the code there. If the platform says `redirect_uri_mismatch`, register
the `redirect_uri` value the CLI printed in the provider's developer
console.

Output is TOON by default; `--json` (compact) and `--pretty` (indented)
are global flags on every command.

---

## Routing

- Native CLI command exists → use it (`providers.md` lists each provider's).
- No native command → `switchpost mcp-call <tool> --args '<json>'`. It runs in-process and works while the `serve` daemon holds the writer lock.
- Unknown tool name → `switchpost mcp-tools --pretty`, then `--toolset <set>` to narrow it.

---

## Auth Priority

1. DB-stored cookie tokens
2. Browser cookie extraction (Chrome/Brave/Firefox/Zen)
3. OAuth tokens from DB
4. Environment variables (`X_AUTH_TOKEN`, `X_CT0`, etc.)

---

## Errors

```bash
switchpost doctor                    # Check all providers
switchpost providers --pretty        # See what's connected
switchpost connect <provider>        # Fix missing provider
switchpost <cmd> --help              # Fix wrong flags
```

---

## MCP tool contracts (v11 — current)

- **`mcp-call`** is real: `switchpost mcp-call <tool> --args '<json>'`. It runs in-process — works while the `serve` daemon holds the writer lock (thin MCP mode).
- **Remote/HTTP MCP (F8)**: with the `serve` daemon running, the same tool surface is served at `POST {APP_URL}/mcp` (MCP streamable-HTTP). Connect an MCP client by URL with `Authorization: Bearer sf_<api-key>` (create one in the WebUI's developer page) or the `sf_session` cookie. Zero processes spawned; toolsets behave as in stdio mode (`SWITCHPOST_TOOLSETS` at daemon boot).
- **Multi-account**: most `*_create_*` tools accept `integration` (UUID or `provider:Name`). Empty → first connected account.
- **Toolsets**: boot default advertises `core` only. `SWITCHPOST_TOOLSETS=x,reddit` pre-enables; at runtime `tools_search {"query":"…"}` finds any tool and reports its set + enabled flag; `tools_enable {"sets":["x"]}` adds sets for the session.
- **Stage v2**: `posts_stage` takes `overrides` (per-provider content/settings), `dry_run` (readiness only, writes nothing), `force` (stage despite readiness errors), `group_id` (attach to a campaign), `scheduled_at: "next-slot"` (each account's next free slot). Readiness errors REJECT that integration unless forced.
- **Lifecycle**: one campaign `group_id` works with `posts_reschedule` (`move_group: true` shifts the whole campaign), `posts_unschedule`, `posts_publish`, `posts_delete`, `posts_group`.
- **Repeat**: `posts_repeat` requires `scheduled_at` + `end_date` (RFC3339) and returns the copies (`group_id`, `post_ids`, `scheduled_dates`).
- **UTM**: put `"utm_campaign": "…"` in a post's settings — every plain URL in its content gets `utm_source=<platform>&utm_medium=social&utm_campaign=<campaign>` at publish time.

### Tool catalogue (names only — parameters via `tools_search` or `--help`)

Tools that the contracts above do not name. Each is a real MCP tool; call it by this name.

- **AI writing**: `ai_change_tone`, `ai_generate_bulk`, `ai_generate_post`, `ai_improve_writing`, `ai_suggest_hashtags`, `ai_summarize`
- **Automation rules**: `create_automation_rule`, `delete_automation_rule`, `get_automation_logs`, `list_automation_rules`, `update_automation_rule`
- **Campaigns**: `campaign_create`, `campaign_delete`, `campaign_list`, `campaign_update`
- **Cross-platform comments and DMs**: `comment_resolve`, `get_comments`, `get_dm_messages`, `list_dm_conversations`, `reply_to_comment`, `send_dm`
- **Discord webhooks**: `di_webhook_info`, `di_webhook_post`
- **`ias_*` family**: `ias_create_container`, `ias_get_comments`, `ias_get_media`, `ias_get_media_detail`, `ias_poll_container`, `ias_publish_container`, `ias_reply_to_comment`
- **Instagram messages**: `ig_get_messages`, `ig_list_conversations`, `ig_send_dm`
- **Unified inbox**: `inbox_list`, `inbox_reply`
- **Integrations**: `integrations_connect_complete`
- **LinkedIn messages and comments**: `li_get_messages`, `li_list_conversations`, `li_reply_comment`, `li_send_dm`
- **Bluesky and Mastodon replies**: `bs_reply`, `ms_reply`
- **Posts**: `posts_best_times`
- **RSS feeds**: `rss_feed_create`, `rss_feed_delete`, `rss_feed_items`, `rss_feed_list`, `rss_feed_poll`, `rss_feed_toggle`
- **Sets (templates)**: `set_create`, `set_delete`, `set_list`
- **Setup**: `setup_config_get`, `setup_config_list`
- **Signatures**: `signatures_create`, `signatures_delete`, `signatures_list`, `signatures_update`
- **Streak**: `streak_get`
- **Telegram**: `tb_list_chats`
- **Webhook delivery log**: `webhook_deliveries`
- **X messages and replies**: `x_get_dm_conversation`, `x_list_dms`, `x_reply_tweet`, `x_send_dm`
