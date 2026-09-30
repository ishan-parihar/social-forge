# Social Forge — Single-User Depth Parity Plan (v25)

> Date: 2026-09-30
> Supersedes: v22 audit, v24 upgrade guide (background reading, not re-litigated here)
> Non-negotiables (AGENTS.md §0.5): single-user `APP_PASSWORD`, single Rust binary, no Redis/Temporal/microservices, triple interface (CLI+REST+MCP), SSE realtime, AES-GCM token encryption.
> User direction (2026-09-30): **depth over breadth** — cover the most popular platforms deeply; do not carry the full Postiz battery. Frontend visual/UX quality is a first-class parity gap.

## 0. Decision log

| Decision | Rationale |
|---|---|
| Single-user only — no orgs/teams UI, no marketplace/orders/payouts, no multi-tenant RBAC | AGENTS.md + user confirm; teams tables stay as dormant FK scaffolding, never surfaced in UI |
| Tier-1 = 12 platforms for depth investment | Maintenance budget of one maintainer; Postiz carries ~36 providers with a team — we cannot match breadth, so we win on depth + reliability for the platforms a solo founder actually uses |
| Tier-2 = publish-maintained, no new depth | Keeps long-tail users unblocked without spending depth budget |
| Tier-3 = archive behind flag, remove from default registry/UI | Cuts provider surface, MCP tool sprawl, OAuth callback complexity, and test matrix |
| Frontend gets its own phase track, not "polish at the end" | User reports UI as the most visible parity gap; design tokens + layout shell land before feature work so later pages inherit the system |

## 1. Platform tiers (final)

### Tier-1 — depth (12): the solo-founder core
`x, linkedin, linkedin-page, facebook, instagram, instagram-standalone, threads, youtube, tiktok, reddit, bluesky, pinterest`

Why these: top-10 global MAU for creators + LinkedIn Page (agency/B2B is the paying use-case) + Instagram-standalone (Meta API without Facebook Page dependency). Reddit + Pinterest included because Forge already has dual-path cookie auth (X, Reddit) and visual discovery — both high-ROI for a solo operator.

### Tier-2 — publish-maintained (14): keep working, no new analytics/DM depth
`telegram-bot, telegram-user, discord, slack, whatsapp, wordpress, mastodon, youtube already T1 — plus medium, devto, hashnode, github, google (gmail/calendar/drive as utility, not social reach)`

Concretely: Tier-2 keeps `publish + validate_post/media + refresh_token + error mapping`. No new analytics/engagement/comments/DM work unless a bug breaks publishing.

Note: blogging/dev providers (medium/devto/hashnode/wordpress/github) are API-key-simple and cheap to keep; they stay Tier-2 as a group. GMB/calendar/drive/skool are user-mandated keeps — publish-maintained only.

### Tier-3 — archive (remove from default registry + UI + MCP, keep code under `src/social/archive/` behind `ENABLE_ARCHIVE_PROVIDERS=1`)
`farcaster, tiktok-business merge into tiktok, mastodon-custom merge into mastodon via instance_url`

Per user direction (2026-09-30): `google_my_business, calendar, drive, skool` are explicitly KEPT — they move to Tier-2 (publish-maintained, zero depth spend). `kick, vk, whop, lemmy` are fully REMOVED (not archived): delete provider file + registry entry + MCP module + CLI wiring + frontend entry in the same phase, no env-flag fallback.

Also formally **won't-port** from Postiz: `dribbble, tumblr, twitch, mewe, moltbook, nostr, listmonk`. Reason documented in code: niche MAU + distinct media pipelines (Twitch clips, Tumblr reblogs) that would each cost a full provider cycle.

Execution effect:
- `registry.rs`: Tier-3 gated behind env flag, absent from `/api/providers` by default.
- MCP: Tier-3 tool modules excluded from default tool registry (cuts ~40 tools of sprawl).
- Frontend `providers.ts` + `ChannelCard`: Tier-3 hidden unless flag set.
- Docs + README platform table rewritten to Tier-1/Tier-2 only.

## 2. Depth definition (what "done" means for Tier-1)

Every Tier-1 provider must satisfy all 10 rows. Current state from grep audit (2026-09-30):

