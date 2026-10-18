# Execution record — 2026-10-19 (red-team follow-through)

Evidence for the register transitions made this cycle. Written after the
red-team pass in this session; every claim below was re-checked in the tree.

## Landed

| Item | Change | Where |
|---|---|---|
| C2 (splitter seam) | The duplicated `platform_limit` table is gone. `split_content` and `needs_splitting` take the provider's `max_content_length()`, which is the same value the validator enforces, so a split can't leave a segment the validator rejects. `preview_split` (no callers) removed. | `src/services/content_splitter.rs`, `src/services/staging.rs`, `src/cli/run.rs` |
| Splitter/validator bug | WhatsApp's splitter limit was 65536 while its validator limit is 4096. Now one source. | same |
| TikTok caption bug | `max_content_length()` returned the 210 MB video cap (220200960), which disabled caption validation. Now 2200. Regression test added. | `src/social/tiktok.rs` |
| Telegram `me` peer (E1 code side) | `resolve_chat("me")` resolves to the session's own account through `get_me`, so the `tu_*` roundtrip needs no public @username. Live run still needs credentials. | `src/services/telegram_client.rs`, `tests/live_telegram_test.rs` |
| Webhooks page crash | The list and deliveries endpoints return bare arrays; the page and typed client expected `{webhooks}` / `{deliveries}`. The page threw `reading 'length'` on every visit. | `frontend/src/lib/api/webhooks.ts`, `frontend/src/routes/settings/webhooks/+page.svelte` |
| Calendar crash | The URL-sync effect called SvelteKit's `replaceState` before the app root mounted, throwing `reading '$set'`. Switched to native `history.replaceState` with the router's own state. Pre-existing (commit `ff103ef`), not from the a11y pass. | `frontend/src/routes/calendar/+page.svelte` |
| Frontend a11y | svelte-check warnings 100 → 0, 0 errors. Labels associated, clickable divs made buttons, modal overlays closed by backdrop target, autofocus removed. Behaviour changes listed in the agent report: Escape now closes several modals; tooltip shows on keyboard focus. | 23 files under `frontend/src` |
| Skill: carousel | Carousel recipe added to SKILL.md (eval 43 needed it). | `skills/social-forge-agent/SKILL.md` |
| Skill: feed read | `x timeline --count 5` added to SKILL.md (eval 42 with-skill arm guessed `--limit`). | same |
| C3 eval loop (partial) | Paired with/without-skill runs on evals 41–43. Results: with-skill 2 pass, 1 partial; baseline 0 pass, 3 fail. | `skills/social-forge-agent/evals/c3-paired-run.json` |
| AGENTS.md doc map | The live-queue row now names `OUTLINE-2026-10-19-v2.md`; the 2026-10-17 outline is marked superseded; the quick-reference "601-line" note is corrected to its real role. | `AGENTS.md` |

## Gate

- `SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings`: exit 0.
- `SQLX_OFFLINE=true cargo test --lib`: 448 passed, 0 failed (before the TikTok test was added; full suite run pending).
- `pnpm build` (includes the hex-token gate): exit 0.
- `svelte-check`: 0 errors, 0 warnings.
- Browser smoke (`/login` then 23 routes × dark/light, 46 visits, Chromium headless): 0 page errors. The only remaining 404 is `/api/signatures/default` on the `/posts/new` redirect, which is the documented "no default signature" response.

## Cycle 2 (outline v3 execution)

