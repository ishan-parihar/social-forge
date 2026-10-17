# TODO — the register

One row per work item; rows update in place, verdicts append — never delete.
`scripts/kb status <id> <state> --evidence <path>` is the supervised transition
(terminal states demand evidence; `scripts/kb claim-register` is the
flock-guarded claim, appended to the notes cell).

Seeded 2026-10-17 from `docs/planning/OUTLINE-2026-10-17.md` — the frozen
plan. Every row's "where" was verified against the tree at `ca63a9e` by the
outline itself; the outline stays as the plan-of-record, this register carries
live status.

| id | status | item / verdict | notes |
|---|---|---|---|
| V1a | open | `live_reddit_test` runs non-skipped: import a working cookie blob or OAuth token; 5 tests green; anything surfaced goes through the audit-fix pattern | creds, not code; S–M; order: V1a first (cheapest creds) |
| V1b | open | `live_telegram_test` roundtrip non-skipped: complete `tu_request_code`/`tu_sign_in` once | MUST stay a single sequential test (gramers sessions are not concurrent-client safe); S |
| V1c | open | `live_discord_test` non-skipped: `DISCORD_BOT_TOKEN` + test guild; 3 tests green, chronological order asserted | S |
| V1d | open | `live_whatsapp_cloud_test` full: `WHATSAPP_CLOUD_*` sandbox creds; template round-trip + `wc_send_template` + webhook GET verification | M — needs the Meta developer-app dance |
| C1 | open | one resumable-upload implementation: shared `youtube_upload::resumable_upload(...)`; `google.rs:286–326` and `youtube.rs:1023–1073` call it | S–M; shrink google.rs |
| C2 | open | seam debt: splitter takes `&dyn SocialProvider` (limits live on providers); `db::types::EpochUtc` newtype wraps the two cutoff binds | S; when touching either file for other reasons |
| R1 | open | Telegram write-queue loss window: document the ceiling in `tu_queue_send` + AGENTS.md (S), or persist a `telegram_queue` table (M) — persist only if V1b shows real payloads | decision point, not a bug |
| P6.1 | rejected | LinkedIn/Facebook/Instagram cookie ports → [2026-10-17] `docs/planning/OUTLINE-2026-10-17.md` standing notes — verdict: REJECTED on evidence | re-evaluate only if a scraping provider is ever shown to work where the API path cannot |
| P6.2 | rejected | (same evidence as P6.1) → [2026-10-17] `docs/planning/OUTLINE-2026-10-17.md` standing notes | same re-open condition |
| P6.3 | rejected | (same evidence as P6.1) → [2026-10-17] `docs/planning/OUTLINE-2026-10-17.md` standing notes | same re-open condition |
| B12 | blocked | Linux Chromium cookie decrypt — deliberately not implemented; upgrade path documented at `browser_cookies.rs:64` (`ponytail:` note) | unblock = evidence the Firefox/Zen plaintext import + paste paths are insufficient |
| WA-inbox | blocked | WhatsApp Cloud inbound webhook messages parsed + trace-logged only | unblock = the first consumer that needs an inbox table (`api/whatsapp_webhook.rs:4`) |
| MCP-identity | rejected | `/mcp` streamable-HTTP advertises `serverInfo: rmcp` (identity macro-owned by `#[tool_handler]`) → [2026-10-17] `docs/planning/OUTLINE-2026-10-17.md` standing notes | not worth overriding while the macro owns capabilities advertisement |