| Capability | Current Tier-1 coverage | Gap to close |
|---|---|---|
| 1. `publish` + `validate_post` + `validate_media` | ✅ all override | Verify per-platform limits match 2026 API docs (X 4 img/1 vid, IG 10, LinkedIn 20, YT video-required, TikTok sound/music picker parity) |
| 2. Threads/replies (`in_reply_to`, group publish) | ⚠️ trait supports; X/Bluesky/Mastodon/Threads link — verify TikTok/YT/IG short-circuit cleanly | Add per-provider thread test |
| 3. `analytics` (dashboard range) | Only `fb, ig, linkedin(+page), reddit, threads, x` | **Add: youtube, tiktok, bluesky, pinterest, instagram-standalone** |
| 4. `get_post_engagement` + `fetch_engagement` normalize | Only `+bluesky, youtube` beyond analytics set | **Add: tiktok, pinterest, instagram-standalone**; parsers already in `parse_engagement_data` for x/reddit/bluesky/ig/linkedin/fb/yt/mastodon/tiktok/threads — extend to pinterest + standalone |
| 5. `get_post_comments` + `comment`/`reply_to_comment` | reddit/fb override; X/LI/IG/Threads/YT missing | **Add read path for all Tier-1** (even if reply unsupported, list must work for Comments tab); write path where API allows |
| 6. Mentions `search_mention` + `format_mention` | Default empty | **Add for x, linkedin, facebook, instagram, threads, youtube, reddit, bluesky** |
| 7. `targets`/`pages` discovery | Partial | **Require: linkedin-page + facebook pages, reddit subreddits, youtube channels, pinterest boards** — powers TargetPicker |
| 8. Token refresh + `needs_cron_refresh` + scheduler integration | ✅ scheduler refresh exists | Audit each Tier-1: correct `refresh_token()` + `needs_cron_refresh=true` where expiring (Meta, Google/YT, TikTok) |
| 9. `map_error` user-friendly messages | Spotty (X has rich mapping) | Port X-style mapping to all Tier-1 (rate-limit, duplicate, media-rejected, auth-expired) |
| 10. `get_recent_posts` (feed import) | Default empty | **Add for x, youtube, reddit, bluesky** — feeds external-post import CLI |

Out of scope for depth (single-user filter): team approval queues, marketplace attribution, multi-org token isolation.

## 3. Backend depth plan (phases B1–B4)

- **B1 — Registry tiers + tool gating (1 unit):** `registry.rs` tier flags, `ENABLE_ARCHIVE_PROVIDERS`, MCP registry respects tiers, `/api/providers` returns `tier` field. Archive Tier-3 files (git mv, keep history). Verification: `cargo check`, providers list shows 12+8 by default.
- **B2 — Tier-1 analytics + engagement (3 units, parallel by provider group):** B2a Meta group (fb/ig/standalone/threads), B2b Google/YT + TikTok + Pinterest, B2c X/LinkedIn-page/Reddit/Bluesky hardening. Each: `analytics()`, `get_post_engagement()`, parser arm, `post_analytics()` where API supports. Verification: live-token test or recorded-fixture test per provider + `analytics_cache` rows populated.
- **B3 — Comments/mentions/targets (2 units):** read-comments for all Tier-1, reply where allowed, mentions search, targets/pages. Wires Comments tab + TargetPicker + @-autocomplete. Verification: `cargo test` provider-methods + frontend manual check.
- **B4 — Refresh + error mapping + recent-posts (1 unit):** refresh flags, X-style `map_error` for all Tier-1, `get_recent_posts` for x/yt/reddit/bluesky. Verification: scheduler refresh dry-run, error snapshot tests.

Media store decision (needed before B2 analytics screenshots): keep local-disk for v25, but raise per-route limit for video (`/api/media` 100MB) while keeping global 10MB; file R2/S3 follow-up as v26 (explicit, not silent).

## 4. Frontend / web-design parity plan (phases F1–F5)

Problem statement: Forge has the pages (composer, calendar Month/Week/Day, channels, analytics, media, comments, dms, feed, kanban, campaigns, settings) but visual execution lags Postiz (Next.js + Mantine + CopilotKit polish). v22 found **80 hardcoded hex colors across 22 files**, broken light mode, dual settings sidebars.