| Item | Change | Where |
|---|---|---|
| A1 (typed client) | Audit of the 70 inline object generics in `frontend/src/lib/api/` against the handlers (read-only subagent). Ten mismatches fixed in the client types: comments resolve/reply (`{ok}`), automation create/update, webhooks create/update/delete/test, `setTags` (now `PostDetail`), reddit-cookie connect (partial object). Callers only read `r.error`, so none crashed; the types are now honest. | `frontend/src/lib/api/{comments,automation,webhooks,posts,integrations}.ts` |
| A3 / D-4 (Escape on four modals) | Escape and backdrop clicks go through a dismiss function. If the form is dirty, `modals.areYouSure` asks first. Dirty means: edited feed text, a typed campaign name, or a changed bulk-reschedule date/time/spread. The repurpose modal has no typed text and keeps plain Escape. Cancel buttons still discard. Kanban name input no longer has its own Escape handler, so Escape is not handled twice. | `frontend/src/routes/{feed,kanban,posts}/+page.svelte` |
| A4 (duplicate ids) | Static check: every id-bearing composer and connect component is rendered in exclusive or single-site branches. No change needed; to be confirmed in the smoke pass. | `frontend/src/lib/{composer,channels}` |
| D2 (disabled integration staging) | `stage_post` never read `integration.disabled`; publish did (`services/posts.rs:390`). Staging now rejects a disabled integration as a hard error that `force` does not override. Regression test added. | `src/services/staging.rs` |
| E1 (SKILL budget) | "Chaining Commands" section folded into "Finding IDs" as one jq example. SKILL.md 270 → 255 lines. Carousel, feed and toolset blocks untouched. | `skills/social-forge-agent/SKILL.md` |
| E3 (eval 42) | The eval's `expected_output` now names `--count 5` (not `--limit`) for the CLI read route. The MCP expectation is unchanged. | `skills/social-forge-agent/evals/evals.json` |

Gate for cycle 2:

- `SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings`: exit 0 (after the D2 change).
- `SQLX_OFFLINE=true cargo test --lib staging`: 5 passed, including the new D2 test.
- `svelte-check`: 0 errors, 0 warnings, 501 files.
- `pnpm build` (hex-token gate included): exit 0.
- Full `SQLX_OFFLINE=true cargo test --no-fail-fast` on the final tree (after the D2 change): exit 0, 21 targets `ok`, lib 450 passed. The run took about 90 minutes on a loaded box; many network-bound tests (bad-token LinkedIn/Threads/IAS, live Skool/WordPress/YouTube) took over 60 seconds each and still passed.
- Earlier full run (before D2) had `live_x_test::test_x_get_me` fail once with a `SendRequest` transport error and pass on the next run. Treated as the X transport flake, not a regression.

## Cycle 3 (goal loop: bugs and doc drift)

Landed in the working tree (uncommitted, D-2 pending):

- **Splitter zero-progress hang (`src/services/content_splitter.rs`):** a thread-capable provider subtracts 10 bytes of numbering overhead, so a limit of 10 left 0 content bytes. `find_break_point` returned 0 and `smart_split` looped forever. It now advances one whole character when nothing fits. Tests: `split_terminates_when_limit_is_below_one_character`, `thread_platform_with_tiny_limit_terminates`.
- **OAuth callback session JWT (`src/api/integrations.rs`):** the between-steps redirect carried a 30-day session JWT in the URL. The frontend never read it (`routes/auth/callback/+page.svelte`, `PagePicker.svelte`), so it is removed. The `jwt` import went with it.
- **Stale MIME test (`src/api/media.rs`):** `resolve_mime_rejects_undeclared_types_and_mismatched_declaration` expected an unrecognised-bytes upload to be stored under its declared type. That contradicted the AGENTS §6.2 rule (the declared type is never trusted alone), so the assertion now expects rejection.
- **Doc-to-CLI drift (README, `skills/social-forge-agent/`):** 20 documented commands failed to parse. Corrected to real shapes: carousel (`carousel "<text>" --integration <UUID> --media <urls>`), telegram-bot `<CHAT_ID>` positional, gcal and gmail-ops positionals, `linkedin-page feed`, `instagram posts`, `automation create` positional trigger and response type, `analytics get <PROVIDER>`. `comment delete` has no CLI action, so those lines were removed.
- **Doc guard (`tests/doc_cli_commands_test.rs`):** placeholders are filled before shell trimming, so `<ID>` is no longer cut as a redirect. `<…NUMBER…>` fills an integer. Lines still containing an unfilled `<template>` are skipped. Result: 3 of 3 pass.
- **Eval 43 (`evals/evals.json`, `evals/c3-paired-run.json`):** expected command corrected. Its C3 with-skill grade was "pass" for the invented `carousel create --images` shape, so it is now "fail" with the reason. The summary counts were recomputed.

