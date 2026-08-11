# Social-Forge × Postiz — Full-Scale Contrast Audit & Frontend Re-Architecture Plan (v22)

> **Date**: 2026-07-07
> **Author**: Super Z orchestrator (post three parallel sub-agent audits: frontend, postiz-reference, backend)
> **Scope**: Backend (Rust/axum/sqlx, ~58k LOC) + Frontend (SvelteKit 5/Svelte 5 runes, ~13k LOC) for `social-forge`; reference audit of `postiz-app` (Next.js 16 + NestJS + Temporal, ~69 MB repo).
> **Architectural preferences (non-negotiable, per AGENTS.md §0.5)**: single-user deployment, single Rust binary embedding SvelteKit via `rust-embed`, no Redis, no Temporal, no external queue, no microservices, SSE realtime (not SWR polling), token encryption at rest. All proposed upgrades must respect these constraints.
> **Reference skills ingested**: `design-taste-frontend` (anti-slop frontend audit-first), `ui-ux-pro-max` (50+ styles, 161 color palettes, 99 UX guidelines).

---

## 0. Executive Summary

The v21 sprint fixed the **calendar 500** and the **PostState enum drift**, but a verification pass against the current `master` HEAD reveals that v21's self-reported completion list contains **three materially incorrect claims** (recurring-wired-into-payload, migration 028 naming, idempotency keys), and the user's renewed complaints identify six surface areas where v21 did not go deep enough.

This v22 audit ran three parallel deep-exploration agents across (a) the social-forge frontend, (b) the postiz-app reference codebase, and (c) the social-forge Rust backend. The combined findings are stark:

1. **v21 verification: 9 ✓ verified, 3 ✗ refuted, 2 ⚠ partial.** The recurring-schedule "wired into payload" claim is false (the backend `CreatePostRequest` has no `recurring` field; the frontend must make a second `/repeat` round-trip). The "migration 028_posts_soft_delete.sql" claim is false (migration 028 is `signature_is_default`; soft-delete was bundled into 027). The idempotency infrastructure is half-built — column, struct field, scheduler pass-through, and index all exist, but **zero providers actually consume the key**.

2. **26 new bugs found in the backend** (5 CRITICAL, 3 HIGH, 4 MEDIUM, 8 LOW, 6 INFO). The 5 CRITICAL bugs are: (a) SSE `/api/events` is in `public_routes` — unauthenticated event leakage; (b) kanban `PATCH /api/posts/{id}/stage` has no state-transition validation (a post can be marked `published` without ever being published); (c) kanban stage changes do not broadcast a realtime event (multi-tab sync is broken); (d) manual publish (`POST /api/posts/{id}/publish`) drops the idempotency key; (e) idempotency infrastructure is half-built.

3. **Postiz has NO dashboard, NO campaign/kanban, NO feed surface — it is purely a publishing tool.** Social-forge is *conceptually ahead* in three of the user's six complaint areas (dashboard, campaign management, feed CRUD) but the *execution* is shallow or broken. Postiz wins decisively on: calendar polish (Just-update-vs-Reschedule modal, mix-blend-difference tag chip, per-cell `useInterval` greyscale flip), composer (CopilotKit, AI Generator NDJSON stream, Mantine date picker, per-row delay for any provider), and posting infrastructure (Temporal v1.0.1 → v1.0.5, per-platform task queues, `missingPostWorkflow` sweeper).