- **F1 — Design tokens + shell (first, blocks rest):** semantic CSS tokens (bg/surface/border/text/muted/accent × light/dark), replace hardcoded hex (codemod + manual for `CommentsThread`, `MediaCarousel`, `RichTextEditor`, `Button`), single settings sidebar (remove duplicate), top-bar streak/notifications unify, empty/loading/error skeletons per route. Reference: `design-taste-frontend` audit-first, `ui-ux-pro-max` palette/UX guidelines. Verification: `pnpm build`, light/dark screenshot pair, no hex outside tokens (grep gate).
- **F2 — Composer (highest leverage):** Postiz parity items already partially built (ChannelSelector, PerPlatformCharCount, PlatformPreviewPane, SchedulePicker, MediaUpload with progress, FirstComment, TargetPicker, TagPicker, SignatureEditor, MusicPicker, AiAssistant) — work is **visual density + correctness**: live per-platform previews (X/IG/LI/TikTok/YT frame-accurate), char-ring with over-limit blocking, schedule timezone picker, draft autosave indicator, AI generator stream state, mobile stacking. Verification: composer matrix checklist (12 Tier-1 previews render, validation toasts show all errors).
- **F3 — Calendar + Kanban:** Month/Week/Day polish (today column, drag affordance, tag-color chips, error `!` badge kept), "Just update vs Reschedule" modal kept, kanban swimlane polish (WIP, due/priority visible, cover images, within-column reorder). Verification: drag e2e happy-path.
- **F4 — Channels + Analytics + Media:** ChannelCard status clarity (connected/expiring/disabled/refresh-needed), connect-flow OAuth + cookie + API-key + PAT + Web3 unified, analytics dashboard engagement-first (not just counts: engagement totals, rate/channel, cadence-vs-goal, scheduled-today, recent activity), media grid keyboard/a11y. Verification: analytics with seeded data renders non-empty.
- **F5 — A11y + responsive + motion:** focus rings, aria for modals/pickers, reduced-motion path, 360px mobile pass on composer/calendar/channels, SSE-driven updates (no polling regressions). Verification: `pnpm build` + manual keyboard/mobile pass.

Design constraints: SvelteKit + Svelte 5 runes stay; no UI framework migration; SSE stays (no SWR); keep `modals` rune manager; dark-first but light must work.

## 5. What we explicitly drop (and why)

| Dropped | Why |
|---|---|
| Team collab, roles, approval queues, org switching | AGENTS.md §0.5 + user; Postiz team features do not transfer |
| Marketplace (agencies, credits, orders, payouts, customers) | Solo-founder command center, not SaaS |
| Announcements/admin-stats, Clipping/HeyGen/ReelFarm, Deepgram widgets, emails/newsletter | No single-user evidence; revisit only on explicit request |
| Tier-3 providers + wont-port list (§1) | Maintenance budget; each provider is OAuth drift + test matrix cost |
| Temporal/Redis/microservices, SWR polling | Single-binary + SSE identity |

## 6. Execution order + gates

```
B0 vendor pins → F1 tokens+shell → B1 tiers (incl. kick/vk/whop/lemmy deletion) → B2 analytics/engagement → F2 composer → B3 comments/mentions/targets → F3 calendar/kanban → B4 refresh/errors/recent → F4 channels/analytics/media → F5 a11y/responsive
```

- Each phase: implement → `cargo check --lib --bin social-forge` → `cargo test --lib` (if Rust touched) → `cd frontend && pnpm build` → commit + push per AGENTS.md golden rule.
- No phase starts if prior gate is red.
- MCP/CLI parity: any new REST capability gets MCP tool + CLI wiring in the same phase (triple-interface rule).

## 7. Acceptance (v25 done when)

- [ ] Default providers list = 12 Tier-1 + 14 Tier-2 (incl. gmb/calendar/drive/skool); kick/vk/whop/lemmy fully deleted; Tier-3 (farcaster/archive) hidden without flag; README table matches runtime.
- [ ] All Tier-1 satisfy §2 rows 3–7 (analytics, engagement, comments-read, mentions, targets) — verified by fixture/live test per provider.
- [ ] Composer renders 12 Tier-1 previews, blocks over-limit, schedules with timezone, shows all validation errors.
- [ ] Analytics dashboard shows engagement (not just counts) with seeded data; calendar/kanban drag works; light mode has no hardcoded-hex leaks.
- [ ] `cargo check` 0 errors, `pnpm build` succeeds, new tests pass.