Gate for cycle 3:

- `SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings`: exit 0.
- `SQLX_OFFLINE=true cargo test --lib`: 456 passed, 0 failed.
- `SQLX_OFFLINE=true cargo test --no-fail-fast` (before the JWT and MIME edits): every target `ok` except `--lib`, which had the one stale MIME test. Fixed; lib re-run green. `linkedin_e2e_test` (20) and `mcp_tools_test` (20) pass after the JWT edit.

Final gate for the AUD-6/7/9/10/11 pass:

- `SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings`: exit 0.
- `SQLX_OFFLINE=true cargo test --no-fail-fast`: 682 passed, 0 failed, 22 test targets, exit 0.
- `svelte-check`: 501 files, 0 errors, 0 warnings. `check-hex-tokens`: pass. `vite build`: pass.
- `contrast.py` on the extracted palette: 0 failing pairs. Anti-slop detector: 20 pass, 4 fail (the waivers above).
- Probe (authenticated, 8 routes × 3 widths): 0 FAIL, 165 advisory warnings.

Checked and closed without a change:

- `instance_url` `unwrap_or_default` (`integrations.rs`): no provider reads the stored `instance_url`. Mastodon takes its instance from server config. The empty default cannot break a publish.

Landed after the first pass (AUD-6, AUD-7, AUD-9, AUD-10, AUD-11):

- **Media alt text (AUD-6):** migration `0004_media_alt_text.sql` adds a nullable `alt_text` column. `create_media` stores it, and the MCP path upload passes `alt` through. The debug-only log block was removed. Test: `media_alt_text_round_trips`. Not yet exposed in `MediaPublic`; see the decisions list.
- **Scheduler (AUD-7):**
  - A missing provider now ends the post in `error` (guarded by `publishing`) and broadcasts `post_failed`. It no longer stays in `publishing`.
  - A panicked publish task becomes `error` with a message to check the platform before retrying.
  - Network errors are retried only when the connection never opened (`is_connect`). Any other network error fails with a duplicate-risk message. The request may have reached the platform.
  - Thread parts whose predecessor ended in `error` fail through `find_failed_thread_predecessor`, instead of requeuing forever.
  - Group reschedule reports real failures (`Group reschedule partly failed (N of M updated)`) and skips published members (`skipped_published` in the response).
  - A failed manual publish ends in `error`, not `queued`.
  - The next free slot is never in the past (`base = max(last, now)`).
  - Repeat copies are `queued`. A draft source stays `draft`.
  - RSS entries are claimed by `(feed_id, guid)`. An edited body is not re-imported. A failed create is compensated with `delete_rss_post`. The poller's HTTP client has a 30 s timeout.
  - Telegram transient send errors are retried up to 3 times at the head of the queue (`MAX_HEAD_RETRIES`). The rate guard no longer panics on `max_calls = 0`.
  - Webhooks retry 429, 5xx and network errors (3 attempts, 1 s then 2 s). The stored response body is capped at 4096 bytes.
  - Known ceiling, accepted: a reclaimed post after a shutdown abort is re-published without an idempotency check. `ponytail:` comment at `drain` in `src/scheduler/mod.rs`. Upgrade path: a provider idempotency key or a platform lookup before re-publishing.
  - Tests: `failed_predecessor_reason_is_visible_to_the_gate`, `rss_entry_is_claimed_once_by_guid_even_if_its_body_changes`, `next_free_slot_fallback_is_never_in_the_past`, `transient_head_failure_is_retried_not_dropped`, `zero_max_calls_defers_without_panicking`, `transient_send_errors_are_classified`, `response_body_is_capped_at_the_stored_limit`.