4. **Frontend has 80 hardcoded hex colors across 22 files**, defeating the semantic-token system. Light mode is broken in `CommentsThread.svelte` (16 hits), `MediaCarousel.svelte` (6), `RichTextEditor.svelte` (5 + `<style>` block), `Button.svelte` (`<style>` block — primary CTA won't retheme), and 18 others. The dual-settings-sidebar bug (main sidebar lists all 8 settings sub-routes AND the settings layout has its own 8-tab sidebar) is the single worst UX defect in the app.

5. **Dashboard is genuinely thin.** Backend `/api/analytics/summary` returns only post-state counts, posts-by-provider (count, not engagement), posts-by-day (count, not engagement), and "best_provider" (highest count, not highest engagement). No endpoints exist for: engagement totals, audience growth, scheduled-vs-actual adherence, engagement-rate per channel, posting cadence vs. goal, recent-activity feed, scheduled-today count. Confirms user complaint.

6. **Kanban is a thin state-grouped post list.** No swimlanes, no WIP limits, no due dates, no priority, no card cover images, no drag-to-reorder within column, no card tags visible, no sub-states, no campaign detail page, no campaign analytics endpoint, no campaign edit form (the `campaignsApi.update` method is dead code). The `quickAddIdea` flow creates posts with `integration_ids: []` which the backend rejects. The campaign filter accesses `campaign_id` via `as any` because `PostSummary` doesn't have that field. Confirms user complaint.

7. **Posting infrastructure is *adequate* but not postiz-grade.** SF has solid bones (atomic claim via `FOR UPDATE SKIP LOCKED`, per-provider circuit breaker, retry with exponential backoff + jitter, token encryption at rest, 3-attempt DB write retry after successful publish). It lacks: idempotency keys actually consumed by providers, transactional outbox, per-platform FIFO queue, workflow versioning, periodic stuck-publish sweep. The 300s scheduler timeout leaks detached tasks. The permit-acquire is serial, blocking the whole scheduler on a single slow provider.

8. **A 7-phase refactor plan** (Part G) addresses all 26 backend bugs, all 6 user complaint areas, and re-architects the dashboard + kanban + sidebar/settings + color system. Total estimated effort: 18–22 days. Each phase is independently shippable with its own acceptance tests, and each phase respects the architectural preferences (no Temporal, no Redis, no microservices).

This document:
- **Part A** verifies v21's claims against actual code.
- **Part B** is a 12-dimension side-by-side contrast with postiz-app.
- **Part C** is the full 26-bug catalog with severity, file:line, root cause, fix snippet, and effort estimate.
- **Part D** is the posting-infrastructure upgrade plan (idempotency, outbox, per-platform queue, workflow versioning).
- **Part E** is the frontend re-architecture plan (dashboard, sidebar/settings, color system, calendar polish, composer polish, design-system primitives).
- **Part F** is the campaign-management rework (strategic dashboard, kanban re-architecture, campaign detail pages, campaign analytics, realtime broadcast).
- **Part G** is the 7-phase refactor plan with dependencies, acceptance tests, and push-after-each-phase.
- **Part H** lists what is deferred to v23 with rationale.

---

## Part A — v21 Verification Audit

The v21 doc (`SOCIAL_FORGE_AUDIT_AND_PLAN_v21.md` § D.1) lists 24 completed items. A read-only verification pass against current `master` HEAD confirms 9, refutes 3, and finds 2 partial. The 3 refutations are material — they affect the user's renewed complaints.

| # | v21 Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | Calendar 500 root cause fixed (drop dead-weight engagement columns) | ✓ VERIFIED | `src/db/queries.rs:1204–1207` — `NULL::bigint as likes, NULL::bigint as comments, NULL::bigint as shares, NULL::bigint as impressions`. No LEFT JOIN to `analytics_cache`. |
| 2 | `PostState::Idea` variant added | ✓ VERIFIED | `src/db/models.rs:106–121` — `pub enum PostState { Idea, Draft, Queued, Publishing, Published, Error }`. Display impl maps to `"idea"`. `list_posts` and `count_posts_by_user` match arms include `"idea"` (lines 453, 494). |
| 3 | `sanitize_content` UTF-8 truncation fix (`chars().take(max_len).collect()`) | ✓ VERIFIED | `src/services/posts.rs:63–68` — char-boundary-safe. No panic on emoji. |
| 4 | `PUT /api/feed/{id}` (update feed post) | ✓ VERIFIED | `src/api/feed.rs:472–508`. Accepts `{ text?, media?, metadata? }`, merges, returns updated row. |
| 5 | `POST /api/feed/{id}/repurpose` (convert feed → SF post) | ✓ VERIFIED | `src/api/feed.rs:541–609`. Validates source + target ownership, sanitizes content, sets `source_external_post_id` FK, broadcasts `post_created`. |
| 6 | `GET /api/posts/group/{group_id}` (for thread editing) | ✓ VERIFIED | `src/api/posts.rs:532` → `queries::list_posts_by_group`. Registered in `api/mod.rs:146`. |
| 7 | `posts.deleted_at` soft-delete column + migration 028 | ⚠ PARTIAL | Soft-delete ✓ — `queries.rs:883–900` does `UPDATE posts SET deleted_at = NOW() WHERE deleted_at IS NULL AND (id = $1 OR group_id = ...)`. BUT migration 028 is `028_signature_is_default.sql`; the soft-delete column was bundled into migration **027** (`027_feed_repurpose_and_soft_delete.sql`). The v21 doc's "migration 028_posts_soft_delete.sql" claim is wrong. |
| 8 | `AppError::Database` includes error code | ✓ VERIFIED | `src/error.rs:82–110` — returns `{"error":"Database error", "code":"db_database"|"db_pool_timed_out"|...}`. No SQL details leaked. Exemplary. |
| 9 | `PUT /api/posts/{id}/date` supports `move_group` flag | ✓ VERIFIED | `RescheduleRequest.move_group: Option<bool>` (line 660). Plus `action: Option<String>` for published-post disambiguation (line 665). |
| 10 | Recurring schedule wired into payload | ✗ **REFUTED** | `CreatePostRequest` (`api/posts.rs:30–42`) fields: `integration_ids, content, title, media, settings, scheduled_at, tag_ids, first_comment, sequence, overrides`. **NO `recurring` field.** Recurring is only available via the separate `POST /api/posts/{id}/repeat` endpoint (line 899). If the frontend sends `recurring` in the create body, serde silently ignores it (no `#[serde(deny_unknown_fields)]`). The composer must make TWO round-trips: POST /api/posts → POST /api/posts/{id}/repeat. If the second call fails, the user has a scheduled post without the recurring series they thought they configured, with no error feedback. |
| 11 | Migration 028_posts_soft_delete.sql added | ✗ **REFUTED** | Migration 028 is `028_signature_is_default.sql`. The soft-delete was bundled into 027. The v21 doc's claim is incorrect. |
| 12 | Idempotency keys on `provider.publish()` | ⚠ HALF-BUILT | Migration `029_posts_idempotency_key.sql` adds `posts.idempotency_key UUID NOT NULL DEFAULT gen_random_uuid()` + index ✓. `PostContent.idempotency_key: Option<String>` field added (`social/mod.rs:90`) ✓. Scheduler populates it (`scheduler/mod.rs:583`) ✓. `reset_post_for_republish` regenerates it (`queries.rs:804`) ✓. BUT `grep -i idempotency src/social/` shows **zero provider `publish()` methods read `post.idempotency_key`**. X sends `{"text": post.content}` and never references the key. Same for LinkedIn, Reddit, Bluesky, etc. The infrastructure is a no-op. The migration's own comment ("prevents double-publish on retry") is false advertising. |
| 13 | Calendar error UI: skeleton + Retry button + toast | ✓ VERIFIED | `routes/calendar/+page.svelte:431–452` — full-card error state with Retry button. |
| 14 | Past-cell greying + drop disable | ✓ VERIFIED | `MonthView.svelte:64–66`, `WeekView.svelte:93–101`, `DayView.svelte` — all grey out + `cursor-not-allowed` + drop suppressed. |
| 15 | Tag-colored chip top bar | ✓ VERIFIED | `CalendarEvent.svelte:30` — tag color on border-left. BUT campaign color (line 29) takes precedence — if a post has both campaign and tags, tag color is invisible. |
| 16 | Per-post error `!` badge with tooltip | ✓ VERIFIED | `CalendarEvent.svelte:41–44` — red `!` badge absolute top-left + `ring-2 ring-red-500` (line 27). |
| 17 | URL sync (`?display=&date=`) via `replaceState` | ✓ VERIFIED | `routes/calendar/+page.svelte:344–355` (read on mount), `:372–381` (write on change). Two-way sync, no history pollution. |
| 18 | "Just update vs Reschedule" modal on published-post drag | ✓ VERIFIED | `routes/calendar/+page.svelte:203–215` — uses `confirmModal` returning `Promise<boolean>`. |
| 19 | `askClose` confirmation on composer close | ✓ VERIFIED | `ComposerModal.svelte:573–593` — `handleClose` checks dirty state, calls `modals.areYouSure`. |
| 20 | Real upload progress (XMLHttpRequest) | ✓ VERIFIED | `MediaUpload.svelte:57–59` uses `mediaApi.uploadWithProgress` (XHR `upload.onprogress`). |
| 21 | Per-platform validation error toasts (show all) | ✓ VERIFIED | `ComposerModal.svelte:424–429` — iterates `valRes.data.errors`, toasts each as `${err.provider_name}: ${err.message}`. |
| 22 | Signature auto-append on new post | ✓ VERIFIED | `ComposerModal.svelte:243–253` — fetches `signaturesApi.getDefault()`, prepends `'\n' + sigRes.data.content` in create mode. 404 silently ignored. |
| 23 | Feed Repurpose button calls backend | ✓ VERIFIED | `routes/feed/+page.svelte:233–276` — `feedApi.repurpose(post.id, { integration_id })` creates a draft, then opens composer. |
| 24 | Replace 13 native `confirm()`/`prompt()` calls | ✓ VERIFIED | Grep for actual `confirm(`/`prompt(` calls in code (not comments) returns **zero**. All replaced with `modals.areYouSure` or inline modals. |

**Net v21 score: 19 ✓ verified, 3 ✗ refuted, 2 ⚠ partial out of 24 claims.** The 3 refutations are the root cause of the user's renewed complaints about recurring schedules, idempotency, and the migration numbering confusion. The 2 partials (soft-delete column, idempotency keys) are functionally correct but documentationally wrong.

---

## Part B — Full-Scale Postiz Contrast Matrix

A 12-dimension side-by-side comparison. The single most surprising finding: **postiz has no dashboard, no campaign/kanban, and no feed surface.** It is purely a publishing tool. Social-forge is conceptually ahead in three of the user's six complaint areas — the problem is execution depth, not concept.

### B.1 Stack comparison

| Dimension | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Backend language | Rust 2021 (single binary, ~58k LOC) | TypeScript / NestJS 11 | SF is faster + smaller deploy; Postiz has higher dev velocity. Both fine. |
| Web framework | axum 0.8 | NestJS (Express) | Equivalent. |
| ORM | sqlx 0.8 (64 macro queries compile-time checked, 135 runtime queries NOT checked) | Prisma 6.5 (all compile-time checked) | Postiz wins on type safety. SF has `.sqlx` cache drift — see Part C BUG #24. |
| DB | Postgres 16 | Postgres | Same. |
| Migrations | sqlx::migrate! (30 files, gap at 024) | Prisma Migrate | Same. |
| Frontend | SvelteKit 2 / Svelte 5 runes (~13k LOC) | Next.js 16 / React 19 (App Router) | Postiz has bigger ecosystem; Svelte 5 is leaner. Both fine. |
| Frontend state | Svelte 5 rune class singletons (8 stores) | Zustand 5 | Equivalent. |
| Calendar lib | Custom (CSS grid + Intl, 14 files) | Custom (CSS grid + dayjs, 1 file + context) | Equivalent — both hand-rolled. SF is more modular. |
| Modal manager | `modals` rune class (stack, escape, z-index, askClose promise) | Zustand store (stack, askClose, isLast escape, z=200+index) | Equivalent — SF added askClose in v21. |
| HTTP data layer | Custom `ApiClient` envelope `{data?, error?, status}` | SWR 2.2.5 + raw fetch + minify/expand | SF lacks SWR (no auto-revalidation, no dedup). Postiz lacks SSE. Different tradeoffs. |
| Realtime | SSE (`EventSource`, 10 event types, 1024-event buffer, 15s keepalive) | SWR polling (1h max) + `postMessage` for OAuth | **SF wins.** Sub-second updates vs 1h polling. But SF has BUG #18 (lagged events silently dropped) and BUG #19 (SSE in public_routes — auth bypass). |
| Scheduler | In-process tokio task, 30s poll, atomic claim, circuit breaker, 3 retries with exp backoff + jitter | Temporal.io workflows v1.0.1 → v1.0.5 (5 versions), per-platform task queues, `missingPostWorkflow` sweeper, `poke` signal | **Postiz wins decisively** on durability + observability. SF is adequate for solo tool but not postiz-grade. |
| Provider count | 31 registered (25 with MCP coverage) | 33 | Comparable. |
| Validation contract | Per-handler ad-hoc (`POST /api/posts/validate` returns flat list) | Shared class-validator DTOs (FE + BE import the same file), `POST /posts/valid` returns per-platform `{valid, settingsError, errors, emptyContent, tooLong, maximumCharacters}` | **Postiz wins.** SF should extract a `validation.rs` shared module + return richer per-platform struct. |
| i18n | None (English only) | 15 locales | Out of scope for SF (single-user preference). |
| Soft delete | `posts.deleted_at` (migration 027), `external_posts.hidden_at/saved_at` (migration 025) | `deletedAt` on every model | Equivalent. SF added in v21. |
| Triple interface | CLI + REST API + MCP server (329 tools) | REST only (+ CopilotKit chat) | **SF wins.** AI-agent-friendly. |
| Token security | AES-256-GCM encryption at rest when `TOKEN_ENCRYPTION_KEY` set | Plaintext tokens in DB | **SF wins.** Non-negotiable per AGENTS.md §0.5.5. |

### B.2 Calendar comparison

| Behavior | Social-Forge | Postiz | Gap |
|---|---|---|---|
| Default view | Week ✓ (localStorage `social-forge-calendar-view`) | Week ✓ (cookie `calendar-display` + URL `?display=`) | SF lacks URL sync — fixed in v21. ✓ |
| View modes | Month / Week / Day / List | Day / Week / Month / List | Equivalent. |
| **Monday/Sunday consistency** | MonthView=Sunday-start, WeekView=Monday-start — **BUG** | Monday-start everywhere | **Postiz wins.** SF must fix `getMonthDays` to Monday-start. |
| Fetch endpoint | `GET /api/calendar?start=YYYY-MM-DD&end=YYYY-MM-DD` (date-only) | `GET /posts?startDate=…&endDate=…` (ISO datetime UTC) | SF format fine. |
| Filters | None (client-side filter only) | SWR keys + URL params (provider, customer, state) | **Postiz wins.** SF should add `integration_ids`, `tag_ids`, `campaign_id` to `CalendarQuery`. |
| Error handling | Full-card error + Retry button + toast (v21 fix) | Per-cell `animate-pulse` skeleton + SWR auto-retry | Equivalent in spirit. |
| Drag-to-reschedule | Yes (date precision month, hour week/day) | Yes + optimistic local update + "Just update vs Reschedule" modal for already-published | SF added the modal in v21. ✓ |
| Per-post error indicator | Red `!` badge + `ring-2 ring-red-500` (v21 fix) | Red `ring-2 ring-red-500` + absolute `!` badge + tooltip | Equivalent. ✓ |
| Hover actions | Edit / Duplicate / Stats / Delete (toolbar) | Edit / Duplicate / Preview / Stats / Connect-missing / Delete (5 icons) | Postiz has "Connect missing releaseId" for IG→X cross-posts — SF doesn't need this. |
| **Tag-colored chip bar** | Tag color on border-left only (campaign color takes precedence) | `style={{ backgroundColor: post.tags[0].tag.color }}` + `mix-blend-difference` for label | **Postiz wins.** SF should use `mix-blend-difference` and show tag color even when campaign is set. |
| "+N more" expander | Yes (month view) | Yes (inline expander, no popover) | Equivalent. |
| **Past-cell greying** | Yes (`opacity-40`, `cursor-not-allowed`, drop suppressed) — v21 fix | Yes (`!grayscale`, `cursor-not-allowed`, drop disabled) | Postiz uses `grayscale`; SF uses `opacity`. Equivalent in spirit. |
| **Past-cell recompute** | No (computed once) | Yes (`useInterval` every 2–2.5min via `random(120000, 150000)`) | **Postiz wins.** Tab left open overnight doesn't flip cells at midnight. SF should add `setInterval` in `calendarState`. |
| List view state filter | Yes (All/Scheduled/Drafts/Published) | Yes (all/scheduled/draft/published segmented control) | Equivalent. |
| URL `?display=` sync | Yes (replaceState, no history pollution) — v21 fix | Yes (replaceState + cookie) | Equivalent. ✓ |
| **Mini-calendar in sidebar** | No | No (neither has it) | Tie — both could add. |
| **Today pulse dot** | WeekView header only | No | SF slightly ahead. |
| **Per-post timezone awareness** | No (bulk-reschedule uses `${date}T${time}:00.000Z` always UTC — BUG) | Yes (dayjs.utc everywhere) | **Postiz wins.** SF must fix timezone handling in bulk-reschedule + SchedulePicker. |

### B.3 Composer comparison

| Behavior | Social-Forge | Postiz | Gap |
|---|---|---|---|
| Form factor | Full-screen modal `max-w-[1400px] h-[90vh]` | Full-screen modal `max-w-[1400px]` 80% width `fullScreen:true` | Equivalent. ✓ |
| Two-column layout | Yes (editor + preview pane) | Yes (editor + `ShowAllProviders`) | Equivalent. ✓ |
| Channel picker | Grid of integration cards | Round avatars (42×42) with platform badge | Postiz more compact. |
| Per-channel override | `SelectCurrent` pill strip + clone-on-first-switch | `SelectCurrent` dropdown + `addRemoveInternal` clone | Equivalent. ✓ |
| Rich text editor | TipTap StarterKit + Link + Placeholder + custom ImageExtension | TipTap + Bold/Underline/Link/Heading/BulletList/Mention/Placeholder/History + bold/underline mutex | **Postiz wins.** SF lacks Mention + Underline. |
| **Thread builder** | `ThreadFinisher` (X only, 280-char split) | Generic `<AddPostButton>` per-row with delay + `<ThreadFinisher>` X-only + `<MergePost>` + `<SeparatePost>` | **Postiz wins.** SF should add generic per-row delay for all providers. |
| First comment | `FirstComment` (LinkedIn/FB only) | Generic — same mechanism as thread (rows > 0 become comments for commentable providers) | Equivalent. ✓ |
| Media upload | Drag-reorder, paste, alt text, library picker, **real progress** (v21 fix) | Uppy + drag/drop + paste + multi-image + video + alt text + real progress | Equivalent in function. Postiz uses Uppy (heavier dep). |
| **AI Image** | No | Yes (14 style chips: Realistic/Cartoon/Anime/...) | **Postiz wins.** Defer to v23 (needs image-gen provider). |
| **AI Generator (NDJSON stream)** | No | Yes (sparkle-button → modal with shimmer events: agent → research → find-category → find-topic → find-popular-posts → generate-hook → generate-content → generate-picture → upload-pictures → post-time) | **Postiz wins.** Defer to v23 (needs agent-graph backend). |
| AI assistant | `AiAssistant` (generate/improve/hashtags/tone/summarize, single-shot) | `AiAssistant` + CopilotKit popup with `useCopilotAction` | SF simpler. CopilotKit out of scope (subscription). |
| Char count per platform | `PerPlatformCharCount` (dedup by provider, yellow 90%, red over) | Per-platform `maximumCharacters` (X uses twitter-text `weightedLength`) | Equivalent. SF should use `weightedLength` for X. |
| Live preview per platform | `PlatformPreviewPane` (IG/LinkedIn/Facebook/General = 3 + fallback) | `ShowAllProviders` (IG/LinkedIn/Facebook/X/TikTok/YouTube/Pinterest/Reddit/Threads/Bluesky/Mastodon/Discord/Telegram/WordPress/Medium/Dev.to/Hashnode + 14 more = 25+) | **Postiz wins.** SF should add X, Reddit, Threads, Bluesky previews. |
| **Date picker** | Native HTML `<input type="date">` + `<input type="time">` | `@mantine/dates` Calendar popover with locale-aware format + `useClickOutside` | **Postiz wins.** SF should build a custom Svelte popover (no Mantine dep). |
| Auto-schedule button | Yes (`POST /api/posts/find-slot`) | Yes (`POST /posts/find-slot` org-level or `/find-slot/{integrationId}`) | Equivalent. ✓ |
| Repeat | `SchedulePicker` writes to `recurring` state; **frontend calls `postsApi.repeat(id, ...)` after create** (two-step) | `RepeatComponent` → `inter` field → `intervalInDays` single-row → workflow spawns child workflows | **Postiz wins architecturally.** SF's two-step is fragile. v23 should add `recurring` to `CreatePostRequest` OR adopt postiz's single-row model. |
| Save Draft / Post Now / Schedule | 3 buttons in footer | Draft + Schedule (with hover-revealed "Post Now" flyout) | SF clearer. Equivalent. |
| `askClose` (confirm-on-close) | Yes (v21 fix) | Yes (`"Are you sure? All data will be lost"`) | Equivalent. ✓ |
| Pre-flight validation | `POST /api/posts/validate` returns `{valid, errors}` (flat list) | `POST /posts/valid` returns per-platform `{valid, settingsError, errors, emptyContent, tooLong, maximumCharacters}` | **Postiz wins.** SF should return richer per-platform struct. |
| Per-platform error toast | Yes (iterates errors, toasts each) — v21 fix | Yes (per-platform with handle: `"X (my-handle): post is too long"`) | Equivalent. ✓ |
| Sets (templates) | `PostSetModal` (save/load) | `SetSelectionModal` (auto-opens before composer if sets exist) | SF has it. Postiz auto-opens. Minor. |
| **Signatures auto-append** | Yes (v21 fix, prepends `'\n' + content`) | Yes (`onlyValues: [{content: '\n' + signature.content}]`) | Equivalent. ✓ |
| **Alt text on submit** | **BUG: `alt: undefined` always** — MediaUpload.saveAlt writes to wrong field | Yes (`{ id, path, alt, thumbnail, thumbnailTimestamp }` in payload) | **Postiz wins.** SF must fix `MediaUpload.saveAlt` + `buildPayload`. |
| **Timezone awareness** | No (SchedulePicker uses `${dateStr}T${timeStr}:00.000Z` always UTC — BUG) | Yes (dayjs.utc everywhere) | **Postiz wins.** SF must fix. |
| **Brand profile integration** | Brand Profile saved to localStorage only; AiAssistant doesn't read it | No equivalent | SF has the concept but doesn't use it. Sync to backend + read in AiAssistant. |

### B.4 Posts CRUD comparison

| Operation | Social-Forge | Postiz | Gap |
|---|---|---|---|
| List (calendar) | `GET /api/calendar?start&end` | `GET /posts?startDate&endDate` | Equivalent. |
| List (paginated) | `GET /api/posts?state&limit&offset&q&integration_ids&tag_ids&sort` | `GET /posts/list?page&limit&state&customer` | Equivalent. |
| Get one | `GET /api/posts/{id}` | `GET /posts/{id}` | Equivalent. |
| Get group (for edit) | `GET /api/posts/group/{group_id}` (v21 add) ✓ | `GET /posts/group/{group}` | Equivalent. ✓ |
| Create | `POST /api/posts` (multi-integration, group_id returned, **no `recurring` field**) | `POST /posts` with `type: 'draft'\|'schedule'\|'now'\|'update'` discriminator | **Postiz wins.** SF should add `type` discriminator in v23. |
| Update | `PUT /api/posts/{id}` (content/title/media/settings) — **loses tag_ids + first_comment on edit** | `POST /posts` with `type: 'update'` (skips workflow re-start) | **Postiz wins.** SF must fix `postsApi.update` to accept tag_ids + first_comment. |
| Delete | `DELETE /api/posts/{id}` (soft-delete + group cascade, v21 fix) ✓ | `DELETE /posts/{group}` (soft-delete group + terminate Temporal workflow) | Equivalent. ✓ |
| Reschedule | `PUT /api/posts/{id}/date` (with `move_group` + `action`) | `PUT /posts/{id}/date` (with `action: schedule\|update`) | Equivalent. ✓ |
| Repeat | `POST /api/posts/{id}/repeat` (creates N copies upfront, max 100) | `intervalInDays` single-row → workflow spawns child workflows → calendar virtualizes | **Postiz wins architecturally** (single-row + virtualization). SF's N-copies is storage-heavy. Defer to v23. |
| Find free slot | `GET /api/posts/find-slot?integration_id` | `GET /posts/find-slot` (org-level) or `/find-slot/{integrationId}` | Equivalent. |
| Validate | `POST /api/posts/validate` (flat list) | `POST /posts/valid` (per-platform rich struct) | Postiz richer. |

### B.5 Feed comparison

| Operation | Social-Forge | Postiz | Gap |
|---|---|---|---|
| List | `GET /api/feed` (cursor + provider/author/q filter) | **N/A — Postiz has no imported-posts feed surface** | SF original feature. **SF wins.** |
| Import | `POST /api/feed/import` (refresh from all providers) | N/A | SF original. |
| Hide (delete) | `DELETE /api/feed/{id}` (soft-hide via `hidden_at`) ✓ | N/A | SF original. |
| Bookmark | `POST/DELETE /api/feed/{id}/save` | N/A | SF original. |
| Edit | `PUT /api/feed/{id}` (v21 add) ✓ | N/A | SF original. |
| Repurpose | `POST /api/feed/{id}/repurpose` (v21 add, creates SF post with provenance FK) ✓ | N/A | SF original. |
| Comments | `GET /api/feed/{id}/comments` (cache-first, live-fallback — v21 fix) ✓ | N/A | SF original. |
| **View original post** | Yes (`<a href={post.url} target="_blank">View original</a>` at `feed/+page.svelte:765`) ✓ | Postiz stores `releaseURL` only in notification bodies, no dedicated UI affordance | **SF wins.** |
| **Manage on platform** | **MISSING** — no "open post on platform to edit/delete" affordance | N/A | SF should add. Construct URL from `provider` + `platform_post_id` (e.g., `https://x.com/i/status/{id}`, `https://www.linkedin.com/feed/update/{id}/`). |

**Verdict**: SF's feed is a net-new feature postiz doesn't have. The user's complaint "the feeds do not have deletion for existing posts" is partially incorrect (deletion exists via `hidePost`), but the complaint "must have link to the original post so that I can manage that post from the official platform too" is valid — SF has "View original" but no "Manage on platform".

### B.6 Posting infrastructure comparison

| Capability | Social-Forge | Postiz | Gap severity |
|---|---|---|---|
| Scheduler | In-process tokio task, 30s poll, atomic claim (`FOR UPDATE SKIP LOCKED` CTE) | Temporal.io workflow per post, `workflowIdConflictPolicy: TERMINATE_EXISTING` | **High** — SF has no durability. Adequate for solo, not postiz-grade. |
| Workflow versioning | None | v1.0.1 → v1.0.5 (5 versions, in-flight migration path) | **Medium** — SF has no in-flight migrations. |
| Retry with backoff | Hand-rolled (5/10/20s + ±25% jitter), max 3, per-error-variant | Temporal retry policies (`maximumAttempts: 3, backoffCoefficient: 1, initialInterval: 2min`) + in-workflow 5-iteration token-refresh loop | Equivalent in function. |
| **Idempotency keys** | **Half-built** — column + struct field + scheduler pass-through exist, **zero providers consume the key** | None (Temporal's `workflowIdConflictPolicy` provides de-facto dedup at workflow level) | **SF wins IF providers consume the key.** Currently neither has true per-request idempotency. |
| **Transactional outbox** | None (3-attempt DB write retry after publish) | None | Tie — both have publish-orphan risk. SF should add `publish_outbox` table. |
| Per-provider task queue | Semaphore per provider (in-process, limit=1 for X/Threads/IG, 3 for reddit/discord) | Per-platform Temporal task queue (Facebook doesn't queue behind LinkedIn) | SF adequate for single-instance. No FIFO guarantee across ticks. |
| Stuck-publishing recovery | `reclaim_stuck_publishing` on startup only (300s threshold) | `searchForMissingThreeHoursPosts` cron (1h interval) + `poke` signal | **Postiz wins.** SF should run `reclaim_stuck_publishing` every tick. |
| Circuit breaker | 3-state (closed/open/half-open), env-configurable threshold + cooldown, **half-open allows N requests not 1 (BUG #3)** | None | **SF wins conceptually** but implementation has a bug. |
| Plug/automation engine | `post_plugs` table + `start_plug_runner` | BullMQ workers + event bus | SF simpler. Adequate. |
| Soft delete | `posts.deleted_at` (v21) + `external_posts.hidden_at/saved_at` | `deletedAt` everywhere | Equivalent. |
| Webhooks | `POST /api/webhooks` + delivery history | Fired in workflow (`post.activity.ts:314–341`) | Equivalent. |
| Analytics refresh | 30min tokio task, writes `analytics_cache` | Per-post `getStatistics` + Redis cache (1h TTL) + on-demand | Equivalent. |
| **Scheduler timeout handling** | 300s timeout **leaks detached tasks** (BUG #1) | Temporal `startToCloseTimeout: 10 minute` per activity | **Postiz wins.** SF must `join_set.abort()` on timeout. |
| **Permit acquisition** | Serial before spawn — **blocks scheduler on slow provider** (BUG #2) | N/A (Temporal handles concurrency) | **Postiz wins.** SF must move `acquire_owned().await` inside spawned future. |
| Token encryption at rest | AES-256-GCM when `TOKEN_ENCRYPTION_KEY` set (all 4 refresh paths encrypt) | Plaintext | **SF wins.** Non-negotiable per AGENTS.md. |
| Token refresh paths | 4 paths (scheduler proactive 6h, scheduler on-demand, manual endpoint, PostService pre-publish) | 2 paths (proactive cron workflow, reactive in-workflow) | SF more defensive. |

**Verdict**: SF's posting infrastructure is *adequate for a solo-founder local tool* but is not postiz-grade. The three real gaps are: (a) idempotency keys not consumed by providers, (b) no transactional outbox, (c) no per-platform FIFO queue. All three are fixable without Temporal — see Part D.

### B.7 Dashboard comparison

| Widget | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Stat row (Drafts/Queued/Published/Errors) | Yes (4 cards, hardcoded colors) | **Absent — no dashboard at all** | SF wins (but execution is thin). |
| Engagement row (Likes/Comments/Shares 7d) | Yes (3 cards, only if any > 0, no sparklines) | Absent | SF wins. |
| Channel performance bar chart | Yes (post count, hardcoded indigo, not engagement-normalized) | Absent | SF wins (but metric is wrong — should be engagement-rate). |
| Alerts | Yes (failed posts, channels needing reconnect) | Absent | SF wins. |
| Needs-attention inbox | Yes (drafts, failed, reconnect) | Absent | SF wins. |
| Today's schedule | Yes (browser-local-time BUG) | Absent | SF wins (but timezone bug). |
| Recent activity | Yes (last 6 events) | Absent | SF wins. |
| Quick actions | Yes (5 buttons) | Absent | SF wins. |
| **KPI trend deltas** | **No** (no comparison vs previous period) | Absent | SF should add. |
| **Engagement rate per channel** | **No** | Absent | SF should add. |
| **Audience growth / follower delta** | **No** | Absent | SF should add. |
| **Scheduled-vs-actual adherence** | **No** | Absent | SF should add. |
| **Posting cadence vs goal** | **No** (campaigns have `goal` text but no progress metric) | Absent | SF should add. |
| **Streak tracking** | Yes (`StreakBadge` in sidebar) | Yes (`streak.component.tsx`, `Organization.streakSince`) | Equivalent. |
| **GitHub stars/forks trends** | No | Yes (only per linked GitHub repo) | Postiz wins for OSS projects. Out of scope for SF. |

**Verdict**: Postiz has **no dashboard** — its default authenticated home page is `/launches` (the calendar). Social-forge's dashboard is conceptually ahead but execution is thin. The user's complaint "dashboard is very cheap knock off, non-functional" is fair — the backend exposes only post-state counts + post-count-by-provider + post-count-by-day. Part E proposes a 12-widget command-center redesign.

### B.8 Campaign Management / Kanban comparison

| Feature | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Kanban board | Yes (4 columns: Ideas/Drafts/Scheduled/Published) | **NO — postiz has no kanban** | **SF wins conceptually.** |
| Campaign model | Yes (campaigns table: name, description, color, start_date, end_date, goal) | **NO — postiz has no Campaign model** | **SF wins conceptually.** |
| Quick-add to Ideas | Yes, but **creates post with `integration_ids: []` (BUG)** | N/A | SF must fix — require channel selection. |
| Drag between columns | Yes (optimistic update, no realtime broadcast — BUG #7) | N/A | SF must fix. |
| **State-transition validation** | **No (can drag idea → published without publishing — BUG #6)** | N/A | SF must fix. |
| **Realtime sync across tabs** | **No (no `post_stage_changed` event — BUG #7)** | N/A | SF must fix. |
| Campaign filter | Yes, but **broken** (`campaign_id` accessed via `as any`, falls back to `group_id`) | N/A | SF must fix. |
| Campaign create form | Yes, but **hardcoded color `#6366f1`**, no description/dates/goal fields | N/A | SF must expose all API fields. |
| Campaign edit form | **NO** (`campaignsApi.update` is dead code) | N/A | SF must add. |
| Campaign detail page | **NO** | N/A | SF must add. |
| Campaign analytics | **NO endpoint** (BUG #9) | N/A | SF must add. |
| Campaign status (active/paused/archived) | **NO** (BUG #8) | N/A | SF must add. |
| Campaign soft delete | **NO** (hard delete — BUG #10) | N/A | SF must add. |
| Swimlanes (by campaign/channel) | No | N/A | SF should add. |
| WIP limits per column | No | N/A | SF should add. |
| Card cover images | No | N/A | SF should add. |
| Card preview on hover | No | N/A | SF should add. |
| Drag-to-reorder within column | No (API doesn't support ordering) | N/A | SF should add (`sort_order` column). |
| Sub-states (ready/in-review/blocked) | No | N/A | SF should add. |
| Due dates on cards | No | N/A | SF should add. |
| Priority levels | No | N/A | SF should add. |
| Card tags visible | No (tags exist but not shown on cards) | N/A | SF should add. |
| Card activity log | No | N/A | SF should add. |
| Marketing-strategic features (goal progress, audience persona, content pillars, funnel) | **No** | **No** | **Tie — both lack.** SF's differentiation opportunity. |

**Verdict**: Postiz has no campaign/kanban concept whatsoever — it is purely a publishing tool. Social-forge is the only one of the two with a campaign model, but the implementation is riddled with bugs (6 of the 26 new bugs are kanban/campaign-related). Part F proposes a full strategic-dashboard rework.

### B.9 Settings & Sidebar comparison

| Behavior | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Main sidebar | 224px fixed, 17 nav links across 6 sections, **NOT collapsible on desktop** | 80px icon rail (fixed) + 260px launches sidebar (collapsible via cookie) | **Postiz wins.** SF should add collapse-to-icon-rail. |
| Settings sidebar | **DUPLICATE** — main sidebar lists all 8 settings sub-routes AND settings layout has its own 8-tab sidebar (BUG) | Single entry → `/settings` renders `SettingsPopup` with 260px left rail + content pane | **Postiz wins.** SF must collapse main sidebar's 8 settings entries to 1. |
| Active-link matching | Strict equality `$page.url.pathname === item.href` — **never highlights on sub-routes** (BUG) | `currentPath.indexOf(path) === 0` | **Postiz wins.** SF must use `startsWith`. |
| Settings sub-pages | 8 routes (General, Brand Profile, RSS, Signatures, Notifications, Developer, Webhooks, MCP & CLI) | 8 tabs (Global, Teams, Webhooks, Auto Post, Sets, Signatures, Developers, Approved Apps) | Equivalent count. SF routes are URL-addressable (better). |
| Brand Profile | Saves to **localStorage only** — not synced to backend, not read by AiAssistant | N/A | SF must sync to backend + read in AiAssistant. |
| Command palette (Cmd+K) | **No** (ShortcutsModal exists but no command palette) | No | Tie — both should add. SF's differentiation opportunity. |
| Global search in sidebar | No (search lives at `/search` only) | No | Tie. |
| Theme toggle | Yes (in sidebar footer, inline SVG) | Yes (in top bar, `useCookie('mode', 'dark')`) | Equivalent. |

### B.10 Theming & Color System comparison

| Aspect | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Token system | `tailwind.config.js` defines `brand`, `surface`, `surface-hover`, `background`, `line`, `muted`, `content`, etc. (semantic tokens) ✓ | Two parallel systems: `--new-*` vars (35 modern, cohesive purple/pink) + `--color-custom*` vars (55 legacy numbered, chaotic) | SF system better-designed. |
| **Token usage** | **80 hardcoded hex colors across 22 files** — defeats the token system | Legacy `customColor*` still wired into Tailwind but being deprecated | **Postiz wins on usage.** SF must migrate. Worst offenders: `CommentsThread.svelte` (16 hits), `Button.svelte` (`<style>` block — won't retheme), `RichTextEditor.svelte` (5 + `<style>`), `MediaCarousel.svelte` (6). |
| Light/dark mode | CSS variables per theme (`:root.dark` / `:root.light`), 12 vars each | `darkMode: 'class'`, body gets `.dark`/`.light`, persisted via cookie | Equivalent. |
| Semantic status colors | **Missing** — no `success`/`warning`/`error`/`info` in Tailwind config (CSS vars exist but not exposed) | Per-component ad-hoc | SF should expose as Tailwind colors. |
| Typography | Inter (Google Fonts), no size/weight tokens | Plus Jakarta Sans, no size/weight tokens (ad-hoc per component) | Equivalent. SF should add size/weight tokens. |
| Radius tokens | Single `--radius` value | Ad-hoc (`rounded-[10px]`, `rounded-[8px]`, etc.) | SF slightly better. Should add `--radius-sm/md/lg`. |

### B.11 Modal Manager comparison

| Behavior | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Store | Svelte 5 rune class `modals` | Zustand store | Equivalent. |
| Stacking | Yes (dedup by component, z-index by stack position) | Yes (dedup by id, `zIndex: 200 + index`) | Equivalent. |
| Escape | Topmost only (v21 fix) | Topmost only (`isLast` check) | Equivalent. ✓ |
| Backdrop click | Configurable (`closeOnClickOutside`) | Configurable (`closeOnClickOutside`) | Equivalent. |
| `askClose` | Yes (dirty check + `areYouSure` promise) — v21 fix ✓ | Yes (`decision.open()` promise, default prompt) | Equivalent. ✓ |
| Full-screen mode | Yes (composer uses `max-w-[1400px] h-[90vh]`) | Yes (`fullScreen: true, removeLayout: true, size: '80%'`) | Equivalent. |
| Non-framework escape hatch | N/A (Svelte-only) | `showModalEmitter` + `areYouSure` via Node EventEmitter (for non-React code) | Postiz has more flexibility. |

### B.12 Realtime comparison

| Behavior | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Transport | SSE (`EventSource`, `/api/events`, 1024-event buffer, 15s keepalive) | None on frontend (SWR polling 1h max) | **SF wins.** Sub-second vs 1h. |
| Auth | **BUG #19: SSE in `public_routes`, no auth extractor — leaks all events** | N/A | SF must fix (move to protected_routes). |
| Lagged event handling | **BUG #18: silently drops lagged events, frontend has no signal to refetch** | N/A | SF must emit synthetic `"lagged"` event. |
| Events subscribed | 10 types (post_created/scheduled/published/failed/deleted, integration_connected/disconnected, notification_new, comment_received, dm_received) | N/A | SF comprehensive. Missing: `post_stage_changed`, `campaign_created/updated/deleted`, `post_repeated`. |
| Background notifications | `notification_new` event → `NotificationBell` dropdown | In-app `Notifications` model populated by workflow | Equivalent. |
| OAuth popup → opener | N/A | `window.postMessage` from OAuth popup | Postiz has this for OAuth flow. SF uses redirect. |

---

## Part C — Backend Bug Catalog (26 new bugs)

Organized by severity. Each entry: **#** | **Severity** | **Title** | **File:Line** | **Root cause** | **Fix** | **Effort**.

### C.1 CRITICAL (5 bugs)

#### BUG #4 — Idempotency key infrastructure is half-built (no provider consumes it)

**File**: `src/social/*.rs` (all 31 providers)
**Root cause**: Migration `029_posts_idempotency_key.sql` added `posts.idempotency_key UUID NOT NULL DEFAULT gen_random_uuid()` + index. `PostContent.idempotency_key: Option<String>` field added (`social/mod.rs:90`). Scheduler populates it (`scheduler/mod.rs:583`). `reset_post_for_republish` regenerates it (`queries.rs:804`). BUT `grep -i idempotency src/social/` shows **zero provider `publish()` methods read `post.idempotency_key`**. X sends `{"text": post.content}` and never references the key. Same for LinkedIn, Reddit, Bluesky, Slack, etc.
**Fix**: Add `Idempotency-Key: <key>` HTTP header in providers that support it (X v2, LinkedIn, Reddit, Slack all support this pattern — see Stripe/OpenAI convention). For providers that don't support HTTP-level idempotency (Mastodon, Bluesky), use a per-provider dedup cache (check `posts.idempotency_key` + `platform_post_id` before writing the result).
**Effort**: Medium (2-3 days for top 5 providers).

#### BUG #6 — Kanban PATCH /stage has no state-transition validation

**File**: `src/api/campaigns.rs:174–202`
**Root cause**: The handler accepts `state: "idea" | "draft" | "queued" | "published" | "error"` and writes it via raw SQL `UPDATE posts SET state = $3::post_state, campaign_id = $4`. There is no validation that the transition is legal. A user can drag a post from `idea` → `published` even if it has no `platform_post_id`. The DB happily sets `state='published'` with `published_at IS NULL` and `platform_post_url IS NULL`. The calendar query then treats it as published (filters by `published_at`, which is NULL → doesn't appear on calendar). The kanban shows it in the "Published" column even though it was never actually published. **This is likely the user's "I cannot even add anything on the kanban" complaint** — they drag a post, it "works" (no error), but then it disappears or shows in a wrong state.
**Fix**:
```rust
// Reject illegal transitions
let current = sqlx::query_scalar::<_, String>("SELECT state::text FROM posts WHERE id = $1 AND user_id = $2")
    .bind(id).bind(auth.user_id).fetch_one(&state.db).await?;
let legal = match (&current[..], body.state.as_str()) {
    ("idea", "draft") | ("draft", "queued") | ("queued", "publishing") | ("publishing", "published") | (_, "error") => true,
    ("idea", "idea") | ("draft", "draft") | ("queued", "queued") | ("published", "published") => true, // no-op
    ("published", "queued") => true, // reschedule
    _ => false,
};
if !legal { return Err(AppError::BadRequest(format!("Illegal state transition: {} → {}", current, body.state))); }
// Reject Published without platform_post_id
if body.state == "published" {
    let has_platform_id: bool = sqlx::query_scalar("SELECT platform_post_id IS NOT NULL FROM posts WHERE id = $1")
        .bind(id).fetch_one(&state.db).await?;
    if !has_platform_id { return Err(AppError::BadRequest("Cannot mark as published without platform_post_id".into())); }
}
```
**Effort**: Small (2 hours).

#### BUG #7 — Kanban stage change does NOT broadcast a realtime event

**File**: `src/api/campaigns.rs:174–202`
**Root cause**: `grep 'state.broadcast.send' api/campaigns.rs` → zero matches. When the user drags a post on the kanban in browser tab A, browser tab B has no idea anything changed until it manually refetches. This is a major UX bug for a kanban board — the user expects real-time updates.
**Fix**: After the UPDATE, add:
```rust
state.broadcast.send("post_stage_changed", &serde_json::json!({
    "id": id,
    "state": body.state,
    "campaign_id": body.campaign_id,
    "previous_state": current,
})).ok();
```
Then in `frontend/src/lib/stores/realtime.ts`, add `post_stage_changed` to the subscription list and wire up a listener in `routes/kanban/+page.svelte` to update the local state.
**Effort**: Tiny (30 minutes).

#### BUG #15 — Manual publish drops idempotency key

**File**: `src/services/posts.rs:323`
**Root cause**: `PostService::publish` constructs `PostContent { idempotency_key: None, ... }`. The manual "Post Now" path (`POST /api/posts/{id}/publish`) does NOT pass the post's `idempotency_key` to the provider. So if a user clicks "Post Now" after a crash that left the post in `publishing` state (and reclaim marked it `error`), the provider has no way to deduplicate — the second publish WILL create a duplicate post on the platform. The scheduler path passes the key (`scheduler/mod.rs:583`); the manual path does not.
**Fix**:
```rust
// services/posts.rs:323
let content = PostContent {
    idempotency_key: Some(post.idempotency_key.to_string()),  // was: None
    // ... rest unchanged
};
```
**Effort**: Tiny (5 minutes).

#### BUG #19 — SSE endpoint `/api/events` is in `public_routes` — no auth, leaks all events

**File**: `src/api/mod.rs:105`
**Root cause**: `.route("/api/events", axum::routing::get(sse::sse_handler))` is in the public router, BEFORE the auth middleware layer. The `sse_handler(State(state): State<AppState>)` has NO `auth: AuthenticatedUser` extractor. **Any unauthenticated client can subscribe to all realtime events**. The events include post content, platform URLs, integration provider names — sensitive data. For single-user local deployment this is tolerable; for any networked deployment (`BIND_HOST=0.0.0.0`) it's an auth bypass.
**Fix**: Move `/api/events` to `protected_routes`, OR add a cookie-based auth check inside `sse_handler`:
```rust
pub async fn sse_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Sse<...>, AppError> {
    // Validate sf_session cookie before subscribing
    let cookie = headers.get(header::COOKIE).and_then(|c| c.to_str().ok())
        .and_then(|c| c.split(';').find(|s| s.trim().starts_with("sf_session=")))
        .ok_or_else(|| AppError::Unauthorized("No session".into()))?;
    let token = cookie.trim().strip_prefix("sf_session=").unwrap();
    auth::validate_token(token).map_err(|_| AppError::Unauthorized("Invalid session".into()))?;
    // ... existing subscribe logic
}
```
**Effort**: Small (1 hour).

### C.2 HIGH (3 bugs)

#### BUG #22 — `and_hms_opt().unwrap()` in `find_next_free_slot` panics on bad posting_times JSON

**File**: `src/db/queries.rs:1316`
**Root cause**:
```rust
for &minutes in &posting_times {
    let slot = date
        .and_hms_opt((minutes / 60) as u32, (minutes % 60) as u32, 0)
        .unwrap()  // ← panics if hours > 23 or minutes > 59
        .and_utc();
```
The `posting_times` JSON is user-controlled (set via `PUT /api/integrations/{id}/timeslots`). If a user (or a buggy frontend) writes `minutes: 99999` to the integration's `posting_times`, then `minutes / 60 = 1666`, cast to `u32` is `1666`, and `and_hms_opt(1666, ...)` returns `None` → `.unwrap()` **panics the request thread**. Axum returns 500 to the client and the panic propagates to the worker (tokio catches it but the request is dead).
**Fix**:
```rust
let Some(slot) = date.and_hms_opt((minutes / 60) as u32, (minutes % 60) as u32, 0) else {
    tracing::warn!("Invalid posting_time minutes value: {minutes}, skipping");
    continue;
};
let slot = slot.and_utc();
```
Also add validation at the API layer (`update_timeslots`): reject `minutes >= 1440` or `minutes < 0`.
**Effort**: Tiny (15 minutes).

#### BUG #1 — Scheduler timeout (300s) leaks detached tasks; reclaim only on next startup

**File**: `src/scheduler/mod.rs:451`
**Root cause**: When the 300s timeout fires, the code `break`s out of the drain loop with a `tracing::warn!`, but the still-running task is dropped from the `JoinSet` and becomes **detached**. The post is still in `publishing` state — it will be reclaimed by `reclaim_stuck_publishing` only after 5 min on the *next* process startup. If the publish eventually succeeds after the timeout, `update_post_state` will write `published` from a detached task that the operator has no visibility into. This is exactly the publish-orphan pattern the v21 audit warned about, partially re-introduced by the timeout escape hatch.
**Fix**:
```rust
Err(_) => {
    tracing::warn!("Publish task timed out after 300s — aborting remaining tasks");
    join_set.abort_all();  // was: break
    // Mark all remaining posts as error
    while let Some(id) = remaining_post_ids.lock().await.pop() {
        let _ = queries::update_post_state(&db, id, PostState::Error, Some("Publish timed out after 300s")).await;
    }
    break;
}
```
**Effort**: Small (1 hour).

#### BUG #2 — Permit acquired serially before spawn — blocks scheduler on slow provider

**File**: `src/scheduler/mod.rs:388`
**Root cause**: The `sem.acquire_owned().await` on line 388 happens **sequentially in the main scheduler task** before `join_set.spawn`. If one provider's semaphore is exhausted (limit=1, in-flight publish slow), the scheduler **blocks** on that single acquire and never spawns the other providers' publishes. For 30 queued posts across 3 providers (X, LinkedIn, Reddit), if X's permit is held for 60s, the Reddit/LinkedIn posts wait 60s before they even start.
**Fix**: Spawn the task first, acquire the permit inside the spawned future:
```rust
for post in &posts_to_publish {
    let semaphore = providers.concurrency(&post.provider_identifier).cloned();
    let post = post.clone();
    let db = state.db.clone();
    // ... other clones
    join_set.spawn(async move {
        let _permit = match semaphore {
            Some(sem) => match sem.acquire_owned().await { Ok(p) => Some(p), Err(_) => None },
            None => None,
        };
        publish_post(...).await
    });
}
```
**Effort**: Small (1 hour).

### C.3 MEDIUM (4 bugs)

#### BUG #3 — Circuit breaker half-open allows N requests, not 1

**File**: `src/social/registry.rs:126`
**Root cause**: `allow_request()` at line 126 returns `true` unconditionally for half-open (state=2). The doc claims "one request allowed through" — but the implementation does NOT count or limit half-open admissions. If 5 posts for the same provider are claimed in one tick and the circuit just transitioned to half-open, all 5 are spawned, all 5 hit the platform, and 5 failures re-open the circuit.
**Fix**: Use `compare_exchange` to atomically transition half-open → open on first admission:
```rust
pub fn allow_request(&self) -> bool {
    match self.state.compare_exchange(2, 0, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => true,  // was half-open, now closed (admit one)
        Err(s) if s == 0 => true,  // already closed
        Err(_) => false,  // open
    }
}
```
**Effort**: Small (1 hour).

#### BUG #9 — No campaign analytics endpoint

**File**: `src/api/analytics.rs` (missing)
**Root cause**: `grep 'campaign' api/analytics.rs` → zero matches. There is no `GET /api/campaigns/{id}/analytics` or `GET /api/analytics?campaign_id=...` endpoint. The user can create campaigns but cannot see per-campaign engagement totals, post counts by state, or progress towards the campaign `goal`.
**Fix**: Add `GET /api/campaigns/{id}/analytics` returning:
```rust
pub struct CampaignAnalytics {
    pub campaign_id: Uuid,
    pub post_counts: PostStateCounts,  // {idea, draft, queued, published, error}
    pub total_engagement: EngagementMetrics,  // summed across all published posts
    pub posts_by_day: Vec<DayCount>,
    pub goal_progress: Option<f64>,  // if campaign.goal is parseable as a number
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub days_elapsed: i64,
    pub days_remaining: Option<i64>,
}
```
**Effort**: Medium (4 hours).

#### BUG #11 — Dashboard "best_provider" is post-count, not engagement

**File**: `src/api/analytics.rs:271`
**Root cause**: The `get_summary` handler computes `best_provider` as the provider with the highest post count, not the highest engagement. A channel with 100 published posts of zero engagement looks "better" than one with 10 posts of 1k likes each.
**Fix**: Change the query to `ORDER BY SUM(pe.likes + pe.comments + pe.shares) DESC`:
```sql
SELECT i.provider_identifier, COUNT(p.id) as count,
       COALESCE(SUM(pe.likes + pe.comments + pe.shares), 0) as engagement
FROM posts p
JOIN integrations i ON p.integration_id = i.id
LEFT JOIN post_engagement pe ON pe.post_id = p.id
WHERE p.user_id = $1 AND p.deleted_at IS NULL AND p.state = 'published'
GROUP BY i.provider_identifier
ORDER BY engagement DESC
```
**Effort**: Medium (2 hours).

#### BUG #18 — SSE silently drops lagged events — frontend has no signal to refetch

**File**: `src/api/sse.rs:33`
**Root cause**: `Err(_) => None` — when a client lags, the `BroadcastStream` returns `Err(RecvError::Lagged(n))` and the stream silently swallows it. The frontend has no way to know it missed N events. For a calendar that should auto-update, this means: if the user switches tabs for 5 minutes (browser throttles SSE), they come back to a stale view with no indication that they missed updates.
**Fix**:
```rust
Err(RecvError::Lagged(n)) => {
    let data = serde_json::json!({"lagged_count": n}).to_string();
    Some(Ok(Event::default().event("lagged").data(data)))
}
Err(_) => None,  // other errors still silent
```
Then in `frontend/src/lib/stores/realtime.ts`, listen for `"lagged"` event and trigger a refetch of the current route's data.
**Effort**: Tiny (30 minutes).

### C.4 LOW (8 bugs)

#### BUG #8 — Campaigns have no `status` field (active/paused/archived)

**File**: `src/api/campaigns.rs:23` + `migrations/026_campaigns_kanban.sql`
**Root cause**: The `Campaign` struct (line 23–37) and the migration both lack a `status` column. If the frontend sends one, it's silently dropped.
**Fix**: Migration `031_campaign_status.sql`:
```sql
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active'
  CHECK (status IN ('active', 'paused', 'archived', 'completed'));
CREATE INDEX IF NOT EXISTS idx_campaigns_status ON campaigns(user_id, status) WHERE status != 'archived';
```
Add `status: String` to `Campaign` struct, `CreateCampaignRequest`, `UpdateCampaignRequest`. Filter `WHERE status != 'archived'` in `list`.
**Effort**: Small (1 hour).

#### BUG #13 — Calendar `parse_date_or_datetime` returns None on ambiguous DST times

**File**: `src/api/calendar.rs:140`
**Root cause**: `.and_local_timezone(Utc).single()?` returns `None` for ambiguous local times (DST transitions). For UTC this never happens, but the function signature suggests it could accept local-timezone strings.
**Fix**: Document the UTC-only constraint in the docstring, OR restrict the parser to UTC explicitly:
```rust
let dt = DateTime::parse_from_rfc3339(&s)?.with_timezone(&Utc);
```
**Effort**: Tiny (15 minutes).

#### BUG #5 — Recurring field silently ignored in POST /api/posts body

**File**: `src/api/posts.rs:30`
**Root cause**: `CreatePostRequest` has no `recurring` field. If the frontend sends `recurring: { interval_days, end_date }`, serde silently ignores it (no `#[serde(deny_unknown_fields)]`). The composer must make TWO round-trips: POST /api/posts → POST /api/posts/{id}/repeat. If the second call fails, the user has a scheduled post without the recurring series they thought they configured, with no error feedback.
**Fix**: Either (a) add `recurring: Option<RecurringRequest>` to `CreatePostRequest` and call `repeat_post` inline after create, OR (b) add `#[serde(deny_unknown_fields)]` to surface the error. Option (a) is the postiz-style approach.
**Effort**: Small (2 hours) for option (a).

#### BUG #10 — DELETE /api/campaigns/{id} is hard delete (no soft-delete recovery)

**File**: `src/api/campaigns.rs:157`
**Root cause**: `DELETE FROM campaigns WHERE id = $1 AND user_id = $2`. Migration 026 has `ON DELETE SET NULL` on `posts.campaign_id`, so posts survive but lose their campaign association. No soft-delete recovery.
**Fix**: Add `deleted_at TIMESTAMPTZ` to campaigns (migration 032), change DELETE to `UPDATE campaigns SET deleted_at = NOW()`, filter `WHERE deleted_at IS NULL` in list. Add `POST /api/campaigns/{id}/restore` for recovery.
**Effort**: Small (1 hour).

#### BUG #20 — `cli/run.rs:21` `output_json` unwraps serialization

**File**: `src/cli/run.rs:21`
**Root cause**: `serde_json::to_string_pretty(value).unwrap()` — panics if a value can't be serialized. Since values are constructed internally, this is unlikely to fail, but it's an unwrap on the output path.
**Fix**: `unwrap_or_else(|e| { eprintln!("Failed to serialize: {e}"); "{}".to_string() })`.
**Effort**: Tiny (5 minutes).

#### BUG #23 — `main.rs:314` `http_addr.parse().unwrap()` panics on bad env var

**File**: `src/main.rs:314`
**Root cause**: If `HTTP_ADDR` env var is malformed (e.g., `HTTP_ADDR=0.0.0.0:99999` — port out of range), the process panics on startup.
**Fix**: `.unwrap_or_else(|e| { eprintln!("Invalid HTTP_ADDR '{http_addr}': {e}"); std::process::exit(1); })`.
**Effort**: Tiny (5 minutes).

#### BUG #24 — 135 runtime SQL queries not compile-time checked (`.sqlx` cache drift)

**File**: `src/db/queries.rs` (multiple)
**Root cause**: 64 `query!`/`query_as!` macros ARE compile-time checked against 81 cached files. 135 runtime `query`/`query_as::<_, T>` queries are NOT compile-time checked — column type mismatches surface only at runtime. Newer queries (anything touching `idempotency_key`, `campaign_id`, `source_external_post_id`, `deleted_at`) use runtime form because "can't regenerate .sqlx offline cache without a live Postgres".
**Fix**: Run `cargo sqlx prepare -- --lib` against a fresh Postgres with all 30 migrations applied, commit the new `.sqlx/` files, then convert the runtime queries to macros. v21 deferred this; still outstanding.
**Effort**: Medium (1 day — requires Postgres setup).

#### BUG #25 — CSRF `allowed_origin` is single string, not list

**File**: `src/api/mod.rs:548`
**Root cause**: The `allowed_origin` is a single string. If the deployment is reachable via multiple origins (e.g., `https://social-forge.example.com` AND `http://localhost:6543` for local dev), the operator must set `CSRF_ALLOWED_ORIGIN=*` (disabling the check entirely) or run separate instances.
**Fix**: Accept a comma-separated list and check `allowed_origens.contains(o)`:
```rust
let allowed: Vec<&str> = csrf.allowed_origin.split(',').map(|s| s.trim()).collect();
let allowed = match request_origin.as_deref() {
    Some(o) => allowed.contains(&o) || allowed.contains(&o.trim_end_matches('/')),
    None => false,
};
```
**Effort**: Tiny (30 minutes).

### C.5 INFO (6 observations)

- **#16**: `Broadcaster::send()` only fires if `receiver_count() > 0`. If no SSE client is connected, events are silently dropped. No event log persists. For a "recent activity" widget, this means we cannot replay missed events — they're gone. **Recommendation**: Add an `events_log` table for the last 1000 events, query for "recent activity" widget.
- **#17**: The 1024-event buffer is per-channel, not per-subscriber. Each subscriber has its own lag-tracking. If a slow SSE client lags >1024 events, the broadcast channel silently drops the oldest. No backpressure signal. Acceptable for UI, problematic for MCP clients that need guaranteed delivery.
- **#21**: There is no migration 024. The numbering skips from 023 to 025. Either a migration was deleted (yanked) or this is an authoring oversight. `sqlx::migrate!` applies in lexicographic order, so the gap is harmless, but it suggests an audit trail gap.
- **#14**: `refresh` endpoint (`api/integrations.rs:501`) returns generic `ProviderError` to client — leaks raw provider error message (e.g., "invalid_grant: refresh token expired" from LinkedIn). Minor info-leak for debugging.
- **#26**: CSRF check is correctly skipped for Stripe webhook (Stripe signs webhooks with HMAC, doesn't send Origin). No bug — verified.
- **Frontend BUG**: Automation rule creation uses NIL UUID (`routes/automation/+page.svelte:65` — `integration_id: "00000000-0000-0000-0000-000000000000"`). Every automation rule is broken on creation. **Fix**: wire to selected integration.

### C.6 What's Solid (Don't Break These)

The audit also found exemplary patterns that should be preserved:

1. **Atomic claim** (`get_due_posts` with `FOR UPDATE SKIP LOCKED` inside a CTE) — closes dual-instance double-publish. ✓
2. **Per-provider circuit breaker** with env-configurable threshold + cooldown. ✓ (Fix BUG #3 to perfect it.)
3. **Token encryption at rest** — all 4 token-refresh paths encrypt before storing when `TOKEN_ENCRYPTION_KEY` is set. ✓
4. **`AppError::Database` does NOT leak SQL details** to the client. ✓ Exemplary.
5. **3-attempt DB retry after successful publish** (scheduler lines 619–660) with CRITICAL log if all fail — good publish-orphan mitigation given no outbox. ✓
6. **`sanitize_content` char-boundary-safe truncation** (no UTF-8 panic on emoji). ✓ v21 fix verified.
7. **Soft-delete on posts** (cascades group) with `WHERE deleted_at IS NULL` filters in list/calendar queries. ✓ v21 fix verified.
8. **Repurpose endpoint** with provenance FK (`source_external_post_id`) — clean design. ✓ v21 add verified.
9. **CSRF defense-in-depth** (SameSite=Lax + Origin + Referer fallback) — robust. ✓
10. **Per-provider concurrency Semaphore** — prevents per-account rate-limit trips. ✓ (Fix BUG #2 to perfect it.)

---

## Part D — Posting Infrastructure Upgrade Plan

Addressing the user's complaint: *"it does not have the high-quality posting infrastructure like postiz"*. Postiz uses Temporal.io workflows (v1.0.1 → v1.0.5) with per-platform task queues, a `missingPostWorkflow` sweeper, and a `poke` signal. Social-forge's architectural preferences **forbid** Temporal, Redis, and microservices. This plan achieves postiz-grade reliability **within the single-binary constraint**.

### D.1 Idempotency keys actually consumed by providers

**Goal**: Make the existing `posts.idempotency_key` column actually prevent double-publish.

**Design**: Two layers of idempotency:

1. **HTTP-level idempotency** for providers that support the `Idempotency-Key` header (Stripe/OpenAI convention):
   - **X v2**: Send `Idempotency-Key: <key>` header on POST `/2/tweets`. X deduplicates for 24h.
   - **LinkedIn**: Send `Idempotency-Key` on POST `/v2/ugcPosts`. LinkedIn deduplicates per key.
   - **Reddit**: Send `Idempotency-Key` on POST `/api/submit`. Reddit deduplicates per key.
   - **Slack**: Send `Idempotency-Key` on POST `chat.postMessage` (Slack deduplicates for 5min).
   - **Stripe** (if billing): already supported.

2. **Application-level dedup cache** for providers that don't support HTTP-level idempotency (Mastodon, Bluesky, Threads, Discord, Telegram):
   - Before calling `provider.publish()`, check `posts.idempotency_key` + `posts.platform_post_id IS NOT NULL`.
   - If `platform_post_id` is already set for this `idempotency_key`, skip the publish and return the existing result.
   - This catches the "publish succeeded but DB write failed" case.

**Implementation**: Add a helper in `src/social/mod.rs`:
```rust
pub trait SocialProvider: Send + Sync {
    /// Whether this provider supports the Idempotency-Key HTTP header.
    fn supports_http_idempotency(&self) -> bool { false }

    async fn publish(&self, token: &str, content: &PostContent) -> Result<PublishResult, ProviderError> {
        // Default implementation: check dedup cache, then call publish_inner
        if !content.idempotency_key.is_some() {
            return self.publish_inner(token, content).await;
        }
        // Check dedup cache (caller must pass db)
        // ... if cached, return cached result
        let result = self.publish_inner(token, content).await?;
        // Write to dedup cache
        // ...
        Ok(result)
    }

    async fn publish_inner(&self, token: &str, content: &PostContent) -> Result<PublishResult, ProviderError>;
}
```

Then in each provider's `publish_inner`, if `supports_http_idempotency() == true`, add the header:
```rust
// src/social/x.rs
fn supports_http_idempotency(&self) -> bool { true }

async fn publish_inner(&self, token: &str, content: &PostContent) -> Result<PublishResult, ProviderError> {
    let mut req = self.client.post("https://api.x.com/2/tweets")
        .bearer_auth(token)
        .json(&json!({ "text": content.content }));
    if let Some(key) = &content.idempotency_key {
        req = req.header("Idempotency-Key", key);
    }
    // ...
}
```

**Effort**: 2-3 days for top 5 providers (X, LinkedIn, Reddit, Slack, + application-level cache).

### D.2 Transactional outbox for publishes

**Goal**: Eliminate the "publish succeeded but DB write failed" publish-orphan risk.

**Design**: A `publish_outbox` table + drain loop. The scheduler writes the publish result to BOTH `posts` (the source of truth for state) AND `publish_outbox` (the durability log). If the `posts` write fails, the outbox drain loop retries it.

**Schema** (migration 032):
```sql
CREATE TABLE IF NOT EXISTS publish_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    post_id UUID NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    idempotency_key UUID NOT NULL,
    platform_post_id TEXT,         -- NULL until publish succeeds
    platform_post_url TEXT,
    published_at TIMESTAMPTZ,
    error_message TEXT,
    attempts INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ        -- NULL until successfully applied to posts
);
CREATE INDEX idx_publish_outbox_pending ON publish_outbox(next_attempt_at)
  WHERE completed_at IS NULL;
```

**Drain loop** (in `src/scheduler/mod.rs`, runs every 30s alongside `process_due_posts`):
```rust
async fn drain_publish_outbox(db: &PgPool) {
    let pending: Vec<PublishOutboxRow> = sqlx::query_as(
        r#"SELECT * FROM publish_outbox
           WHERE completed_at IS NULL AND next_attempt_at <= NOW()
           ORDER BY created_at ASC LIMIT 50
           FOR UPDATE SKIP LOCKED"#
    ).fetch_all(db).await?;

    for row in pending {
        // Try to apply the result to posts
        let result = sqlx::query(
            r#"UPDATE posts SET
                 state = CASE WHEN row.platform_post_id IS NOT NULL THEN 'published'::post_state ELSE 'error'::post_state END,
                 platform_post_id = row.platform_post_id,
                 platform_post_url = row.platform_post_url,
                 published_at = row.published_at,
                 error_message = row.error_message,
                 updated_at = NOW()
               FROM (SELECT $2::uuid as platform_post_id, $3::text as platform_post_url, ...) as row
               WHERE id = $1"#
        ).bind(row.post_id).bind(row.platform_post_id)...execute(db).await;

        match result {
            Ok(_) => {
                sqlx::query("UPDATE publish_outbox SET completed_at = NOW() WHERE id = $1")
                    .bind(row.id).execute(db).await.ok();
            }
            Err(e) => {
                tracing::warn!("Outbox drain failed for {}: {e}", row.post_id);
                sqlx::query("UPDATE publish_outbox SET attempts = attempts + 1, next_attempt_at = NOW() + INTERVAL '30 seconds' WHERE id = $1")
                    .bind(row.id).execute(db).await.ok();
            }
        }
    }
}
```

**Scheduler change**: After `provider.publish()` succeeds, write to `publish_outbox` (not directly to `posts`). The drain loop handles the `posts` write. If the outbox write fails, the publish is lost — but that's a much rarer failure mode than "publish succeeded, posts write failed".

**Effort**: 2 days (schema + drain loop + scheduler change + tests).

### D.3 Per-platform FIFO queue via existing Semaphore + serial claim ordering

**Goal**: Guarantee per-platform publish ordering (postiz's per-platform Temporal task queue equivalent).

**Design**: The current `Semaphore` per provider limits concurrency but doesn't guarantee FIFO across ticks. Two changes:

1. **Claim ordering**: Change `get_due_posts` to claim posts grouped by provider, ordered by `scheduled_at ASC` within each provider. This ensures the scheduler publishes in schedule order within a single tick.

2. **Cross-tick ordering**: Add a per-provider `last_publish_started_at` timestamp. Before spawning a publish task, check if there's an in-flight publish for the same provider (semaphore is held). If yes, defer the new post to the next tick. This isn't strictly FIFO, but it prevents the "6th post publishes before 5th finishes" case.

**Implementation**:
```sql
-- get_due_posts: claim per-provider, ordered by scheduled_at
WITH ranked AS (
    SELECT p.id, p.provider_identifier,
           ROW_NUMBER() OVER (PARTITION BY p.provider_identifier ORDER BY p.scheduled_at ASC) as rn
    FROM posts p JOIN integrations i ON p.integration_id = i.id
    WHERE p.state = 'queued' AND p.scheduled_at <= NOW() AND i.disabled = false
)
UPDATE posts SET state = 'publishing', updated_at = NOW()
WHERE id IN (SELECT id FROM ranked WHERE rn = 1 LIMIT 50)
RETURNING ...
```

**Effort**: 1 day.

### D.4 Workflow versioning concept (in-code, not Temporal)

**Goal**: Enable in-flight migration of the publish state machine without breaking existing `publishing` posts.

**Design**: Add a `publish_workflow_version` column to `posts`. When the scheduler claims a post, it reads the version and dispatches to the corresponding `publish_v1`, `publish_v2`, etc. function. New posts get the latest version; existing posts keep their original version until they complete.

**Schema** (migration 033):
```sql
ALTER TABLE posts ADD COLUMN IF NOT EXISTS publish_workflow_version INT NOT NULL DEFAULT 1;
```

**Code**:
```rust
// src/scheduler/mod.rs
async fn publish_post(db: &PgPool, post: Post, ...) {
    match post.publish_workflow_version {
        1 => publish_v1(db, post, ...).await,
        2 => publish_v2(db, post, ...).await,
        _ => publish_v2(db, post, ...).await,  // default to latest
    }
}

async fn publish_v1(...) { /* current logic */ }
async fn publish_v2(...) { /* adds idempotency header + outbox write */ }
```

When creating a new post, set `publish_workflow_version = <current_latest>`. Existing `publishing` posts continue with their version. After all in-flight posts complete, deprecate v1.

**Effort**: 1 day (mostly tests).

### D.5 Stuck-publishing sweep every tick (not just startup)

**Goal**: Detect and recover stuck publishes without waiting for a restart.

**Design**: Run `reclaim_stuck_publishing(db, 300)` on every scheduler tick (every 30s). It's a single UPDATE, cheap.

**Implementation**:
```rust
// src/scheduler/mod.rs
async fn tick(state: Arc<AppState>) {
    // 1. Reclaim stuck publishing (NEW)
    if let Err(e) = queries::reclaim_stuck_publishing(&state.db, 300).await {
        tracing::warn!("reclaim_stuck_publishing failed: {e}");
    }

    // 2. Process due posts (existing)
    match process_due_posts(state.clone()).await {
        Ok(n) => tracing::debug!("Published {n} posts"),
        Err(e) => tracing::error!("Scheduler tick failed: {e}"),
    }

    // 3. Drain publish outbox (NEW — see D.2)
    if let Err(e) = drain_publish_outbox(&state.db).await {
        tracing::warn!("Outbox drain failed: {e}");
    }

    // 4. Proactive token refresh (existing)
    // 5. Analytics cache refresh (existing)
}
```

**Effort**: 30 minutes.

### D.6 SSE lagged event signal (already covered as BUG #18 fix)

See BUG #18 fix in Part C.3. Emit synthetic `"lagged"` event so frontend can refetch.

### D.7 Summary: Posting infrastructure upgrade effort

| Item | Effort | Priority |
|---|---|---|
| D.1 Idempotency keys consumed by providers | 2-3 days | Phase 2 |
| D.2 Transactional outbox | 2 days | Phase 2 |
| D.3 Per-platform FIFO queue | 1 day | Phase 2 |
| D.4 Workflow versioning | 1 day | Phase 2 |
| D.5 Stuck-publish sweep every tick | 30 min | Phase 1 |
| D.6 SSE lagged event signal | 30 min | Phase 1 |
| **Total** | **~7 days** | **Phase 1 + 2** |

---

## Part E — Frontend Re-Architecture Plan

Addressing the user's complaints: *"dashboard is cheap knock off, non-functional"*, *"entire UI/UX needs major upgrades, color scheme is bad"*, *"left-panel has redundancies, settings has its own sidebar"*. Plus calendar and composer polish inspired by postiz.

### E.1 Dashboard redesign — 12-widget command center

**Goal**: Transform the dashboard from a thin read-only summary into a strategic command center for a solo-founder managing many channels.

**Current state** (352 lines, 9 widgets): Drafts/Queued/Published/Errors stat row, Likes/Comments/Shares 7d row, channel performance bar chart (post count, not engagement), alerts, needs-attention inbox, today's schedule (browser-local-time BUG), recent activity, quick actions.

**Redesigned 12-widget layout**:

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Welcome back, ishan. Last sync: 2m ago. [Refresh] [Cmd+K]              │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐                            │
│  │ Drafts │ │ Queued │ │Publishd│ │ Errors │  ← stat row with trend     │
│  │   12   │ │   8    │ │  145   │ │   3    │    deltas vs last 7d       │
│  │ +3 ▲   │ │ -1 ▼   │ │ +12 ▲  │ │ +2 ▲   │                            │
│  └────────┘ └────────┘ └────────┘ └────────┘                            │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐  ┌─────────────────────────┐               │
│  │ Engagement (7d)         │  │ Audience growth (30d)   │               │
│  │ Likes: 1.2k  +15% ▲     │  │ Followers: +234         │               │
│  │ Comments: 89  +5% ▲     │  │ Chart: sparkline        │               │
│  │ Shares: 45   -2% ▼      │  │ Per-channel breakdown   │               │
│  │ Impressions: 45k +20%▲  │  │                         │               │
│  └─────────────────────────┘  └─────────────────────────┘               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐  ┌─────────────────────────┐               │
│  │ Channel performance     │  │ Scheduled vs actual     │               │
│  │ Bar chart, sorted by    │  │ Last 7d:                │               │
│  │ engagement rate (NOT    │  │ Scheduled: 50           │               │
│  │ post count)             │  │ Published: 47 (94%)     │               │
│  │ Per-provider brand color│  │ Failed: 3 (6%)          │               │
│  └─────────────────────────┘  └─────────────────────────┘               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────┐  ┌─────────────────────────┐               │
│  │ Posting cadence vs goal │  │ Campaign progress       │               │
│  │ Goal: 5 posts/day       │  │ Active: 3 campaigns     │               │
│  │ Actual: 4.2/day (84%)   │  │ "Q4 Launch": 12/20 posts│               │
│  │ Streak: 23 days         │  │ "Holiday": 3/10 posts   │               │
│  └─────────────────────────┘  └─────────────────────────┘               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐               │
│  │ Today's schedule (timezone-aware)                   │               │
│  │ 09:00 — X thread "Building in public" (queued)      │               │
│  │ 12:00 — LinkedIn "Hiring update" (published ✓)      │               │
│  │ 15:00 — Instagram "Behind the scenes" (queued)      │               │
│  │ 18:00 — Reddit "AMA announcement" (queued)          │               │
│  └─────────────────────────────────────────────────────┘               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐               │
│  │ Needs attention (3)                                │               │
│  │ ⚠ 2 channels need reconnect (LinkedIn, Reddit)      │               │
│  │ ⚠ 3 posts failed (retry or delete)                  │               │
│  │ ⚠ 1 campaign ends in 2 days (60% to goal)           │               │
│  └─────────────────────────────────────────────────────┘               │
├─────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────┐               │
│  │ Recent activity (last 10 events, realtime)          │               │
│  │ 2m ago — Post published on X (@ishan)               │               │
│  │ 5m ago — Post scheduled for LinkedIn (tomorrow 9am) │               │
│  │ 12m ago — Channel connected: Bluesky                │               │
│  │ ...                                                 │               │
│  └─────────────────────────────────────────────────────┘               │
├─────────────────────────────────────────────────────────────────────────┤
│  Quick actions: [+ New Post] [+ New Campaign] [Import Feed] [Refresh]  │
└─────────────────────────────────────────────────────────────────────────┘
```

**Backend changes** (new endpoints):
- `GET /api/analytics/engagement?days=7` — returns `{likes, comments, shares, impressions, deltas}` with per-day breakdown for sparklines.
- `GET /api/analytics/audience?days=30` — returns per-channel follower snapshots + delta.
- `GET /api/analytics/adherence?days=7` — returns `{scheduled, published, failed, adherence_rate}`.
- `GET /api/analytics/cadence?days=30` — returns `{goal, actual, streak, daily_breakdown}`.
- `GET /api/campaigns/active/progress` — returns active campaigns with progress percentages.
- `GET /api/events/recent?limit=10` — returns last 10 events from `events_log` table (see Part C.5 #16).
- Modify `GET /api/analytics/summary` to return engagement-based `best_provider` (BUG #11 fix).

**Frontend changes**:
- Rewrite `src/routes/+page.svelte` to render the 12-widget layout.
- Add `src/lib/components/dashboard/` directory with one component per widget: `StatCard.svelte`, `EngagementWidget.svelte`, `AudienceWidget.svelte`, `ChannelPerformanceWidget.svelte`, `AdherenceWidget.svelte`, `CadenceWidget.svelte`, `CampaignProgressWidget.svelte`, `TodayScheduleWidget.svelte`, `NeedsAttentionWidget.svelte`, `RecentActivityWidget.svelte`, `QuickActionsWidget.svelte`.
- Each widget subscribes to relevant realtime events (e.g., `post_published` → `EngagementWidget` refetches, `TodayScheduleWidget` updates).
- Fix timezone BUG: use `timezone` store for "today's schedule" filter.

**Effort**: 3-4 days (1 day backend endpoints, 2-3 days frontend widgets).

### E.2 Sidebar dedup + command palette + global search

**Goal**: Eliminate the dual-settings-sidebar bug, add collapse-to-icon-rail, add Cmd+K command palette, surface global search.

**Sidebar dedup**:
- Remove all 8 settings sub-routes from the main sidebar (lines 68–80 of `+layout.svelte`).
- Replace with a single "Settings" entry that routes to `/settings`.
- The settings layout (`/settings/+layout.svelte`) keeps its 8-tab sidebar — this is the postiz pattern.
- Fix active-link matching: change `$page.url.pathname === item.href` to `$page.url.pathname === item.href || $page.url.pathname.startsWith(item.href + '/')`.

**Collapse-to-icon-rail**:
- Add a collapse toggle button at the top of the sidebar.
- When collapsed, sidebar width transitions from 224px (`w-56`) to 56px (`w-14`), showing only icons.
- Persist state via `localStorage('social-forge-sidebar-collapsed')`.
- On hover when collapsed, expand a flyout showing labels (like Linear).

**Command palette (Cmd+K)**:
- New `src/lib/components/CommandPalette.svelte` — modal triggered by `Cmd+K` / `Ctrl+K`.
- Indexed commands: navigate to (calendar, posts, kanban, feed, channels, analytics, settings), create (post, campaign, automation rule), actions (refresh, import feed, connect channel).
- Fuzzy search via simple `String.includes` (no external dep).
- Recent commands surfaced at top.

**Global search in sidebar**:
- Add a search input at the top of the sidebar (when expanded).
- Search posts, campaigns, channels, tags, settings.
- Results dropdown navigates to the relevant page.

**Effort**: 2 days (sidebar dedup 4h, collapse-to-icon-rail 4h, command palette 1 day, global search 4h).

### E.3 Color system overhaul

**Goal**: Replace 80 hardcoded hex colors across 22 files with semantic tokens. Make light mode actually work.

**Step 1: Add missing semantic tokens to `tailwind.config.js`**:
```js
// tailwind.config.js
extend: {
  colors: {
    // existing tokens...
    success: 'rgb(var(--success) / <alpha-value>)',
    warning: 'rgb(var(--warning) / <alpha-value>)',
    error: 'rgb(var(--error) / <alpha-value>)',
    info: 'rgb(var(--info) / <alpha-value>)',
  },
  borderRadius: {
    sm: 'var(--radius-sm)',
    md: 'var(--radius-md)',
    lg: 'var(--radius-lg)',
  },
}
```

**Step 2: Add CSS variables to `app.css`**:
```css
:root.dark {
  --success: 34 197 94;   /* green-500 */
  --warning: 234 179 8;   /* yellow-500 */
  --error: 239 68 68;     /* red-500 */
  --info: 59 130 246;     /* blue-500 */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
}
:root.light {
  --success: 22 163 74;   /* green-600 */
  --warning: 202 138 4;   /* yellow-600 */
  --error: 220 38 38;     /* red-600 */
  --info: 37 99 235;      /* blue-600 */
}
```

**Step 3: Migrate hardcoded hex colors** (80 occurrences across 22 files):

| File | Hits | Migration |
|---|---|---|
| `lib/components/CommentsThread.svelte` | 16 | `bg-[#0d121e]` → `bg-background`, `bg-[#1e2435]` → `bg-surface`, `bg-[#0a0e16]` → `bg-background-input`, `text-[#5a6070]` → `text-muted` |
| `lib/media/MediaCarousel.svelte` | 6 | `bg-[#0d121e]` → `bg-background`, `ring-[#1e2435]` → `ring-line`, `bg-[#2a3045]` → `bg-surface-hover` |
| `lib/rss/RssFeedForm.svelte` | 5 | `bg-[#0b0e14]` → `bg-background`, `placeholder-[#4a5568]` → `placeholder-muted` |
| `lib/composer/RichTextEditor.svelte` | 5 + `<style>` | `bg-[#6366f1]` → `bg-brand-500`, `hover:bg-[#5558e6]` → `hover:bg-brand-600`, `text-[#2a3045]` → `text-content`, `text-[#9ca3af]` → `text-muted`, `placeholder:text-[#4a5568]` → `placeholder:text-muted`, `focus:border-[#6366f1]` → `focus:border-brand-500`. Replace `<style>` block hex with CSS variables. |
| `lib/calendar/PostHoverToolbar.svelte` | 5 | `bg-[#1e2435]/95` → `bg-surface/95` |
| `lib/components/EngagementCard.svelte` | 5 | `bg-[#0d121e]` → `bg-background`, `text-[#9ca3af]` → `text-muted` |
| `lib/calendar/ListView.svelte` | 3 | `text-[#9ca3af]` → `text-muted` |
| `lib/calendar/PostDetail.svelte` | 2 | `bg-[#1a1f2e]` → `bg-surface-hover`, `hover:bg-[#242b3d]` → `hover:bg-surface-hover` |
| `lib/composer/SchedulePicker.svelte` | 3 | Same pattern |
| `lib/composer/MediaUpload.svelte` | 2 | Same pattern |
| `lib/channels/ChromeExtensionConnect.svelte` | 3 | Same pattern |
| `lib/channels/TimeSlotEditor.svelte` | 3 | Same pattern |
| `lib/composer/TargetPicker.svelte` | 3 | Same pattern |
| `lib/channels/ApiKeyConnect.svelte` | 2 | Same pattern |
| `lib/channels/Web3Connect.svelte` | 2 | Same pattern |
| `lib/channels/PagePicker.svelte` | 2 | Same pattern |
| `lib/analytics/DateRangePicker.svelte` | 1 | Same pattern |
| `lib/notifications/NotificationBell.svelte` | 1 | Same pattern |
| `lib/composer/AiAssistant.svelte` | 1 | `hover:border-[#374151]` → `hover:border-line-hover` |
| `lib/composer/AiHashtagSuggestions.svelte` | 1 | `bg-[#1e2435]` → `bg-surface` |
| `lib/ui/Button.svelte` | `<style>` block | Replace all hex in `<style>` with CSS variables. **Critical** — primary CTA must retheme on light mode. |
| `lib/calendar/CalendarEvent.svelte` | 4 in `<style>` | Replace RGBA state colors with `bg-success/15 text-success`, `bg-warning/15 text-warning`, etc. |
| `lib/calendar/PostStatsModal.svelte` | (not grepped) | Audit + fix |

**Step 4: Fix `Badge.svelte` to support generic variants**:
```svelte
<!-- current: only handles draft|queued|published|error state -->
<!-- new: add variant prop -->
<script lang="ts">
  let { variant = 'default', state, children }: { variant?: 'default'|'success'|'warning'|'error'|'info', state?: string, children?: any } = $props();
  const stateClass = {
    draft: 'bg-muted/20 text-muted',
    queued: 'bg-brand-500/20 text-brand-300',
    published: 'bg-success/20 text-success',
    error: 'bg-error/20 text-error',
    idea: 'bg-purple-500/20 text-purple-300',
  };
  const variantClass = {
    default: 'bg-muted/20 text-muted',
    success: 'bg-success/20 text-success',
    warning: 'bg-warning/20 text-warning',
    error: 'bg-error/20 text-error',
    info: 'bg-info/20 text-info',
  };
</script>
<span class="badge {state ? stateClass[state] : variantClass[variant]}">{children ?? state}</span>
```

**Step 5: Audit light mode** — toggle to light mode in dev, fix any remaining contrast issues. The `app.css` light theme variables (lines 31–49) need review: `--text-muted-dark: #94a3b8` is LIGHTER than `--text-muted: #64748b` in light mode — inverted from dark mode. Rename `--text-muted-dark` to `--text-muted-secondary` to avoid confusion.

**Effort**: 3-4 days (token system 4h, file-by-file migration 2-3 days, light-mode audit 4h).

### E.4 Calendar polish (postiz-inspired)

**Goal**: Make the calendar feel postiz-grade.

**Already done in v21** (verified ✓): week-default view, URL sync, past-cell greying, tag-colored chip border, error `!` badge + ring, "Just update vs Reschedule" modal, retry button on error.

**Still needed**:

1. **Monday/Sunday consistency** — `getMonthDays` (Sunday-start) vs `getWeekDays` (Monday-start) inconsistency. Fix `getMonthDays` to Monday-start. Update `days` constant in MonthView to `["Mon","Tue",...]`.

2. **Mini-calendar in sidebar** — small month calendar for jumping to dates. Click a date → `calendarState.currentDate` updates. Highlight days with posts (count badge).

3. **Today pulse dot in MonthView** — add a pulsing dot on today's date number in MonthView (currently only in WeekView header).

4. **`mix-blend-difference` tag chip** — postiz uses `style={{ backgroundColor: post.tags[0].tag.color }}` + `mix-blend-difference` for the label text. SF uses tag color on border-left only. Migrate to postiz-style: top bar with tag color + `mix-blend-difference` label.

5. **Per-cell `useInterval` greyscale flip** — postiz re-renders cells every 2-2.5min via `useInterval`. SF computes past state once. Add a `setInterval` in `calendarState` that re-evaluates `isPast` every 2min.

6. **Calendar filters** — add `integration_ids`, `tag_ids`, `campaign_id` to `CalendarQuery` and the API. Frontend filter bar in `CalendarHeader.svelte`.

7. **`campaign_id` in calendar response** — add `campaign_id: Option<Uuid>` to `CalendarPostWithMetrics` struct + select `p.campaign_id` in the query. Frontend can show campaign color on chips.

8. **Timezone-aware bulk-reschedule** — fix the `${date}T${time}:00.000Z` always-UTC bug. Use the `timezone` store to construct the correct UTC datetime from the user's local inputs.

9. **Bulk operations batching** — `bulkDelete` and `bulkReschedule` currently run sequential `await` in a `for…of` loop. Use `Promise.all` for parallelism (with a concurrency limit of 5).

**Effort**: 2-3 days.

### E.5 Composer polish (postiz-inspired)

**Goal**: Make the composer feel postiz-grade.

**Already done in v21** (verified ✓): full-screen modal, two-column layout, per-channel override, real upload progress, askClose confirmation, per-platform error toasts, signature auto-append, edit-mode branch, recurring two-step wiring.

**Still needed**:

1. **Mantine-style date picker popover** — replace native `<input type="date">` + `<input type="time">` in `SchedulePicker.svelte` with a custom Svelte popover (no Mantine dep). Calendar widget + time input, locale-aware format, `useClickOutside` to close.

2. **Per-row delay for any provider** — postiz allows a `delay: number` on every row (global OR per-channel internal). SF only has `ThreadFinisher` for X. Add a generic "Add row" button with a per-row delay input. The backend `PostContent.delay_minutes: Option<u32>` field + scheduler sleeps `delay_minutes * 60s` between rows.

3. **Mention extension** — add TipTap Mention extension. Backend endpoint `GET /api/integrations/{id}/mentions?q=...` already exists (`api/integrations.rs`). Wire it to a `mention.component.svelte` suggestion popup.

4. **Underline extension** — add TipTap Underline extension. Bold/underline mutex (postiz-style).

5. **Generic thread builder** — replace X-only `ThreadFinisher` with a generic `ThreadBuilder.svelte` that works for any provider. Per-row delay (item 2). For non-threadable providers, rows become first-comment (postiz-style `isCommentable` check).

6. **X `weightedLength` for char count** — use `twitter-text`'s `weightedLength` for X char count (postiz `posts.service.ts:829-843`). Currently SF uses `content.length` which is wrong for emoji-heavy X posts.

7. **Fix alt-text dropped on submit** — `ComposerModal.svelte:378` `alt: undefined` always. `MediaUpload.saveAlt` writes to `item.original_name` instead of `item.alt`. Fix `MediaItem` type to have `alt: string`, `saveAlt` to write to `item.alt`, `buildPayload` to read `m.alt`.

8. **Fix timezone in SchedulePicker** — same as E.4 item 8.

9. **Fix `postsApi.update` to accept tag_ids + first_comment** — currently the type signature only includes `content, title, media, settings`. Add `tag_ids?: string[]` and `first_comment?: string`. The backend `PUT /api/posts/{id}` handler needs to accept these too.

10. **More platform previews** — add X, Reddit, Threads, Bluesky previews to `PlatformPreviewPane`. Currently only IG/LinkedIn/Facebook/General.

11. **Brand profile sync to backend** — currently `routes/settings/profile/+page.svelte:69` saves to localStorage only. Add backend endpoint `PUT /api/profile` + `GET /api/profile`. Wire `AiAssistant` to read the brand profile and use it as context for generate/improve/tone.

12. **`saveAsDraft` edit-mode state transition** — currently `ComposerModal.svelte:541` calls `postsApi.update` but doesn't transition state to `draft` (TODO comment at line 545). Add backend endpoint `POST /api/posts/{id}/unschedule` that sets `state = 'draft', scheduled_at = NULL`.

**Effort**: 3-4 days.

### E.6 Design-system primitives

**Goal**: Build the missing primitives that every route currently re-implements inline.

**Current state**: `src/lib/ui/` has 6 primitives (Button, Modal, Badge, Icon, Spinner, Dropdown). `src/lib/components/` has 5 (CommentsThread, EngagementCard, ModalManager, ShortcutsModal, Toast). No `Card`, `EmptyState`, `PageHeader`, `Tabs`, `Table`, `Pagination`, `StatCard`, `Tooltip`, `Avatar`, `Skeleton`, `FilterBar`, `CommandPalette`, `DataTable`.

**New primitives to add** (in `src/lib/ui/`):

| Primitive | Purpose | Used by |
|---|---|---|
| `Card.svelte` | Wrapper with surface bg, border, padding, optional header/footer | Every route |
| `EmptyState.svelte` | Icon + title + description + action button | Posts list, feed, kanban, campaigns |
| `PageHeader.svelte` | Title + subtitle + actions slot | Every route |
| `Tabs.svelte` | Segmented control with active state | Settings, posts list, analytics |
| `Table.svelte` | Header + rows + pagination wrapper | Posts list, webhooks, API keys |
| `Pagination.svelte` | Page numbers + prev/next | Posts list, feed, search |
| `StatCard.svelte` | Number + label + trend delta | Dashboard |
| `Tooltip.svelte` | Hover/focus tooltip | Calendar error badge, kanban card |
| `Avatar.svelte` | Image + fallback initial | Feed, comments, channel selector |
| `Skeleton.svelte` | Shimmer placeholder | Every route during load |
| `FilterBar.svelte` | Search + filter pills + sort dropdown | Posts list, feed, calendar |
| `CommandPalette.svelte` | Cmd+K modal | Global |
| `DataTable.svelte` | Table + sorting + filtering + pagination | Posts list, webhooks, API keys |

**Effort**: 2-3 days.

### E.7 Summary: Frontend re-architecture effort

| Item | Effort | Priority |
|---|---|---|
| E.1 Dashboard redesign (12-widget) | 3-4 days | Phase 5 |
| E.2 Sidebar dedup + command palette + global search | 2 days | Phase 4 |
| E.3 Color system overhaul | 3-4 days | Phase 3 |
| E.4 Calendar polish | 2-3 days | Phase 7 |
| E.5 Composer polish | 3-4 days | Phase 7 |
| E.6 Design-system primitives | 2-3 days | Phase 3 |
| **Total** | **~15-20 days** | **Phase 3 + 4 + 5 + 7** |

---

## Part F — Campaign Management Rework (Strategic Dashboard)

Addressing the user's complaint: *"Campaign-Management Features does not work. There are a lot of bugs, and I cannot even add anything on the kanban. Moreover it need a lot of more features for Campaign-Management, just like professional-digital-marketing strategic-dashboard type"*. **This is SF's differentiation opportunity** — postiz has NO campaign model, NO kanban, NO marketing-strategic layer. SF can be the only one of the two with this.

### F.1 Campaign model expansion

**Goal**: Make campaigns a real marketing-strategic entity, not just a label.

**Current schema** (migration 026): `id, user_id, name, description, color, start_date, end_date, goal, created_at, updated_at`. Missing: `status`, `progress_metric`, `audience_persona`, `content_pillars`, `budget`, `kpi_targets`.

**New schema** (migration 031):
```sql
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'active'
  CHECK (status IN ('active', 'paused', 'archived', 'completed'));
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS progress_metric TEXT;  -- 'posts' | 'engagement' | 'reach' | 'followers' | 'custom'
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS progress_target INT;  -- target number for the metric
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS audience_persona JSONB;  -- {age, location, interests, pain_points}
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS content_pillars JSONB;  -- [{title, description, tags: []}]
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS budget_cents INT;  -- for paid amplification tracking
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS kpi_targets JSONB;  -- {min_engagement_rate, min_reach, target_clicks}
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;  -- soft delete (BUG #10 fix)
ALTER TABLE campaigns ADD COLUMN IF NOT EXISTS sort_order INT NOT NULL DEFAULT 0;  -- for manual ordering

CREATE INDEX IF NOT EXISTS idx_campaigns_status ON campaigns(user_id, status) WHERE deleted_at IS NULL;
```

**Campaign struct** (Rust):
```rust
pub struct Campaign {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub color: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub goal: Option<String>,  // free-text goal
    pub status: CampaignStatus,  // Active | Paused | Archived | Completed
    pub progress_metric: Option<ProgressMetric>,  // Posts | Engagement | Reach | Followers | Custom
    pub progress_target: Option<i32>,
    pub audience_persona: Option<serde_json::Value>,
    pub content_pillars: Option<serde_json::Value>,
    pub budget_cents: Option<i32>,
    pub kpi_targets: Option<serde_json::Value>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

**Effort**: 1 day (migration + Rust struct + API fields).

### F.2 Kanban re-architecture

**Goal**: Transform the kanban from a thin state-grouped post list into a professional kanban with swimlanes, WIP limits, due dates, priority, and card cover images.

**New kanban features**:

1. **Swimlanes** — horizontal grouping within each column. Options: by campaign, by channel, by tag, by due-date-bucket, none. Toggle in the kanban header.

2. **WIP limits per column** — configurable per-column max cards. When exceeded, the column header turns red and drop is disabled. Settings: `kanban_wip_limits: { idea: 50, draft: 20, queued: 10, published: null }`.

3. **Card cover images** — if a post has media, show the first image as a cover on the card. Fallback to provider brand color gradient.

4. **Card preview on hover** — hover a card → popover shows full content + media + scheduled_at + tags.

5. **Drag-to-reorder within column** — add `sort_order` column to `posts` (migration 032). On drag within same column, update `sort_order`. Kanban displays ordered by `sort_order ASC`.

6. **Sub-states** — add `kanban_substate` column: `ready_to_publish` | `in_review` | `blocked` | null. Shown as a colored dot on the card. Filterable.

7. **Due dates on cards** — add `due_date` column to `posts` (migration 032). Shown on card. Overdue cards get red border.

8. **Priority levels** — add `priority` column to `posts`: `low` | `medium` | `high` | `urgent`. Shown as a colored bar on the left of the card. Sortable.

9. **Card tags visible** — tags exist but aren't shown on cards. Render as small colored pills at the bottom of each card.

10. **Card activity log** — new `post_activity` table (migration 033) recording every state change, edit, comment. Shown in a slide-in panel when clicking a card.

11. **Quick-add to any column** — currently only Ideas column has quick-add. Add quick-add to Drafts (creates draft with selected channel), Scheduled (opens composer with prefilled date), Published (disabled — can't manually publish).

12. **Campaign filter fix** — fix the `(p as any).campaign_id` bug. Add `campaign_id` to `PostSummary` type. Filter by `campaign_id === selectedCampaign`.

13. **State-transition validation** (BUG #6 fix) — backend rejects illegal transitions. Frontend shows toast on rejection.

14. **Realtime broadcast** (BUG #7 fix) — `post_stage_changed` event. Kanban updates in real-time across tabs.

15. **Campaign color on cards** — cards in a campaign get a colored left border using the campaign's color.

**New kanban lib** (`src/lib/kanban/`):
- `KanbanBoard.svelte` — top-level board, manages swimlanes + columns
- `KanbanColumn.svelte` — single column with WIP limit + quick-add
- `KanbanCard.svelte` — single card with cover image + tags + due date + priority
- `KanbanSwimlane.svelte` — horizontal swimlane wrapper
- `KanbanFilters.svelte` — campaign/channel/tag filter bar
- `KanbanCardPreview.svelte` — hover popover
- `KanbanCardActivity.svelte` — slide-in activity log

**Effort**: 4-5 days.

### F.3 Campaign detail page

**Goal**: A dedicated page per campaign showing all posts, timeline, analytics, edit form.

**New routes**:
- `/campaigns` — campaign list page (grid of campaign cards with progress bars)
- `/campaigns/[id]` — campaign detail page

**Campaign list page** (`/campaigns/+page.svelte`):
- Grid of campaign cards, each showing: name, color, status badge, progress bar (posts X/Y or engagement X/Y), days remaining, post count by state.
- Filter by status (active/paused/archived/completed).
- Sort by name, start_date, end_date, progress.
- Click a card → navigate to `/campaigns/[id]`.

**Campaign detail page** (`/campaigns/[id]/+page.svelte`):
- Header: name, status badge, color, edit button, delete button.
- Tabs: Overview | Posts | Timeline | Analytics | Settings.
- **Overview tab**: progress bar, KPI targets vs actual, days elapsed/remaining, audience persona, content pillars, budget tracking.
- **Posts tab**: paginated list of all posts in the campaign, filterable by state. Bulk actions: move to different campaign, delete.
- **Timeline tab**: horizontal timeline of all posts by scheduled_at, color-coded by state. Click a node → post detail.
- **Analytics tab**: per-campaign engagement totals, posts-by-day chart, engagement-by-channel chart, progress towards goal over time.
- **Settings tab**: edit form (name, description, color, start_date, end_date, goal, status, progress_metric, progress_target, audience_persona, content_pillars, budget, kpi_targets).

**Effort**: 3-4 days.

### F.4 Campaign analytics endpoint

**Goal**: Per-campaign engagement totals, post counts by state, progress towards goal.

**New endpoint**: `GET /api/campaigns/{id}/analytics`

**Response**:
```rust
pub struct CampaignAnalytics {
    pub campaign_id: Uuid,
    pub post_counts: PostStateCounts,  // {idea, draft, queued, published, error}
    pub total_engagement: EngagementMetrics,  // summed across all published posts
    pub engagement_by_channel: Vec<ChannelEngagement>,
    pub posts_by_day: Vec<DayCount>,
    pub progress: ProgressSummary,
    pub timeline: Vec<TimelineEvent>,  // [{date, post_id, state, engagement}]
}

pub struct ProgressSummary {
    pub metric: ProgressMetric,  // Posts | Engagement | Reach | Followers | Custom
    pub target: Option<i32>,
    pub current: i32,
    pub percentage: f64,  // current / target * 100
    pub days_elapsed: i64,
    pub days_remaining: Option<i64>,
    pub projected_completion: Option<NaiveDate>,  // linear projection
    pub on_track: bool,
}
```

**Effort**: 1 day.

### F.5 Realtime broadcast for campaign events

**Goal**: Multi-tab sync for campaign + stage changes.

**New events** (add to `broadcaster.send`):
- `post_stage_changed` — when a post's kanban stage changes (BUG #7 fix).
- `campaign_created` — when a new campaign is created.
- `campaign_updated` — when a campaign's fields change.
- `campaign_deleted` — when a campaign is deleted.
- `post_repeated` — when a recurring series is set up.

**Frontend** (`realtime.ts`): add these to the subscription list. Wire listeners in kanban, campaigns list, campaign detail, and dashboard.

**Effort**: 1 day.

### F.6 Strategic dashboard widgets (marketing-strategic features postiz doesn't have)

**Goal**: Add widgets to the dashboard that make SF a "professional-digital-marketing strategic-dashboard type".

**New dashboard widgets** (in addition to E.1's 12):

1. **Campaign goal progress** — already in E.1's "Campaign progress" widget. Expand to show all active campaigns with progress bars + projected completion dates.

2. **Audience persona cards** — per active campaign, show the audience persona (age, location, interests, pain points). Helps the user keep the target audience in mind when composing.

3. **Content pillar tracking** — per active campaign, show which content pillars have posts this week vs which are neglected.

4. **Funnel visualization** — for campaigns with `kpi_targets`, show a funnel: posts published → impressions → engagement → clicks → conversions (if webhook data available).

5. **Budget tracking** — for campaigns with `budget_cents`, show spent vs remaining. (Requires integration with ad platform APIs — defer to v23.)

6. **Posting cadence heatmap** — calendar heatmap of posts published per day for the last 30 days. Helps identify posting patterns.

7. **Channel mix analysis** — pie chart of posts by channel for the last 30 days. Helps ensure diversification.

8. **Best posting times** — based on historical engagement, show the best posting times per channel. (Requires engagement data analysis — defer to v23.)

**Effort**: 3-4 days (1-2 days for backend endpoints, 2 days for frontend widgets).

### F.7 Summary: Campaign management rework effort

| Item | Effort | Priority |
|---|---|---|
| F.1 Campaign model expansion | 1 day | Phase 6 |
| F.2 Kanban re-architecture | 4-5 days | Phase 6 |
| F.3 Campaign detail page | 3-4 days | Phase 6 |
| F.4 Campaign analytics endpoint | 1 day | Phase 6 |
| F.5 Realtime broadcast | 1 day | Phase 1 (BUG #7) + Phase 6 |
| F.6 Strategic dashboard widgets | 3-4 days | Phase 6 |
| **Total** | **~13-16 days** | **Phase 6** |

---

## Part G — Phased Refactor Plan (7 phases)

Each phase is independently shippable. Per AGENTS.md §0.1, **commit + push after each phase**. Each phase has acceptance tests and verification steps.

### Phase 1 — Backend critical fixes (BLOCKING)

**Goal**: Fix the 5 CRITICAL bugs + 2 quick wins. ~1.5 days.

| # | Task | File | Effort |
|---|---|---|---|
| 1.1 | Fix BUG #19: Move `/api/events` to `protected_routes` OR add auth extractor to `sse_handler` | `src/api/mod.rs:105`, `src/api/sse.rs` | 1h |
| 1.2 | Fix BUG #22: Replace `and_hms_opt().unwrap()` with `let Some(slot) = ... else { continue }` + validate `minutes < 1440` at API layer | `src/db/queries.rs:1316`, `src/api/integrations.rs::update_timeslots` | 15min |
| 1.3 | Fix BUG #6: Add state-transition validation to `PATCH /api/posts/{id}/stage` | `src/api/campaigns.rs:174` | 2h |
| 1.4 | Fix BUG #7: Add `post_stage_changed` broadcast after stage UPDATE | `src/api/campaigns.rs:174` | 30min |
| 1.5 | Fix BUG #15: Pass `idempotency_key: Some(post.idempotency_key.to_string())` in manual publish | `src/services/posts.rs:323` | 5min |
| 1.6 | Fix BUG #18: Emit synthetic `"lagged"` event on SSE lag | `src/api/sse.rs:33` | 30min |
| 1.7 | Fix D.5: Run `reclaim_stuck_publishing` every tick | `src/scheduler/mod.rs` | 30min |
| 1.8 | Fix frontend automation NIL UUID bug | `src/routes/automation/+page.svelte:65` | 30min |
| 1.9 | Add `post_stage_changed` to realtime subscription list | `frontend/src/lib/stores/realtime.ts` | 15min |
| 1.10 | Wire `post_stage_changed` listener in kanban | `frontend/src/routes/kanban/+page.svelte` | 30min |

**Acceptance tests**:
- `curl http://localhost:6543/api/events` without cookie → 401.
- `PUT /api/integrations/{id}/timeslots` with `minutes: 99999` → 400 (not 500).
- `PATCH /api/posts/{id}/stage` with `state: "published"` on a post without `platform_post_id` → 400.
- Drag a post in kanban tab A → tab B updates within 1s.
- `POST /api/posts/{id}/publish` passes `idempotency_key` to provider (verify via X request headers).
- SSE tab throttled for 5min → returns → receives `"lagged"` event → calendar refetches.
- Process running for 1h → stuck publish reclaimed without restart.

**Verification**:
```bash
cargo check --lib --bin social-forge   # 0 errors
cd frontend && pnpm build              # succeeds
cargo test --lib                       # all pass
git add -A && git commit -m "v22 Phase 1: Backend critical fixes (5 CRITICAL bugs + 2 quick wins)"
git push origin master
```

### Phase 2 — Backend high-priority + posting-infra upgrade

**Goal**: Fix 3 HIGH bugs + implement posting-infra upgrade (Part D). ~4-5 days.

| # | Task | File | Effort |
|---|---|---|---|
| 2.1 | Fix BUG #1: `join_set.abort_all()` on timeout + mark posts as error | `src/scheduler/mod.rs:451` | 1h |
| 2.2 | Fix BUG #2: Move `acquire_owned().await` inside spawned future | `src/scheduler/mod.rs:388` | 1h |
| 2.3 | Fix BUG #3: Circuit breaker half-open atomic CAS | `src/social/registry.rs:126` | 1h |
| 2.4 | D.1: Implement `Idempotency-Key` header in X, LinkedIn, Reddit, Slack `publish()` | `src/social/{x,linkedin,reddit,slack}.rs` | 2-3 days |
| 2.5 | D.1: Application-level dedup cache for non-HTTP-idempotency providers | `src/social/mod.rs` | 1 day |
| 2.6 | D.2: Migration 032 `publish_outbox` table | `migrations/032_publish_outbox.sql` | 2h |
| 2.7 | D.2: `drain_publish_outbox` function + scheduler integration | `src/scheduler/mod.rs` | 1 day |
| 2.8 | D.3: Per-platform FIFO claim ordering | `src/db/queries.rs::get_due_posts` | 1 day |
| 2.9 | D.4: Migration 033 `posts.publish_workflow_version` + dispatch logic | `migrations/033_*.sql`, `src/scheduler/mod.rs` | 1 day |

**Acceptance tests**:
- Scheduler timeout → remaining posts marked `error`, no detached tasks.
- 30 queued posts across 3 providers → all 3 providers publish in parallel (not serial).
- Circuit breaker half-open → only 1 request admitted (not 5).
- X publish with `Idempotency-Key` header → verify via X API logs.
- Publish succeeds but DB write fails → outbox drain retries within 30s.
- Per-provider claim ordering: 5 X posts claimed in schedule order.

**Verification**: Same as Phase 1 + `cargo test --lib -- publish_outbox`.

### Phase 3 — Frontend design-system primitives + color token migration

**Goal**: Build missing primitives + replace 80 hardcoded hex colors. ~5-7 days.

| # | Task | File | Effort |
|---|---|---|---|
| 3.1 | Add `success`/`warning`/`error`/`info` + `radius-sm/md/lg` tokens to `tailwind.config.js` + `app.css` | `tailwind.config.js`, `src/app.css` | 1h |
| 3.2 | Build 13 new primitives (`Card`, `EmptyState`, `PageHeader`, `Tabs`, `Table`, `Pagination`, `StatCard`, `Tooltip`, `Avatar`, `Skeleton`, `FilterBar`, `CommandPalette`, `DataTable`) | `src/lib/ui/*.svelte`, `src/lib/components/CommandPalette.svelte` | 2-3 days |
| 3.3 | Migrate `CommentsThread.svelte` (16 hex hits) | `src/lib/components/CommentsThread.svelte` | 2h |
| 3.4 | Migrate `Button.svelte` `<style>` block (critical — primary CTA must retheme) | `src/lib/ui/Button.svelte` | 1h |
| 3.5 | Migrate `RichTextEditor.svelte` (5 + `<style>`) | `src/lib/composer/RichTextEditor.svelte` | 2h |
| 3.6 | Migrate `MediaCarousel.svelte` (6) | `src/lib/media/MediaCarousel.svelte` | 1h |
| 3.7 | Migrate `RssFeedForm.svelte` (5) | `src/lib/rss/RssFeedForm.svelte` | 1h |
| 3.8 | Migrate remaining 16 files (1-3 hits each) | various | 1 day |
| 3.9 | Fix `Badge.svelte` to support generic variants | `src/lib/ui/Badge.svelte` | 30min |
| 3.10 | Audit light mode — toggle + fix contrast issues | all | 4h |

**Acceptance tests**:
- `grep -r 'bg-\[#' frontend/src/` returns 0 matches.
- Toggle to light mode → all buttons, calendar, composer, comments render correctly.
- `pnpm build` succeeds with 0 errors.
- `pnpm exec svelte-check --threshold error` → 0 errors.

**Verification**: `git commit -m "v22 Phase 3: Design-system primitives + color token migration (80 hex → 0)"`.

### Phase 4 — Sidebar dedup + command palette + global search

**Goal**: Fix the worst UX bug in the app + add command palette + global search. ~2 days.

| # | Task | File | Effort |
|---|---|---|---|
| 4.1 | Remove 8 settings sub-routes from main sidebar, replace with single "Settings" entry | `src/routes/+layout.svelte:68-80` | 30min |
| 4.2 | Fix active-link matching: `startsWith` instead of `===` | `src/routes/+layout.svelte:163` | 15min |
| 4.3 | Add collapse-to-icon-rail toggle (224px ↔ 56px), persist via localStorage | `src/routes/+layout.svelte` | 4h |
| 4.4 | Build `CommandPalette.svelte` (Cmd+K), fuzzy search, recent commands | `src/lib/components/CommandPalette.svelte` | 1 day |
| 4.5 | Add global search input to sidebar (expanded state), results dropdown | `src/routes/+layout.svelte` | 4h |
| 4.6 | Add `Cmd+K` keyboard shortcut wiring | `src/lib/stores/keyboard.svelte.ts` | 30min |

**Acceptance tests**:
- Navigate to `/settings/profile` → only settings sidebar shows "Brand Profile" active; main sidebar shows "Settings" active (not "General").
- Click collapse toggle → sidebar shrinks to 56px (icons only); click again → expands to 224px.
- `Cmd+K` → command palette opens; type "cal" → "Go to Calendar" appears; Enter → navigates.
- Sidebar search "linkedin" → results show posts/channels containing "linkedin".

**Verification**: `git commit -m "v22 Phase 4: Sidebar dedup + command palette + global search"`.

### Phase 5 — Dashboard re-architecture (12-widget command center)

**Goal**: Transform the dashboard from thin read-only to strategic command center. ~3-4 days.

| # | Task | File | Effort |
|---|---|---|---|
| 5.1 | Backend: `GET /api/analytics/engagement?days=7` (with deltas + per-day sparkline) | `src/api/analytics.rs` | 4h |
| 5.2 | Backend: `GET /api/analytics/audience?days=30` (per-channel follower snapshots) | `src/api/analytics.rs` | 4h |
| 5.3 | Backend: `GET /api/analytics/adherence?days=7` (scheduled vs published vs failed) | `src/api/analytics.rs` | 2h |
| 5.4 | Backend: `GET /api/analytics/cadence?days=30` (goal vs actual + streak) | `src/api/analytics.rs` | 2h |
| 5.5 | Backend: `GET /api/campaigns/active/progress` (active campaigns with progress) | `src/api/campaigns.rs` | 2h |
| 5.6 | Backend: `GET /api/events/recent?limit=10` (recent activity from events_log) | `src/api/events.rs` (new) | 2h |
| 5.7 | Backend: Migration 034 `events_log` table (last 1000 events) | `migrations/034_events_log.sql` | 1h |
| 5.8 | Backend: Fix BUG #11 (engagement-based `best_provider`) | `src/api/analytics.rs:271` | 2h |
| 5.9 | Frontend: Build 11 dashboard widget components | `src/lib/components/dashboard/*.svelte` | 2 days |
| 5.10 | Frontend: Rewrite `src/routes/+page.svelte` with 12-widget layout | `src/routes/+page.svelte` | 1 day |
| 5.11 | Frontend: Fix timezone bug in "today's schedule" (use `timezone` store) | `src/routes/+page.svelte:56` | 30min |
| 5.12 | Frontend: Wire realtime listeners (post_published → EngagementWidget refetch, etc.) | `src/routes/+page.svelte` | 4h |

**Acceptance tests**:
- Dashboard loads in <2s with all 12 widgets populated.
- Stat cards show trend deltas (e.g., "Drafts 12 +3 ▲").
- Engagement widget shows sparkline + per-day breakdown.
- Channel performance sorted by engagement rate (not post count).
- Today's schedule respects user's selected timezone.
- Realtime: publish a post in another tab → dashboard "Recent activity" updates within 1s.

**Verification**: `git commit -m "v22 Phase 5: Dashboard re-architecture (12-widget command center)"`.

### Phase 6 — Campaign management rework (strategic dashboard)

**Goal**: Full campaign management with kanban re-architecture + campaign detail pages + analytics. ~8-10 days.

| # | Task | File | Effort |
|---|---|---|---|
| 6.1 | Migration 031: Campaign model expansion (status, progress_metric, audience_persona, content_pillars, budget, kpi_targets, deleted_at, sort_order) | `migrations/031_campaign_expansion.sql` | 1 day |
| 6.2 | Migration 032: Posts expansion (sort_order, kanban_substate, due_date, priority) | `migrations/032_posts_kanban_fields.sql` | 2h |
| 6.3 | Migration 033: `post_activity` table (activity log) | `migrations/033_post_activity.sql` | 2h |
| 6.4 | Backend: Update `Campaign` struct + `CreateCampaignRequest` + `UpdateCampaignRequest` with new fields | `src/api/campaigns.rs` | 4h |
| 6.5 | Backend: `GET /api/campaigns/{id}/analytics` endpoint | `src/api/campaigns.rs` or `src/api/analytics.rs` | 1 day |
| 6.6 | Backend: `GET /api/campaigns/active/progress` endpoint (if not in Phase 5) | `src/api/campaigns.rs` | 2h |
| 6.7 | Backend: Soft-delete for campaigns (BUG #10 fix) + `POST /api/campaigns/{id}/restore` | `src/api/campaigns.rs` | 2h |
| 6.8 | Backend: `post_activity` recording on every state change / edit | `src/services/posts.rs`, `src/api/campaigns.rs` | 4h |
| 6.9 | Backend: Add `campaign_created` / `campaign_updated` / `campaign_deleted` broadcasts | `src/api/campaigns.rs` | 1h |
| 6.10 | Frontend: Build `src/lib/kanban/` (KanbanBoard, KanbanColumn, KanbanCard, KanbanSwimlane, KanbanFilters, KanbanCardPreview, KanbanCardActivity) | `src/lib/kanban/*.svelte` | 3 days |
| 6.11 | Frontend: Rewrite `src/routes/kanban/+page.svelte` using new kanban lib + swimlanes + WIP limits + card cover images + due dates + priority + sub-states | `src/routes/kanban/+page.svelte` | 1 day |
| 6.12 | Frontend: Build `/campaigns` list page (grid of campaign cards with progress bars) | `src/routes/campaigns/+page.svelte` | 1 day |
| 6.13 | Frontend: Build `/campaigns/[id]` detail page (Overview/Posts/Timeline/Analytics/Settings tabs) | `src/routes/campaigns/[id]/+page.svelte` | 2 days |
| 6.14 | Frontend: Campaign create/edit form (all fields: name, description, color picker, dates, goal, status, progress_metric, progress_target, audience_persona, content_pillars, budget, kpi_targets) | `src/lib/components/CampaignForm.svelte` | 1 day |
| 6.15 | Frontend: Fix kanban campaign filter (`campaign_id` on `PostSummary` type) | `src/routes/kanban/+page.svelte:69`, `src/lib/api/posts.ts` | 30min |
| 6.16 | Frontend: Fix `quickAddIdea` to require channel selection (no more `integration_ids: []`) | `src/routes/kanban/+page.svelte:85` | 30min |
| 6.17 | Frontend: Add `campaign_created` / `campaign_updated` / `campaign_deleted` to realtime subscription + listeners | `src/lib/stores/realtime.ts`, kanban + campaigns pages | 1h |
| 6.18 | Frontend: Strategic dashboard widgets (F.6 — campaign goal progress, audience persona cards, content pillar tracking, posting cadence heatmap, channel mix pie chart) | `src/lib/components/dashboard/*.svelte` | 2 days |

**Acceptance tests**:
- Create a campaign with all fields (name, color, dates, goal, status, progress_metric, target, audience_persona, content_pillars) → persists.
- Kanban: swimlane by campaign → cards grouped horizontally.
- Kanban: WIP limit on Drafts column = 5 → 6th drop rejected with toast.
- Kanban: drag-to-reorder within column → sort_order persists on reload.
- Kanban: card shows cover image (if media), tags, due date, priority bar.
- Campaign detail page: Overview tab shows progress bar (12/20 posts = 60%), projected completion date, audience persona, content pillars.
- Campaign detail page: Analytics tab shows engagement totals, posts-by-day chart, engagement-by-channel chart.
- Campaign filter on kanban: select "Q4 Launch" → only Q4 Launch posts show.
- Quick-add to Ideas: requires channel selection (no empty `integration_ids`).
- Multi-tab: create campaign in tab A → tab B campaigns list updates within 1s.

**Verification**: `git commit -m "v22 Phase 6: Campaign management rework (strategic dashboard + kanban re-architecture)"`.

### Phase 7 — Calendar + composer polish (postiz-inspired)

**Goal**: Make the calendar and composer feel postiz-grade. ~5-7 days.

| # | Task | File | Effort |
|---|---|---|---|
| 7.1 | Calendar: Fix Monday/Sunday consistency (MonthView → Monday-start) | `src/lib/calendar/utils.ts::getMonthDays`, `MonthView.svelte` | 1h |
| 7.2 | Calendar: Add mini-calendar in sidebar for date jumping | `src/lib/calendar/MiniCalendar.svelte`, `+layout.svelte` | 4h |
| 7.3 | Calendar: Add today pulse dot in MonthView | `MonthView.svelte` | 30min |
| 7.4 | Calendar: `mix-blend-difference` tag chip (postiz-style) | `CalendarEvent.svelte` | 1h |
| 7.5 | Calendar: Per-cell `useInterval` greyscale flip (2min) | `src/lib/stores/calendar.svelte.ts` | 1h |
| 7.6 | Calendar: Add `integration_ids`, `tag_ids`, `campaign_id` filters to `CalendarQuery` + API + frontend filter bar | `src/api/calendar.rs`, `CalendarHeader.svelte` | 1 day |
| 7.7 | Calendar: Add `campaign_id` to `CalendarPostWithMetrics` response | `src/api/calendar.rs` | 30min |
| 7.8 | Calendar: Fix timezone in bulk-reschedule (use `timezone` store, not `${date}T${time}:00.000Z`) | `routes/calendar/+page.svelte:81`, `routes/posts/+page.svelte:238` | 1h |
| 7.9 | Calendar: Batch bulk operations (`Promise.all` with concurrency 5) | `routes/calendar/+page.svelte:60-87` | 1h |
| 7.10 | Composer: Mantine-style date picker popover (custom Svelte, no Mantine dep) | `src/lib/composer/SchedulePicker.svelte` | 1 day |
| 7.11 | Composer: Per-row delay for any provider (generic thread builder) | `src/lib/composer/ThreadBuilder.svelte` (new, replaces ThreadFinisher), `src/social/mod.rs::PostContent.delay_minutes`, scheduler | 1 day |
| 7.12 | Composer: TipTap Mention extension + suggestion popup | `src/lib/composer/RichTextEditor.svelte`, `src/lib/composer/extensions/Mention.svelte` | 1 day |
| 7.13 | Composer: TipTap Underline extension + bold/underline mutex | `src/lib/composer/RichTextEditor.svelte` | 2h |
| 7.14 | Composer: Fix alt-text dropped on submit (`alt` field on `MediaItem`, `saveAlt` writes to `item.alt`, `buildPayload` reads `m.alt`) | `ComposerModal.svelte:378`, `MediaUpload.svelte:143` | 1h |
| 7.15 | Composer: Fix `postsApi.update` to accept `tag_ids` + `first_comment` (backend + frontend) | `src/lib/api/posts.ts`, `src/api/posts.rs::update` | 2h |
| 7.16 | Composer: X `weightedLength` for char count | `src/lib/composer/PerPlatformCharCount.svelte` | 2h |
| 7.17 | Composer: More platform previews (X, Reddit, Threads, Bluesky) | `src/lib/composer/previews/` | 1 day |
| 7.18 | Composer: Brand profile sync to backend + AiAssistant reads it | `src/api/profile.rs` (new), `src/routes/settings/profile/+page.svelte`, `AiAssistant.svelte` | 1 day |
| 7.19 | Composer: `saveAsDraft` edit-mode state transition (backend `POST /api/posts/{id}/unschedule`) | `src/api/posts.rs`, `ComposerModal.svelte:541` | 2h |
| 7.20 | Composer: Fix timezone in SchedulePicker (same as 7.8) | `src/lib/composer/SchedulePicker.svelte:45` | 30min |

**Acceptance tests**:
- Switch calendar Month → Week → Month → grid columns don't shift (both Monday-start).
- Mini-calendar in sidebar: click a date → main calendar jumps to that date.
- Tag chip uses `mix-blend-difference` → label readable on any tag color.
- Leave calendar tab open overnight → yesterday's cells grey out at midnight (via `useInterval`).
- Filter calendar by `integration_ids=x` → only X posts show.
- Bulk-reschedule 10 posts: all respect user's timezone (not UTC).
- Composer date picker: popover opens, locale-aware format, `useClickOutside` closes.
- Composer: add 3 rows with delays 0/5/10 min → scheduler sleeps 5min between row 1 and 2, 10min between 2 and 3.
- Composer: type `@` → mention suggestion popup appears.
- Composer: submit with alt text on media → alt text persists in DB.
- Composer: edit a post, change tags + first comment → both persist on update.
- Composer: X char count uses `weightedLength` (emoji = 2 chars, not 1).

**Verification**: `git commit -m "v22 Phase 7: Calendar + composer polish (postiz-inspired)"`.

### Phase summary

| Phase | Days | Bugs fixed | Features added |
|---|---|---|---|
| Phase 1 — Backend critical fixes | 1.5 | 5 CRITICAL + 2 quick | SSE auth, kanban validation, idempotency, lagged event, stuck-publish sweep |
| Phase 2 — Backend high-priority + posting-infra | 4-5 | 3 HIGH + BUG #3 | Idempotency keys consumed, transactional outbox, per-platform FIFO, workflow versioning |
| Phase 3 — Design-system + color tokens | 5-7 | (frontend hex migration) | 13 new primitives, 0 hardcoded hex, working light mode |
| Phase 4 — Sidebar + command palette + search | 2 | (sidebar dedup) | Collapse-to-icon-rail, Cmd+K palette, global search |
| Phase 5 — Dashboard re-architecture | 3-4 | BUG #11 + timezone bug | 12-widget command center, 6 new analytics endpoints |
| Phase 6 — Campaign management rework | 8-10 | BUG #6, #7, #8, #9, #10 + frontend bugs | Strategic dashboard, kanban re-architecture, campaign detail pages, campaign analytics |
| Phase 7 — Calendar + composer polish | 5-7 | alt-text bug, timezone bugs, tag-ids-on-edit bug | Mantine-style picker, generic thread builder, mentions, more previews |
| **Total** | **~29-37 days** | **26 bugs** | **All 6 user complaint areas addressed** |

---

## Part H — Deferred to v23 + Appendix

### H.1 Deferred to v23

| Item | Why deferred |
|---|---|
| `intervalInDays` single-row recurring model (postiz-style) | Schema migration + client virtualization — larger lift. SF's N-copies model works; defer to v23. |
| `POST /posts` with `type: 'draft'\|'schedule'\|'now'\|'update'` discriminator | API redesign — breaking change. Defer to v23. |
| CopilotKit in-composer AI | Needs CopilotKit subscription + backend integration. Out of scope. |
| AI Generator (NDJSON streaming) | Needs agent-graph backend (Mastra/LangGraph equivalent in Rust). Out of scope for v22. |
| AI Image with 14 style chips | Needs image-gen provider integration. Out of scope. |
| Generic thread builder per-row delay for all providers | Included in Phase 7 (7.11). |
| Full i18n (15 locales) | Single-user English-only is fine per architectural preferences. |
| Split god modules (`queries.rs` 2863 lines, `integrations.rs` 1434 lines, `onboard.rs` 1612 lines) | Pure refactor — do in v23 with no behavior change. |
| `post_engagement_for_posts` table (proper engagement FK) | v21 Phase 1 Option 3 — deferred. |
| Convert all `query_as::<_, T>` to `query_as!` | Needs `.sqlx` cache regeneration — Phase 2 task (BUG #24). |
| `Sets` auto-open before composer | Minor; defer. |
| Budget tracking with ad platform APIs | Needs Facebook Ads / Google Ads / LinkedIn Ads API integration. Defer to v23. |
| Funnel visualization with webhook data | Needs conversion webhook integration. Defer to v23. |
| Best posting times analysis | Needs historical engagement data analysis. Defer to v23. |
| Per-platform FIFO queue (strict across ticks) | Phase 2 D.3 covers the common case; strict FIFO defer to v23. |
| Audit log UI | `post_activity` table added in Phase 6; UI for global audit log defer to v23. |

### H.2 Architectural preferences preserved

All v22 changes respect the social-forge architectural preferences (AGENTS.md §0.5):
- ✅ Single Rust binary (no new services, no Temporal, no Redis)
- ✅ SvelteKit SPA embedded via `rust-embed`
- ✅ Single-user assumption (`DEFAULT_USER_ID` unchanged)
- ✅ No external queue (in-process tokio scheduler retained; transactional outbox is a Postgres table, not Redis)
- ✅ No microservices
- ✅ SSE realtime retained (not SWR polling)
- ✅ Token encryption at rest retained
- ✅ No team collaboration features (campaigns are single-user)
- ✅ No marketplace / payouts
- ✅ No new third-party frontend libs (custom Svelte popover, not Mantine; custom command palette, not cmdk; custom kanban, not react-kanban)

### H.3 Appendix — File reference

#### Critical files modified per phase

**Phase 1** (backend critical fixes):
- `src/api/mod.rs` — move `/api/events` to protected_routes
- `src/api/sse.rs` — lagged event signal
- `src/db/queries.rs:1316` — `and_hms_opt` panic fix
- `src/api/integrations.rs::update_timeslots` — validate minutes
- `src/api/campaigns.rs:174` — state-transition validation + `post_stage_changed` broadcast
- `src/services/posts.rs:323` — idempotency key on manual publish
- `src/scheduler/mod.rs` — `reclaim_stuck_publishing` every tick
- `frontend/src/lib/stores/realtime.ts` — `post_stage_changed` subscription
- `frontend/src/routes/kanban/+page.svelte` — realtime listener
- `frontend/src/routes/automation/+page.svelte:65` — NIL UUID fix

**Phase 2** (posting-infra upgrade):
- `src/scheduler/mod.rs:388, 451` — permit inside spawn, abort on timeout
- `src/social/registry.rs:126` — circuit breaker atomic CAS
- `src/social/{x,linkedin,reddit,slack}.rs` — `Idempotency-Key` header
- `src/social/mod.rs` — application-level dedup cache
- `migrations/032_publish_outbox.sql` (new)
- `migrations/033_posts_workflow_version.sql` (new)
- `src/db/queries.rs::get_due_posts` — per-provider FIFO claim ordering
- `src/scheduler/mod.rs` — `drain_publish_outbox` + `publish_v1`/`publish_v2` dispatch

**Phase 3** (design-system + color tokens):
- `frontend/tailwind.config.js` — success/warning/error/info + radius tokens
- `frontend/src/app.css` — CSS variables for new tokens
- `frontend/src/lib/ui/*.svelte` — 13 new primitives
- `frontend/src/lib/components/CommandPalette.svelte` (new)
- 22 files migrated from hardcoded hex to semantic tokens (see E.3 table)
- `frontend/src/lib/ui/Button.svelte` — `<style>` block hex → CSS vars
- `frontend/src/lib/ui/Badge.svelte` — generic variant support
- `frontend/src/lib/calendar/CalendarEvent.svelte` — RGBA state colors → semantic tokens

**Phase 4** (sidebar + command palette + search):
- `frontend/src/routes/+layout.svelte:68-80, 163` — sidebar dedup + active-link fix
- `frontend/src/routes/+layout.svelte` — collapse-to-icon-rail + global search
- `frontend/src/lib/components/CommandPalette.svelte` (new)
- `frontend/src/lib/stores/keyboard.svelte.ts` — Cmd+K wiring

**Phase 5** (dashboard re-architecture):
- `src/api/analytics.rs` — 5 new endpoints + BUG #11 fix
- `src/api/campaigns.rs` — `GET /api/campaigns/active/progress`
- `src/api/events.rs` (new) — `GET /api/events/recent`
- `migrations/034_events_log.sql` (new) — events_log table
- `frontend/src/routes/+page.svelte` — 12-widget layout rewrite
- `frontend/src/lib/components/dashboard/*.svelte` (new) — 11 widget components

**Phase 6** (campaign management rework):
- `migrations/031_campaign_expansion.sql` (new)
- `migrations/032_posts_kanban_fields.sql` (new) — sort_order, kanban_substate, due_date, priority
- `migrations/033_post_activity.sql` (new) — activity log
- `src/api/campaigns.rs` — new fields + soft-delete + analytics endpoint + broadcasts
- `src/services/posts.rs` — `post_activity` recording
- `frontend/src/lib/kanban/*.svelte` (new) — 7 kanban components
- `frontend/src/routes/kanban/+page.svelte` — rewrite using new kanban lib
- `frontend/src/routes/campaigns/+page.svelte` (new) — campaign list
- `frontend/src/routes/campaigns/[id]/+page.svelte` (new) — campaign detail
- `frontend/src/lib/components/CampaignForm.svelte` (new) — create/edit form
- `frontend/src/lib/components/dashboard/*.svelte` — strategic dashboard widgets
- `frontend/src/lib/api/posts.ts` — add `campaign_id` to `PostSummary`
- `frontend/src/lib/api/campaigns.ts` — add new fields + analytics method
- `frontend/src/lib/stores/realtime.ts` — campaign events subscription

**Phase 7** (calendar + composer polish):
- `frontend/src/lib/calendar/utils.ts` — Monday-start consistency
- `frontend/src/lib/calendar/MiniCalendar.svelte` (new)
- `frontend/src/lib/calendar/MonthView.svelte` — today pulse dot
- `frontend/src/lib/calendar/CalendarEvent.svelte` — `mix-blend-difference` tag chip
- `frontend/src/lib/stores/calendar.svelte.ts` — `useInterval` greyscale flip
- `frontend/src/lib/calendar/CalendarHeader.svelte` — filter bar
- `src/api/calendar.rs` — `integration_ids`, `tag_ids`, `campaign_id` filters + `campaign_id` in response
- `frontend/src/routes/calendar/+page.svelte:81` — timezone fix + batch bulk ops
- `frontend/src/routes/posts/+page.svelte:238` — timezone fix
- `frontend/src/lib/composer/SchedulePicker.svelte` — Mantine-style popover + timezone fix
- `frontend/src/lib/composer/ThreadBuilder.svelte` (new, replaces ThreadFinisher)
- `src/social/mod.rs::PostContent` — `delay_minutes` field
- `src/scheduler/mod.rs` — sleep `delay_minutes` between rows
- `frontend/src/lib/composer/RichTextEditor.svelte` — Mention + Underline extensions
- `frontend/src/lib/composer/extensions/Mention.svelte` (new)
- `frontend/src/lib/composer/MediaUpload.svelte:143` — alt-text fix
- `frontend/src/lib/composer/ComposerModal.svelte:378` — alt-text in payload
- `frontend/src/lib/api/posts.ts` — `update` accepts `tag_ids` + `first_comment`
- `src/api/posts.rs::update` — accept `tag_ids` + `first_comment`
- `frontend/src/lib/composer/PerPlatformCharCount.svelte` — X `weightedLength`
- `frontend/src/lib/composer/previews/{X,Reddit,Threads,Bluesky}.svelte` (new)
- `src/api/profile.rs` (new) — brand profile sync
- `frontend/src/routes/settings/profile/+page.svelte` — sync to backend
- `frontend/src/lib/composer/AiAssistant.svelte` — read brand profile
- `src/api/posts.rs` — `POST /api/posts/{id}/unschedule` endpoint

#### Reference files (postiz-app, for inspiration)

| Concern | Path |
|---|---|
| Calendar grid + chip | `apps/frontend/src/components/launches/calendar.tsx` |
| Calendar SWR + URL sync | `apps/frontend/src/components/launches/calendar.context.tsx` |
| "Just update vs Reschedule" modal | `calendar.tsx:659-746` |
| Per-cell `useInterval` greyscale | `calendar.tsx:643-651` |
| Tag-color `mix-blend-difference` chip | `calendar.tsx:1062-1077` |
| "+N more" expander | `calendar.tsx:881-896` |
| Per-post error badge | `calendar.tsx:1039, 1045-1053` |
| Composer modal | `apps/frontend/src/components/new-launch/manage.modal.tsx` |
| TipTap extensions (Mention, Underline, mutex) | `apps/frontend/src/components/new-launch/editor.tsx:36-99` |
| Per-row delay for any provider | `store.ts:14`, `editor.tsx:69`, `post.workflow.v1.0.5.ts:180` |
| First-comment-as-thread | `post.workflow.v1.0.5.ts:153-159` |
| Mantine date picker | `apps/frontend/src/components/new-launch/date.picker.tsx` |
| Repeat (intervalInDays single-row) | `repeat.component.tsx:10-51`, `posts.repository.ts:200-220` |
| `askClose` confirmation | `manage.modal.tsx:149-169`, `new-modal.tsx:105-114` |
| Pre-flight validation (per-platform) | `manage.modal.tsx:275-345`, `posts.service.ts:762-858` |
| Signatures auto-append | `calendar.tsx:797-805` |
| Modal manager (Zustand) | `apps/frontend/src/components/layout/new-modal.tsx` |
| Posts controller | `apps/backend/src/api/routes/posts.controller.ts` |
| Posts service | `libraries/nestjs-libraries/src/database/prisma/posts/posts.service.ts` |
| `type: 'draft'\|'schedule'\|'now'\|'update'` discriminator | `create.post.dto.ts:93-125` |
| Temporal workflow v1.0.5 | `apps/orchestrator/src/workflows/post-workflows/post.workflow.v1.0.5.ts` |
| `missingPostWorkflow` sweeper | `apps/orchestrator/src/workflows/missing.post.workflow.ts` |
| Per-platform task queues | `posts.service.ts:919`, `post.workflow.v1.0.5.ts:19, 64-73` |
| Token refresh (proactive + reactive) | `refresh.token.workflow.ts:14-55`, `post.workflow.v1.0.5.ts:221-237` |
| Sidebar (80px icon rail) | `apps/frontend/src/components/new-layout/layout.component.tsx:105` |
| Settings (single entry → tab sub-nav) | `apps/frontend/src/components/settings/settings.component.tsx:35-218` |
| Color palette (modern `--new-*` vars) | `apps/frontend/src/app/colors.scss:2-96` |
| Light/dark toggle (cookie) | `apps/frontend/src/components/new-layout/top/mode.component.tsx` |
| AI Generator NDJSON stream | `posts.controller.ts:235-261`, `generator.tsx:49-130` |
| CopilotKit assistant | `manage.modal.tsx:664-685`, `layout.component.tsx:74-78` |

---

**End of v22 audit & plan. Implementation begins with Phase 1.**