## 8. Product alignment: Postiz-human vs Forge-agent UX

Per user direction (2026-09-30): Postiz is the human-facing reference; Social Forge is the AI-agentic surface. This changes how parity is judged:

- Human UI (Postiz parity) covers: composer readability, calendar legibility, channel connect flow, analytics glanceability, settings discoverability (phases F1–F5). No human team workflows are ported (AGENTS.md §0.5).
- Agent surface (Forge lead, must not regress): every REST capability added in B-phases ships with MCP tool + CLI wiring in the same phase (triple-interface rule); MCP tool descriptions carry Tier-1 limits (char caps, media caps, thread support) so agents plan correctly; `integrations targets`, mentions search, and validation endpoints stay agent-callable with JSON-first errors; SSE event names remain stable for agent watchers.
- Acceptance for agent UX: `mcp_meta_audit` + `provider_methods_test` pass; new Tier-1 depth (analytics/engagement/comments/mentions/targets) is reachable via MCP with the same filters as REST.

## 9. Vendor freshness: build-time pull for ported sources

Ported/vendored code (wa-rs WhatsApp stack, gog-cli Google patterns, `vendor/core2` stub for yanked core2 0.4.0 via glass_pumpkin/grammers-crypto, wreq fork) must not silently stale:

- New script `scripts/vendor-pull.sh` (runs at build time, before `cargo check` in CI and `Makefile` build target): records upstream URL + pinned rev per source, attempts `cargo update -p <crate>` / `git submodule`-style refetch where applicable, verifies `Cargo.lock` digest, and fails loud (non-zero + diff output) when a pinned source cannot be fetched so stale builds never pass silently.
- `vendor/core2`: annotated as temporary stub with upstream watch (remove when glass_pumpkin/grammers releases drop the yanked core2 dep); script checks crates.io yank status each run.
- wa-rs / wreq-util RC pins (`=6.0.0-rc.23`, `=3.0.0-rc.10`): script flags newer stable/tagged releases and opens the update as an explicit follow-up task — never auto-bumps RC→stable inside a feature phase.
- Phase B0 (precedes B1): land `vendor-pull.sh`, wire into CI + Makefile, record current pins in `docs/planning/VENDOR_PINS.md`. Verification: `scripts/vendor-pull.sh --check` green on clean tree.

## 10. Rust disciplines for the subagent fleet (rust-best-practices, invoked 2026-09-30)

Every implementation subagent working B/F phases follows these (Apollo handbook via skill):

- Ownership: `&str`/`&[T]` params, `Cow` for ambiguous ownership, no redundant `.clone()` (clippy `redundant_clone` clean); small Copy ≤24B by value.
- Errors: `Result<T, E>` + `?`; `thiserror` for library/provider errors, `anyhow` only at binary edges; zero `unwrap`/`expect` outside tests; new fallible paths map to user-friendly `map_error` copy for Tier-1.
- No new `sqlx::query!` macros (AGENTS.md §0.4) — runtime `query()`/`query_as()` + `FromRow`; clippy gate `cargo clippy --all-targets --all-features --locked -- -D warnings` must pass per phase.
- Dispatch: keep existing `Arc<dyn SocialProvider>` object-safety (no generics refactor mid-parity); box at API boundaries only.
- Docs: `///` on new public provider/API methods, `//` for safety rationale; `TODO(#issue)` format only.
- Tests: descriptive names (`publish_should_reject_5th_image_on_x`), one behaviour per test, fixture-based (no live-token dependence for gates); criterion/`black_box` only if perf claims are made — no test-set-fitted constants (mark `provisional:` if inherited from Postiz).
- Delegation prompt footer (orchestrator adds verbatim): "Follow docs/planning/PLAN_PARITY_DEPTH_SINGLEUSER_v25.md §10 + AGENTS.md §§0–0.5; run cargo check + clippy + relevant tests before reporting done."

## 11. Next step

Tiers (§1) approved with 2026-09-30 keeps/removes applied; media-store deferral (§3) approved. Execution order becomes B0 (vendor pins) → §6 sequence. Awaiting explicit execution go-ahead to dispatch the fleet.