- **Docs drift (AUD-10):** README tool count 327 → 367 (the `#[tool(` count under `src/mcp/`) and CLI commands 55. AGENTS §7 reclaim is `> 15min (RECLAIM_AFTER_SECS = 900)`. AGENTS §9.8 lists the 19 realtime events. `tests/surface_docs_audit.rs` now checks the README tool and command counts against the source (`d1_documented_cli_command_count_matches_clap`).
- **Unwraps (AUD-11):** the rate-limit panic on an empty bucket is fixed. The infallible sites in `mastodon.rs`, `x.rs`, `short_link.rs`, `api/mod.rs` and `api/media.rs` are `expect`s with a reason. A scan of `src/` finds no bare `.unwrap()` before any file's `#[cfg(test)]` module. The remaining flagged sites are `expect`s with a reason.
- **Website-design gates (AUD-9):**
  - Contrast: `contrast.py` reports 0 failing pairs when run on an extracted copy of the palette. The checker expects `:root` and `[data-theme="light"]` selectors, and `app.css` uses `:root, :root.dark` and `:root.light`. This is a checker limitation, not an app defect.
  - Solid accent buttons (`bg-accent-fill`) inherited body text, measured at 3.8 to 3.9:1. They now set `text-accent-fg` (white on brand fill, 4.5:1 or better).
  - The provider letter tile on a brand fill picks white or black by contrast (white fails on Google blue, 3.6:1). The tile on the neutral fill uses the content colour.
  - Every route title is an `h1` (it was an `h2`). Section headings and `EmptyState` / `GettingStarted` titles moved one level up, so the outline has no skips.
  - `MediaCarousel` lightbox drops `backdrop-blur-xl`. The `RichTextEditor` surface has a focus ring from `:focus-visible` inside it.
  - Probe (`scripts/probe.mjs`, copy with `ignoreHTTPSErrors` and `waitUntil: 'load'` and a session cookie) at 1440, 768 and 375 over `/`, `/calendar`, `/posts`, `/channels`, `/analytics`, `/settings`, `/media`, `/tags`: 0 FAIL, 165 warnings. The warnings are advisory: 10 px bottom-nav labels, tap targets under 44 px on touch, and no `.btn-primary` or `[data-cta=primary]` element. The run used an empty database, so the empty states were measured.
  - Anti-slop detector (`detect.sh` patched copy): 20 pass, 4 fail. Waivers:
    - Check 05 (`ai-purple-glow`): the purple is the platform-preview colour in `GeneralPreview.svelte`, allowlisted as a platform preview. The tags palette is user-selectable.
    - Check 08 (`eyebrow-spam`): the uppercase labels are dashboard labels, not marketing eyebrows.
    - Check 20 (`focus-removed`): the remaining `outline-none` sites have a visible replacement focus or the global `app.css` `:focus-visible` ring for input, textarea and select. `PostDetail.svelte` is `tabindex=-1`.
    - Check 21 (`raw-hex`): raw hex appears only in composer platform previews and mention highlights. `frontend/scripts/check-hex-tokens.mjs` allowlists these and passes.
  - Gates: `svelte-check` 0 errors, 0 warnings. `check-hex-tokens` passes. `vite build` passes.
- **Migration race (`src/db/mod.rs`, `create_pool`):** two processes opening one SQLite file both saw migration 4 pending. The second `ALTER TABLE media ADD COLUMN alt_text` failed with "duplicate column name". sqlx-sqlite 0.8.6's migration `lock()` is a no-op, so the migrate step now holds an exclusive `File::lock` on `<db>.migrate.lock`. In-memory databases skip it. Caught by `tests/linkedin_e2e_test.rs` (3 failures).
- **Stale test expectation (`src/services/posts.rs`):** `publish_rejects_a_success_that_carries_no_platform_post_id` asserted the row stays `queued`. It now asserts `error` with the reason. The AUD-7 change made that the intended outcome, and `error` is editable, so the post can be rescheduled.
- **Test database (AUD-17):** `load_dotenv_for_tests()` (`src/config.rs`) points the isolated test binaries at `target/test-data/social-forge-test.db`, so they no longer migrate the operator's `data/social-forge.db`. The `data/` file's mtime is unchanged by an isolated run. `load_dotenv_for_live_tests()` keeps the `.env` `DATABASE_URL` for the `live_*` tests and `x_live_test`, because those read OAuth tokens stored in the operator database; isolating them would skip them silently. Decision: see the decisions list.

- **Postgres-only SQL on SQLite (AUD-13):** `NOW()` and `make_interval(...)` remained in `src/mcp/tools_admin.rs` (signature update, analytics summary, feed analytics) and `src/api/integrations.rs` (posting times, enable/disable). `sqlite3` reports `no such function: now` for them, so each call failed at runtime. They now use `unixepoch()` and `? * 86400`. The enable/disable write moved to `queries::set_integration_disabled`, which the REST handler and the new CLI share. Test: `admin_analytics_and_integration_toggle_run_on_sqlite` (`tests/mcp_parity_test.rs`).
- **Posting times read as a string (AUD-14):** `posting_times` is stored as TEXT JSON. sqlx decodes a `serde_json::Value` column as that raw text, so `/api/integrations` returned `"posting_times":"[{\"time\":120},...]"`. The channels page then threw `fe.map is not a function`, and the slot finder's `as_array()` returned `None`, so configured posting times were ignored. `models::decode_posting_times` is the single decode step, used by `IntegrationPublic` and the slot finder. Tests: `stored_posting_times_text_decodes_to_an_array`. Found by the seeded-data probe, not by reading the source.
- **Contrast on tinted status (AUD-15):**
  - `Badge` and the global `.badge-*` classes put the status hue on a 15–20% tint. That measured 3.9–4.3:1 at 10–12 px, and the token's own 4.5:1 is only on white. The text is now the content colour, and the hue moves to a 1 px inset ring.
  - Dashboard channel bars put white counts on provider fills at 2.1–2.5:1. `inkFor` (`frontend/src/lib/ui/ink.ts`) picks white or black, whichever clears more contrast. The better of the two is always at least 4.58:1. `ProviderIcon` uses the same helper.
- **Parity tools (AUD-8):** 22 MCP tools for the REST-only surfaces (campaigns, sets, RSS, AI, streak, comment resolve, webhook deliveries), taking the MCP count to 389. CLI `posts update`, `posts tags` and `posts validate` are added. `integrations enable|disable` is added; `providers` already lists integrations, so no separate list command. Developer keys have no MCP tools (decision).
- **Public endpoints (AUD-12):** `/api/metrics` and `/api/streak` moved behind the session. AGENTS §public-endpoints and the router comment are updated (decision: auth required).
- **FRONTEND_URL and port (AUD-16):** README and AGENTS say that `APP_URL` and `FRONTEND_URL` must use the port `serve` listens on. `serve --help` says the same. A mismatch makes `POST /api/feed/import` return 403 `csrf_origin_mismatch`.
- **Seeded-data browser pass (A2, B1, B2, B4, B5 for AUD-9 follow-up):** the scratch DB was seeded with 2 integrations, 4 posts (draft, queued, published, error), and 2 external posts with engagement. UUIDs were written as blobs, which is how sqlx stores them. Results:
  - Probe, light, 8 routes × 1440/768/375 (`probe-out/run12`): 0 FAIL, 208 advisory warnings, no page errors, no HTTP 5xx.
  - Probe, dark, 8 routes × 1440/375 (`probe-out/run13-dark`): 0 FAIL, 139 warnings, no page errors.
  - B2, browser: at 300 of 280 weighted characters for X, `Post Now` and `Schedule` are disabled and the message reads "X - Weighted count is 300/280 - 20 over". Under the limit both are enabled. No page errors.
  - A2: cold `/calendar` loads with zero page errors in both themes (probe `pageerror` checks).
  - A5: `Tooltip.svelte` has no call sites, so the focus behaviour is not observable in the app. The class is `group-focus-within`, read from source. Not verified in a browser.
  - B3 (drag): run after the calendar fix. Week-view drag persists (see the calendar bullets above). Kanban drag persists (`kanban_stage_test`).
  - B4: contrast is sampled by the probe in both themes, with 0 FAIL. This is not an exhaustive pair audit.
  - Probe change, scratch copy only: text inside `.truncate` is skipped by the overlap check. Clipped text keeps its full range box under the next column, while its ink is clipped. The 375 screenshot showed no overlap.

- **Gate (this pass):** `SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings`: exit 0. `SQLX_OFFLINE=true cargo test --no-fail-fast --locked`: 687 passed, 0 failed, exit 0. `live_x_test::test_x_get_me` failed once on a network send error and passed on re-run; it is a live test. `svelte-check`: 502 files, 0 errors, 0 warnings. `vite build`: pass. `surface_docs_audit` and `doc_cli_commands_test`: pass.

- **Calendar range query (AUD-18):** `get_calendar_posts_with_metrics` bound `DateTime` (TEXT) against INTEGER epoch columns, and SQLite sorts TEXT after INTEGER, so no date range ever matched. Its `OR` branch was also unparenthesised, so the published branch escaped the `user_id` and `deleted_at` filters. Fixed with `EpochUtc` binds and parentheses. The same bind class was fixed in `get_posts_by_date_range` (MCP calendar tool and `services/posts.rs`), `record_publish_attempt`, `set_post_recurring` and `set_post_recurring_with_copies`, and the analytics-cache lookup. Regression: `tests/calendar_range_test.rs`, 5 of 5. Mutation checks: reverting the bind fails the two in-range tests; removing the parentheses fails the other-user leak test.
- **Kanban `update_stage` bind order (AUD-18):** four `UPDATE posts` branches bound values in the wrong order for SQLite's positional `?`. The WHERE matched no row and the handler still answered `updated: true`. Binds now follow placeholder order. Regression: `tests/kanban_stage_test.rs`, 3 of 3.
- **Calendar refetch loop (AUD-19):** the list-view and filter `$effect`s in `routes/calendar/+page.svelte` called `refresh()`, which reads and writes `refreshing` state. The effect re-ran on its own write, issuing about 10 `/api/calendar` requests in 6 seconds. Both calls are now wrapped in `untrack`. Two requests in 15 seconds afterwards.
- **Week-view chip key (AUD-19):** `WeekView.svelte` bucketed events by `HH`, while cells look up `HH:00`, so no chip rendered in the week grid. Events now bucket by `HH:00`. Browser drag of the queued post from Tue 20 Oct to Thu 22 Oct 09:00 (IST): `scheduled_at` went from `2026-10-20 11:07:13` to `2026-10-22 03:30:00` UTC, with no page errors and no API errors.
- **Dead `Tooltip` removed (BR-1, A5):** `frontend/src/lib/ui/Tooltip.svelte` had no importers. It is deleted. The uncommitted copy is kept in the scratchpad. A5 is closed as not applicable.
- **Skill references:** `skills/social-forge-agent/references/quick-reference.md` gains a tool catalogue for the 65 MCP tools that the contracts did not name. Names come from the `#[tool]` fn names. `doc_cli_commands_test` passes 3 of 3.
- **Gate (after the calendar and week-view fixes):** `SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings`: exit 0. `SQLX_OFFLINE=true cargo test --no-fail-fast --locked`: 694 passed, 1 failed. The failure is `live_x_test::test_x_get_me`, a network send error. It is the same known flake as before. `doc_cli_commands_test` passes 3 of 3 after the catalogue edit. `svelte-check`: 501 files, 0 errors, 0 warnings. `vite build`: pass. Seeded probe, 6 routes × 1440/768/375, light and dark (`probe-out/run14`, `run15-dark`): 0 FAIL.
- **API key expiry (AUD-20, security):** `verify_api_key` accepts a key while `expires_at IS NULL OR expires_at > unixepoch()`. The create handler bound `expires_at` as a DateTime, which SQLite stores as TEXT, and TEXT sorts after every INTEGER. A key created with a 2020 expiry still verified. Reproduced first (`tests/api_key_expiry_test.rs`, past-expiry test failed), then fixed by binding `EpochUtc`. Migration `0005_epoch_api_key_and_plug_times.sql` converts rows already stored as TEXT. Dry-run on a scratch copy: a legacy 2020 key went from valid (`expires_at > unixepoch()` = 1) to expired (0).
- **Plug runner binds (AUD-21):** `services/plugs.rs` wrote `next_run_at` and `fired_at` as TEXT. The due check is `next_run_at <= unixepoch()`, so a plug was never due again. Binds are now `EpochUtc`, and migration 0005 converts stored rows. No test: the runner loop has no seam.
- **Sweep for raw DateTime binds:** every `.bind(x)` in `src/` where `x` is declared as a DateTime was checked against its column type. Analytics windows bind `i64`. Billing periods bind `i64`. Scheduler `now` binds are `i64`. Still raw, and intentionally consistent as TEXT: `token_expires_at` (written and compared as RFC 3339 by the refresh query) and `external_posts.created_at` plus its cursors (written and compared as TEXT). Converting either needs the write and the compare changed together. Not changed this pass.
- **Unwrap and expect outside tests:** 17 sites. All are infallible (static regex, static header, HMAC with any key length) or client construction at startup. None is reachable from request input. Accepted, not churned.
- **Frontend effect audit:** the `$effect` blocks in `routes/analytics`, `CommentsThread` and `kanban` read only their intended inputs before their first `await`. No other refetch loop found in the sampled blocks. Not exhaustive: 58 effect sites across the frontend.
- **D-1 dry run (no lockfile change):** `cargo update -p glass_pumpkin --dry-run --locked` downgrades `glass_pumpkin` 1.10.0 → 1.9.1 and changes nothing else (228 other packages unchanged). 1.10.0 is yanked, and this matches the plan in `VENDOR_PINS.md`.
- **Gate (after the API key and plug fixes):** `cargo clippy --all-targets --locked -- -D warnings`: exit 0. `cargo test --no-fail-fast --locked`: 698 passed, 0 failed.
- **Behaviour changes to review:** failed manual publish now ends in `error` (was `queued`). Group reschedule with `action=schedule` skips published members. Network errors after the request was sent are no longer retried. Repeat copies are `queued` unless the source is a draft. RSS entries are deduplicated by guid. Webhooks retry 429, 5xx and network errors, and store at most 4 KiB of response.

## Cycle 4 (public-route security pass)

- **AUD-23 (done):** `/api/proxy-media` moved from `public_routes` into `protected_routes` (`src/api/mod.rs`). An anonymous request reached the upstream fetch: 500 before the fix, 401 after. `tests/proxy_media_auth_test.rs` fails on the old route table and passes on the new one. AGENTS §6.4 and the auth list at line 444 now name the route as gated. The frontend's only caller (`frontend/src/lib/api/feed.ts:72`) is same-origin and carries the cookie.
- **AUD-24 (done):** `src/api/media.rs` checks each redirect hop against the media allowlist. wreq takes the policy per request. reqwest 0.12 has no per-request policy, so the proxy uses a dedicated client (`PROXY_HTTP_CLIENT`) whose policy is set at build time and fails closed. The shared media clients (Discord attachments in `tools_discord.rs`) are unchanged. Mutation check: with the policy replaced by `follow`, both redirect tests fail.
- **AUD-22 (done, decision made):** removed the plug runner (`src/services/plugs.rs`, its `pub mod` line, and the spawn in `src/main.rs`). The user chose "remove the runner, keep the table": `post_plugs` stays in place, unused, so the migration rule holds. The live automation feature is `automation_rules`.
- **Gate:** clippy `--all-targets --all-features --locked -D warnings` exit 0; `cargo test --all-targets` 702 passed, 0 failed.

## Cycle 5 (F1, W006, lint)

- **F1 (rejected as written):** the outline's fix routes all 46 non-test `Provider::new(` sites through `state.providers`. The cookie-session providers cannot share one instance: `XProvider::prepare_from_token(&mut self)` writes the account's credentials into `self` (`src/social/x.rs:155-163`), so concurrent calls for two accounts would race on one credential slot. Per-call construction stays. The cost the outline cited is already gone: the shared pools (`shared_http`, `shared_wreq`, `x_http`) carry the TLS setup.
- **W006 (done):** AGENTS §0.6 names `OUTLINE-2026-10-19-v2.md` as the live queue. `OUTLINE-2026-10-19.md`, `OUTLINE-2026-10-19-v3.md` and `OUTLINE-2026-10-17.md` were archived with `kb archive` (tombstones at the original paths, rows in `docs/archive/INDEX.md`). The two superseded history files `GAPS_2026-10-15.md` and `OUTLINE-2026-10-16.md` were archived the same way. `AUDIT_2026-10-14.md` moved to `docs/audit/` (the AGENTS routing table's home for audits). Routing and link references point at the new paths.
- **Lint:** `kb lint` reports 0 errors. The `providers.md` E001 was a bare filename in a description cell; the full path is now written there.

## Decisions for the user

Defaults applied in this pass. Each can be reversed without a schema change.

1. **Live tests keep the operator database.** `live_*` tests and `x_live_test` read OAuth tokens from `DATABASE_URL`. A separate scratch store would skip them silently.
2. **`/api/metrics` and `/api/streak` require the session (AUD-12).** They were public. The default treats post counts and streak as account data.
3. **Developer keys have no MCP tools (AUD-8).** Creating or revoking keys stays on REST and the web UI. Adding them to MCP would let an agent mint credentials.
4. **CLI parity is posts update/tags/validate and integrations enable/disable (AUD-8).** Campaigns, sets, RSS, AI and webhook deliveries reach agents through MCP. `providers` is the integration list.
5. **Status badges use content-colour text with a hue ring (AUD-15).** This is a visual change: the status hue no longer colours the label.
6. **`alt_text` is not in `MediaPublic` yet (AUD-6).** It is stored and returned by the upload path. Exposing it to the read model is a separate call.

## Not landed (honest)

- **F1 (shared provider construction):** 46 `*Provider::new(` call sites remain across `src/mcp`, `src/api`, `src/cli`. The shared HTTP pools exist (`shared_http`, `shared_wreq`), but providers are still built per call. Registry-only construction is not enforced.
- **C3:** 3 of 43 evals run paired. The other 40 are not run.
- **E1 live run:** needs `TELEGRAM_TEST_PEER=me` (or a real @username) plus a session. Not run. The completed run the user reported has no output on record.
- **E2 / E3:** need `DISCORD_BOT_TOKEN` and `WHATSAPP_CLOUD_*` credentials. Not run.
- **D1 (glass_pumpkin pin):** not applied. Needs the D-1 go-ahead, since it changes `Cargo.lock`.
- **D-2 commit scope:** nothing committed. Needs the D-2 go-ahead.
- **A5 (tooltip on keyboard focus):** closed by removal. `Tooltip` had no call sites, so the component was deleted rather than checked.
- **B3 (calendar and kanban drag):** run. Both drags persist to the server (see Cycle 3).
- **B1 and B4 depth:** the seeded probe and screenshots cover the routes at three widths. Analytics engagement copy and kanban cards were not inspected one by one.
- **Skill budget:** SKILL.md is 255 lines, which meets the outline's ≤255 target.
- **Visual check on seeded data:** done for the probe routes (see Cycle 3). The first-run onboarding overlay is dismissed by its own Skip button in the browser check.

## Note on the smoke environment

The smoke server ran with a scratch database under the session scratchpad, `APP_URL` and `FRONTEND_URL` set to `https://localhost:8799`, and a throwaway `APP_PASSWORD`. Nothing in the user's real `data/` directory was touched.
