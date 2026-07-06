# Social-Forge × Postiz — Full-Scale Contrast Audit & Frontend Refactor Plan (v21)

> **Date**: 2026-07-07
> **Author**: Super Z orchestrator (post three parallel sub-agent audits)
> **Scope**: Backend (Rust/axum/sqlx) + Frontend (SvelteKit/Svelte 5) for `social-forge`; reference audit of `postiz-app` (Next.js 14 + NestJS + Temporal).
> **Architectural preferences (non-negotiable)**: single-user deployment, single Rust binary embedding SvelteKit via `rust-embed`, no Redis, no Temporal, no external queue, no microservices. All proposed upgrades must respect these constraints.

---

## 0. Executive Summary

Social-Forge has the right *bones* (Rust + axum + sqlx + SvelteKit 5 + TipTap + custom calendar), and the v18/v19/v20 plans already ported most of the *visible* Postiz UX (modal composer, week-default calendar, per-platform previews, multi-channel overrides). What it is missing is **depth**:

1. **A latent 500-bug in the calendar SQL** that fires on any malformed `analytics_cache` row. Verified empirically.
2. **A `PostState` enum drift** between Rust and Postgres that makes any kanban `'idea'`-state row unreadable.
3. **No real CRUD on feed posts** — only import + read + soft-hide + bookmark. The "Repurpose" button is a frontend-only no-op (opens composer with prefilled text, makes no backend call).
4. **A broken composer edit-mode** that creates duplicate posts instead of updating.
5. **Recurring schedule silently dropped** from the composer payload.
6. **Inconsistent error UX** — 13 native `confirm()`/`prompt()` calls, no retry button on calendar 500, no per-post error ring on the calendar.
7. **No durable execution guarantee** — in-process tokio scheduler with no idempotency keys, no transactional outbox, no replay. Adequate for a solo tool, not postiz-grade.

This document:
- **Part A** contrasts the two products across 12 architectural dimensions.
- **Part B** root-causes the three reported bugs (calendar 500, missing feed CRUD, posting infra gaps).
- **Part C** is a phased refactor plan (7 phases) with exact file paths, code snippets, and acceptance tests.
- **Part D** lists what is being fixed in this iteration vs. what is deferred to v22.

---

## Part A — Contrast Audit

### A.1 Stack comparison

| Dimension | Social-Forge | Postiz | Verdict |
|---|---|---|---|
| Backend language | Rust 2021 (single binary) | TypeScript / NestJS | SF is faster + smaller deploy; Postiz is more dev-velocity. Both fine. |
| Web framework | axum 0.8 | NestJS (Express under the hood) | Equivalent. |
| ORM | sqlx 0.8 (compile-time checked *mostly*) | Prisma 5 | sqlx is faster + type-safe at compile time; Prisma has better DX. SF has 1 un-checked `query_as::<_, T>` (the calendar one) — must fix. |
| DB | Postgres 16 | Postgres | Same. |
| Migrations | sqlx::migrate! (26 files) | Prisma Migrate | Same. |
| Frontend | SvelteKit 2 / Svelte 5 (runes) | Next.js 14 (App Router) | Postiz has bigger ecosystem; Svelte 5 is leaner. Both fine. |
| Frontend state | Svelte 5 rune class singletons | Zustand stores | Equivalent — SF's pattern is idiomatic for Svelte 5. |
| Calendar lib | Custom (CSS grid + Intl) | Custom (CSS grid + dayjs) | **Equivalent** — both hand-rolled. SF just needs polish. |
| Modal manager | `modals` rune class (stack, escape, z-index) | Zustand store (stack, askClose, isLast escape) | **Equivalent** — SF is missing `askClose` confirmation. |
| HTTP data layer | Custom `ApiClient` envelope | SWR + raw fetch + minify/expand | SF lacks SWR (no auto-revalidation, no dedup). |
| Realtime | SSE (`EventSource`) | SWR polling (1h) + `postMessage` for OAuth | Different choices; both work. |
| Scheduler | In-process tokio task, 30s poll, no durability | Temporal.io workflows, versioned, durable | **Postiz wins decisively.** SF needs at minimum: idempotency keys + transactional outbox. |
| Provider count | ~21 (CLI list) + ~30 in providers.ts | 33 | Comparable. |
| Validation contract | Per-handler ad-hoc | Shared class-validator DTOs (FE + BE import the same file) | **Postiz wins.** SF should extract a `validation.rs` shared module. |
| i18n | None (English only) | 15 locales | Out of scope for SF v21. |
| Soft delete | `external_posts.hidden_at`/`saved_at`, no soft-delete on `posts` | `deletedAt` everywhere | SF should add `posts.deleted_at`. |

### A.2 Calendar comparison

| Behavior | Social-Forge | Postiz | Gap |
|---|---|---|---|
| Default view | Week ✅ (persisted to localStorage) | Week ✅ (cookie + URL) | SF lacks URL sync — deep-linking breaks. |
| View modes | Month / Week / Day / List | Day / Week / Month / List | Equivalent. |
| Fetch endpoint | `GET /api/calendar?start=YYYY-MM-DD&end=YYYY-MM-DD` (date-only) | `GET /posts?startDate=…&endDate=…` (ISO datetime, UTC) | SF format is fine; just less standard. |
| **Error handling** | Tiny red `<div>` with no retry, no toast | Per-cell `animate-pulse` skeleton + SWR auto-retry | **Postiz wins.** SF needs skeleton + retry button. |
| Drag-to-reschedule | Yes (date precision in month, hour in week/day) | Yes (with optimistic local update + "Just update vs Reschedule" modal for already-published) | SF lacks the published-post-reschedule modal. |
| Per-post error indicator | Color on chip (red border for `error` state) | Red `ring-2 ring-red-500` + absolute `!` badge + tooltip | Equivalent in spirit; SF could add the `!` badge. |
| Hover actions | Edit / Duplicate / Stats / Delete (toolbar) | Edit / Duplicate / Preview / Stats / Connect-missing / Delete (5 icons) | Equivalent; Postiz has "Connect missing releaseId" for IG→X cross-posts — SF doesn't need this. |
| Tag-colored chip bar | No (chips use state colors) | Yes (`bg = post.tags[0].tag.color`) | **Borrow.** |
| "+N more" expander | Yes (month view) | Yes (inline expander, no popover) | Equivalent. |
| Past-cell greying | No | Yes (`!grayscale`, `cursor-not-allowed`, drop disabled) | **Borrow.** |
| Past-cell recompute | No (computed once) | Yes (every 2–2.5min via `useInterval`) | **Borrow** — minor. |
| List view state filter | Yes (All/Scheduled/Drafts/Published) | Yes (all/scheduled/draft/published segmented control) | Equivalent. |
| URL `?display=` sync | No | Yes (replaceState, no history pollution) | **Borrow.** |

### A.3 Composer comparison

| Behavior | Social-Forge | Postiz | Gap |
|---|---|---|---|
| Form factor | Full-screen modal (`max-w-[1400px]`, `h-[90vh]`) | Full-screen modal (`max-w-[1400px]`, 80% width, `fullScreen:true`) | Equivalent ✅. |
| Trigger | Global `composer` store (`openCreate`/`openEdit`) | Global modal manager (`modal.openModal({id:'add-edit-modal'})`) | SF's pattern is fine. |
| Two-column layout | Yes (editor + preview pane) | Yes (editor + `ShowAllProviders`) | Equivalent ✅. |
| Channel picker | Grid of integration cards | Round avatars (42×42) with platform badge | Equivalent in function; Postiz's avatar style is more compact. |
| Per-channel override | `SelectCurrent` pill strip + clone-on-first-switch | `SelectCurrent` dropdown + `addRemoveInternal` clone | Equivalent ✅. |
| Rich text editor | TipTap (Document/Text/Paragraph/Bold/Link/Heading/BulletList/ListItem/Placeholder/History) | TipTap + Bold/Underline/Link/Heading/BulletList/Mention/Placeholder/History + bold/underline mutex | SF lacks Mention + Underline. Minor. |
| Thread builder | `ThreadFinisher` (X only) | Generic `<AddPostButton>` per-row with delay + `<ThreadFinisher>` X-only + `<MergePost>` + `<SeparatePost>` | **Borrow generic thread builder.** SF has only X thread finisher. |
| First comment | `FirstComment` (LinkedIn/FB) | Generic — same mechanism as thread (rows > 0 become comments) | Equivalent ✅. |
| Media upload | Drag-reorder, paste, alt text, library picker, simulated progress | Uppy + drag/drop + paste + multi-image + video + alt text + real progress | SF simulates progress (bad UX). **Fix.** |
| AI Image | No | Yes (14 style chips) | **Borrow** (deferred to v22). |
| AI Generator (NDJSON stream) | No | Yes (sparkle-button → modal with shimmer events) | **Borrow** (deferred to v22). |
| AI assistant | `AiAssistant` (generate/improve/hashtags/tone/summarize) | `AiAssistant` + CopilotKit popup with `useCopilotAction` | SF's is simpler but functional. CopilotKit out of scope. |
| Char count per platform | `PerPlatformCharCount` | Per-platform `maximumCharacters` (static or fn of settings) | Equivalent ✅. |
| Live preview per platform | `PlatformPreviewPane` (IG/LinkedIn/Facebook/General) | `ShowAllProviders` (IG/LinkedIn/Facebook/X/TikTok/YouTube/Pinterest/General) | SF has 3 previews; Postiz has 7. Add as provider count grows. |
| Date picker | Native HTML date+time inputs | `@mantine/dates` Calendar popover with locale-aware format | SF is functional but less polished. **Borrow Mantine-style popover.** |
| Auto-schedule button | Yes (`POST /api/posts/find-slot`) | Yes (`GET /posts/find-slot`) | Equivalent ✅. |
| Repeat | Yes (`SchedulePicker` writes to `recurring` state) | Yes (`repeat.component.tsx` → `inter` field) | **SF BUG: `recurring` never sent in payload.** Fix. |
| Save Draft / Post Now / Schedule | Yes — 3 buttons in footer | Yes — Draft + Schedule (with hover-revealed "Post Now") | SF is clearer; Postiz is more compact. Equivalent. |
| `askClose` (confirm-on-close) | No | Yes (`"Are you sure? All data will be lost"`) | **Borrow.** SF loses drafts on accidental Escape. |
| Pre-flight validation | `POST /api/posts/validate` returns `{valid, errors}` | `POST /posts/valid` returns per-platform `{valid, settingsError, errors, emptyContent, tooLong}` | SF returns flat list; Postiz returns per-platform richer struct. **Borrow** richer response. |
| Per-platform error toast | Toast with first error only | Toast per platform with handle: `"X (my-handle): post is too long"` | **Borrow** — show all errors, not just first. |
| Sets (templates) | `PostSetModal` (save/load) | `SetSelectionModal` (opens before composer if sets exist) | SF has it; Postiz auto-opens. Minor. |
| Signatures auto-append | No | Yes (`onlyValues: [{content: '\n' + signature.content}]`) | **Borrow.** |

### A.4 Posts CRUD comparison

| Operation | Social-Forge | Postiz |
|---|---|---|
| List (calendar) | `GET /api/calendar?start&end` | `GET /posts?startDate&endDate` |
| List (paginated) | `GET /api/posts?state&limit&offset&q&integration_ids&tag_ids&sort` | `GET /posts/list?page&limit&state&customer` |
| Get one | `GET /api/posts/{id}` | `GET /posts/{id}` |
| Get group (for edit) | ❌ **No group endpoint** — frontend doesn't fetch siblings | `GET /posts/group/{group}` |
| Create | `POST /api/posts` (multi-integration, group_id returned) | `POST /posts` (type: draft/schedule/now/update) |
| Update | `PUT /api/posts/{id}` (content/title/media/settings) — **never called from UI** | `POST /posts` with `type: 'update'` |
| Delete | `DELETE /api/posts/{id}` (cascades group via `delete_post`) | `DELETE /posts/{group}` (soft-delete group + terminate workflow) |
| Reschedule | `PUT /api/posts/{id}/date` (with `move_group` flag) | `PUT /posts/{id}/date` (with `action: schedule\|update`) |
| Repeat | `POST /api/posts/{id}/repeat` (creates N copies upfront) | `intervalInDays` column — single row, virtualized client-side |
| Find free slot | `GET /api/posts/find-slot?integration_id` | `GET /posts/find-slot` (org-level) or `/find-slot/{integrationId}` |
| Validate | `POST /api/posts/validate` | `POST /posts/valid` |

**Gap analysis**:

1. **SF has no `GET /api/posts/group/{group_id}` endpoint** — the frontend can't fetch all sibling posts in a thread/group for editing. Postiz's edit flow always fetches the group first. SF's composer edit-mode bug (creates duplicate) is partly because the frontend doesn't have the right data to call update.
2. **SF's `intervalInDays` model is "create N copies upfront"** (`POST /api/posts/{id}/repeat` generates up to 100 rows). Postiz uses a single row with `intervalInDays` and virtualizes client-side. Postiz's approach is more storage-efficient and easier to update (one row → all virtualized chips update). **Borrow** for v22 — but it's a schema migration, so defer.
3. **SF has no `type: 'update'` discriminator** — `POST /api/posts` is always create. Postiz's single-endpoint upsert with `type` is cleaner. **Borrow** in v22.

### A.5 Feed comparison

| Operation | Social-Forge | Postiz |
|---|---|---|
| List | `GET /api/feed` (cursor + provider/author/q filter) | N/A — Postiz has no equivalent "imported posts from connected accounts" surface |
| Import | `POST /api/feed/import` (refresh from all providers) | N/A |
| Hide | `DELETE /api/feed/{id}` (soft-hide via `hidden_at`) | N/A |
| Bookmark | `POST/DELETE /api/feed/{id}/save` | N/A |
| Comments | `GET /api/feed/{id}/comments` (live-fetch — bypasses `cached_comments`!) | N/A |
| **Update** | ❌ **MISSING** | N/A |
| **Repurpose** | ❌ Frontend-only button (no backend call) | N/A |
| **Create manually** | ❌ MISSING | N/A |

> Note: Postiz doesn't have an equivalent "imported posts feed" — this is a SF-original feature. The user's complaint about "no CRUD for existing posts in feeds" needs interpretation: they want **(a)** CRUD on the user's own scheduled/published posts (which lives in `/posts`, not `/feed`), AND **(b)** the ability to edit/repost an imported feed item. Both gaps are real.

### A.6 Posting infrastructure comparison

| Capability | Social-Forge | Postiz | Gap severity |
|---|---|---|---|
| Scheduler | In-process tokio task, 30s poll | Temporal.io workflow per post | **High** — SF has no durability. |
| Workflow versioning | None | v1.0.1 → v1.0.5 (5 versions) | **Medium** — SF has no in-flight migrations. |
| Retry with backoff | Hand-rolled (5/10/20s + jitter), max 3 | Temporal retry policies | Equivalent in function. |
| Idempotency keys | ❌ None | ❌ None (Postiz also lacks this!) | Both have double-publish risk on retry. |
| Transactional outbox | ❌ None | ❌ None | Both have publish-orphan risk. |
| Per-provider task queue | Semaphore per provider (in-process) | Per-platform Temporal task queue | SF adequate for single-instance. |
| Stuck-publishing recovery | `reclaim_stuck_publishing` on startup (5min threshold) | `searchForMissingThreeHoursPosts` cron + `poke` signal | SF's is cruder but functional. |
| Plug/automation engine | `post_plugs` table + `start_plug_runner` | BullMQ workers + event bus | SF is simpler; adequate. |
| Soft delete | `external_posts.hidden_at/saved_at` only | `deletedAt` everywhere | **Borrow** for `posts`. |
| Webhooks | Yes (`/api/webhooks` + deliveries) | Yes (fired in workflow) | Equivalent. |
| Analytics refresh | 30min tokio task, writes `analytics_cache` | Per-post `getStatistics` + on-demand | Equivalent. |

**Verdict**: SF's posting infrastructure is *adequate for a solo-founder local tool* but is not postiz-grade. The two real gaps are (a) **no idempotency keys** → double-publish risk, and (b) **no transactional outbox** → publish-orphan risk. Both are fixable without Temporal.

---

## Part B — Root-Cause Analysis of Reported Bugs

### B.1 Calendar "error:500"

**File**: `src/db/queries.rs:1043-1084` (function `get_calendar_posts_with_metrics`)

**Root cause** (verified empirically against a fresh Postgres 17 with all 26 migrations applied):

```sql
-- Lines 1061-1064
(ac.data->>'likes')::bigint        as likes,
(ac.data->>'comments')::bigint     as comments,
(ac.data->>'shares')::bigint       as shares,
(ac.data->>'impressions')::bigint  as impressions
```

Postgres `->>` returns `text` (or NULL). `text::bigint` throws `invalid input syntax for type bigint` for any non-integer text. Verified failing cases:
- `'{"likes":"N/A"}'::jsonb` → `"N/A"::bigint` → ERROR
- `'{"likes":4.2}'::jsonb` → `"4.2"::bigint` → ERROR (float)
- `'{"likes":{"foo":1}}'::jsonb` → `'{"foo": 1}'::bigint` → ERROR (object)
- `'{"likes":[1,2,3]}'::jsonb` → `'[1, 2, 3]'::bigint` → ERROR (array)

**Aggravating factor**: The LEFT JOIN is semantically dead weight. Current writers (`src/api/analytics.rs:81-88`, `src/scheduler/mod.rs:1080-1096`) always store `Vec<AnalyticsData>` (a JSON **array** of `{label, data, percentage_change}` objects) — never an object with `likes`/`comments` keys. So in a pristine database the columns are always NULL. The 500 fires the moment any non-conforming row lands (e.g., from an older code version, an external script, or a future code path that stores `EngagementData` directly).

**Error path**: `sqlx::Error::Database` → `AppError::Database` (`src/error.rs:82-85`) → HTTP 500 with body `{"error":"Database error"}`. The frontend's `ApiClient` extracts `data.error` and renders it as the red `<div>` — hence "error:500" / "Database error".

**Fix** (3 options, ordered by robustness):

```sql
-- Option 1 (recommended): defensive CASE guard
CASE
  WHEN ac.data IS NULL
    OR jsonb_typeof(ac.data) <> 'object'
    OR NOT (ac.data ? 'likes')
    OR jsonb_typeof(ac.data->'likes') NOT IN ('number', 'null')
  THEN NULL::bigint
  ELSE (ac.data->>'likes')::bigint
END AS likes
-- (repeat for comments, shares, impressions)

-- Option 2 (simplest): drop the engagement columns entirely.
-- They're always NULL in the current schema. Engagement belongs to
-- post_engagement, not analytics_cache.
NULL::bigint AS likes,
NULL::bigint AS comments,
NULL::bigint AS shares,
NULL::bigint AS impressions

-- Option 3 (long-term): LEFT JOIN post_engagement on a new FK
-- posts.id ↔ post_engagement.post_id (currently the FK references
-- external_posts.id, which is a separate UUID space — see B.4).
```

**Recommendation**: **Option 2** for v21 (one-line change, zero risk), **Option 3** for v22 (requires migration).

### B.2 `PostState` enum drift (secondary 500 cause)

**Files**:
- `src/db/models.rs:103-117` — Rust enum `Draft|Queued|Publishing|Published|Error`
- `migrations/026_campaigns_kanban.sql:29` — DB enum adds `'idea'`
- `src/api/campaigns.rs:181` — kanban `PATCH /api/posts/{id}/stage` accepts `"idea"` and writes it via raw SQL

**Root cause**: Migration 026 added `'idea'` to the DB enum (for the kanban "Ideas" column) but the Rust `PostState` enum was never updated. Any `query_as!` macro call decoding `state as "state: PostState"` will fail with `sqlx::Error::Decode` on any row with `state='idea'`. 15 call sites in `queries.rs` are affected, including `list_posts`, `count_posts_by_user`, `get_post_by_id`, `update_post_date`, `schedule_post`, `publish_post`, `repeat_post`, `find_free_slot`, `set_post_tags`.

**Symptom**: After dragging a post to the "Ideas" kanban column, every subsequent `/api/posts*` GET returns 500. The calendar endpoint (`/api/calendar`) is NOT affected because it uses `p.state::text as state` + `state: String` — sidesteps the enum decode.

**Fix**: Add `Idea` variant to `PostState` enum + `Display` impl + `FromStr` impl + `FromRow` decode + update match arms in `list_posts`/`count_posts_by_user` that filter by state.

### B.3 Missing CRUD on feed posts

**Files**: `src/api/feed.rs` (371 lines) + `src/api/mod.rs:172-178` (router).

**Current endpoints** (verified by route inspection):
- `GET /api/feed` — list with cursor/provider/author/q filter
- `POST /api/feed/import` — refresh from providers
- `GET /api/feed/accounts` — distinct (provider, author) pairs
- `GET /api/feed/analytics` — engagement summary
- `GET /api/feed/{id}/comments` — **live-fetches, bypasses `cached_comments` table**
- `DELETE /api/feed/{id}` — soft-hide (`hidden_at = NOW()`)
- `POST /api/feed/{id}/save` — bookmark
- `DELETE /api/feed/{id}/save` — unsave

**Missing**:
- `PUT /api/feed/{id}` — update text/media/metadata on an imported post (rare but useful for fixing import errors)
- `POST /api/feed/{id}/repurpose` — convert an imported post into a Social Forge `posts` row (with `source_external_post_id` FK for provenance). The frontend "Repurpose" button currently opens the composer with `prefilledContent = post.text` and makes **zero backend calls** — the link between the new post and the source feed post is lost.
- `POST /api/feed` — manually create a feed post (for bookmarking external content the user found but hasn't imported yet)

**Fix**: Add `PUT /api/feed/{id}` + `POST /api/feed/{id}/repurpose` in v21. Defer manual-create to v22.

### B.4 `list_posts_search` LEFT JOINs `post_engagement` on wrong FK

**File**: `src/db/queries.rs:607`

```sql
LEFT JOIN post_engagement pe ON pe.post_id = p.id
```

But `post_engagement.post_id` references `external_posts.id` (migration 016), **NOT** `posts.id`. The join compares two unrelated UUID spaces. It "works" (returns NULLs) only because UUID collisions are vanishingly rare — but the `ORDER BY (COALESCE(pe.likes, 0) + ...)` engagement sort is effectively random.

This is the "Bug D deferred" from v20 Phase 2. The deferral note was wrong: the fix isn't "LEFT JOIN analytics_cache" (same problem), it's "create a separate engagement table for `posts` OR drop the engagement sort."

**Fix** (v21): Drop the engagement sort from `list_posts_search`. Add a proper `post_engagement_for_posts` table in v22 migration.

### B.5 ComposerModal edit-mode bug

**File**: `src/lib/composer/ComposerModal.svelte:290-313`

```ts
async function submit() {
  // ... validation ...
  const r = await postsApi.create(buildPayload());  // ← BUG: always create
  // ...
}
```

`submit()`, `postNow()` (line 315), and `saveAsDraft()` (line 344) all unconditionally call `postsApi.create()` even when `composer.mode === 'edit'`. The `postsApi.update(id, {content, title})` method exists in `src/lib/api/posts.ts:69` but is **never called**.

**Symptom**: User clicks "Edit" on a post → modifies content → clicks "Schedule" → a **duplicate** post is created with the new content; the original post is unchanged.

**Fix**: Branch on `composer.mode === 'edit'` and call `postsApi.update(editingPostId, payload)` instead. Also fetch the post group first (add `GET /api/posts/group/{group_id}` endpoint) so the user can edit all sibling posts in a thread.

### B.6 Recurring schedule silently dropped

**File**: `src/lib/composer/ComposerModal.svelte` `buildPayload()` (around line 250).

`SchedulePicker.svelte` writes to a `recurring` state object (`{ intervalDays: number, endDate: string | null }`), but `buildPayload()` never includes it in the payload. The `postsApi.repeat(id, { interval_days, end_date })` method exists (`src/lib/api/posts.ts`) but is only called from outside the composer.

**Fix**: After `postsApi.create()` succeeds in `submit()`, if `recurring.intervalDays` is set, call `postsApi.repeat(createdPost.id, { interval_days, endDate })`. Show a toast: "Post scheduled + N recurring copies created".

### B.7 `sanitize_content` UTF-8 truncation

**File**: `src/services/posts.rs:63-67`

```rust
if clean.len() > max_len {
    clean[..max_len].to_string()  // panics if max_len is not on a char boundary
}
```

`String::len()` returns byte length; slicing `clean[..max_len]` will **panic** if `max_len` falls in the middle of a multi-byte UTF-8 sequence (e.g., emoji). Also hard-caps at 2000 chars regardless of provider.

**Fix**: Use `chars().take(max_len).collect()` or `char_indices` to find the largest valid boundary.

---

## Part C — Phased Refactor Plan

### Phase 1 — Backend critical fixes (BLOCKING, do first)

**Goal**: Eliminate the calendar 500, the PostState decode 500, and the UTF-8 panic.

| # | File | Change | Acceptance |
|---|---|---|---|
| 1.1 | `src/db/queries.rs:1061-1064` | Replace `(ac.data->>'likes')::bigint` with `NULL::bigint` (Option 2) — drop the dead-weight engagement columns from the calendar query. | `GET /api/calendar?start=2026-07-01&end=2026-07-31` returns 200 even when `analytics_cache` has malformed rows. |
| 1.2 | `src/db/models.rs:103-117` | Add `Idea` variant to `PostState` enum + update `Display`/`FromStr` impls. | `cargo build` succeeds; `query_as!` decoding a row with `state='idea'` returns `PostState::Idea`. |
| 1.3 | `src/db/queries.rs` | Update `list_posts` / `count_posts_by_user` match arms to include `Idea`. | Kanban "Ideas" column loads without 500. |
| 1.4 | `src/services/posts.rs:63-67` | Replace `clean[..max_len].to_string()` with `clean.chars().take(max_len).collect()`. | Post with emoji in content + provider max_len=280 truncates without panic. |
| 1.5 | `src/error.rs:82-85` | Include `sqlx::Error` type tag in the response body (e.g., `{"error":"Database error", "code":"decode_failed"}`) — no SQL details leaked. | Calendar 500 returns a more actionable error code. |

### Phase 2 — Feed CRUD endpoints

**Goal**: Add UPDATE + repurpose endpoints so feed posts have real CRUD.

| # | File | Change | Acceptance |
|---|---|---|---|
| 2.1 | `migrations/027_feed_repurpose.sql` (new) | Add `posts.source_external_post_id UUID REFERENCES external_posts(id) ON DELETE SET NULL` column. | Migration applies cleanly. |
| 2.2 | `src/api/feed.rs` | Add `PUT /api/feed/{id}` handler — accepts `{ text?, media?, metadata? }`, updates `external_posts` row (only if it belongs to current user). Returns updated `FeedPost`. | `curl -X PUT /api/feed/{id} -d '{"text":"new text"}'` returns 200. |
| 2.3 | `src/api/feed.rs` | Add `POST /api/feed/{id}/repurpose` handler — creates a new `posts` row with `content = external_post.text`, `media = external_post.media`, `source_external_post_id = external_post.id`. Returns the new `PostPublic`. | Repurposed post appears in `/api/posts` list with provenance FK set. |
| 2.4 | `src/api/feed.rs:284-321` | Make `GET /api/feed/{id}/comments` read from `cached_comments` first, fall back to live-fetch on cache miss. | Comments load instantly on second view. |
| 2.5 | `src/api/mod.rs:172-178` | Register the 2 new routes. | Routes show in `cargo run serve` startup log. |

### Phase 3 — ComposerModal bug fixes

**Goal**: Fix the duplicate-on-edit bug + wire recurring into payload.

| # | File | Change | Acceptance |
|---|---|---|---|
| 3.1 | `src/lib/composer/ComposerModal.svelte:290` | In `submit()`, branch on `composer.mode === 'edit'` → call `postsApi.update(editingPostId, payload)` instead of `postsApi.create(payload)`. | Editing a post and clicking "Schedule" updates the post; no duplicate is created. |
| 3.2 | `src/lib/composer/ComposerModal.svelte:315` | Same branch in `postNow()`. | Editing + "Post Now" updates and publishes. |
| 3.3 | `src/lib/composer/ComposerModal.svelte:344` | Same branch in `saveAsDraft()`. | Editing + "Save Draft" updates draft. |
| 3.4 | `src/lib/composer/ComposerModal.svelte:250` (`buildPayload`) | Include `recurring: { interval_days, end_date }` in payload if set. | Payload includes `recurring` field when user configures a repeat. |
| 3.5 | `src/api/posts.rs::create` | After creating, if `recurring` is in the request, call `repeat_post` logic inline (or return group_id and let frontend call `/repeat` — current pattern). | Recurring series is created from the composer. |
| 3.6 | `src/lib/stores/composer.svelte.ts` | Add `presetDate` support for `openEdit(postId, presetDate?)` — needed when editing from a calendar slot. | Edit-from-calendar works. |

### Phase 4 — Calendar UX polish (postiz-inspired)

**Goal**: Make the calendar feel postiz-grade.

| # | File | Change | Acceptance |
|---|---|---|---|
| 4.1 | `src/routes/calendar/+page.svelte:80-90` | Replace tiny red `<div>` with: skeleton shimmer during load, full-card error state with **Retry** button + toast on error. | Calendar 500 shows "Couldn't load calendar. [Retry]" instead of "Database error". |
| 4.2 | `src/lib/calendar/MonthView.svelte` + `WeekView.svelte` + `DayView.svelte` | Add `!grayscale cursor-not-allowed` to past cells; disable drop targets. | Past cells visibly grey; can't drop on them. |
| 4.3 | `src/lib/calendar/utils.ts` | Add `isPast(date: Date): boolean` helper; recompute every 2 minutes via `$state` + `setInterval` in `calendarState`. | Tab left open overnight shows yesterday as past. |
| 4.4 | `src/lib/calendar/CalendarEvent.svelte` | Use `post.tags[0].color` as the chip's top bar background (postiz-style). Fallback to current state color if no tags. | Tagged posts show tag color on chip. |
| 4.5 | `src/lib/calendar/CalendarEvent.svelte` | Add red `!` badge (absolute top-left) for posts with `state === 'error'`, with tooltip showing `error_message`. | Failed posts are immediately visible. |
| 4.6 | `src/routes/calendar/+page.svelte` | Sync `?display=week&date=2026-07-07` to URL via `replaceState` (no history pollution). On mount, read URL params to restore state. | Deep-linking to `?display=month&date=2026-08-01` works. |
| 4.7 | `src/lib/calendar/WeekView.svelte` + `DayView.svelte` | On drop of a published post, open `modals.areYouSure` with two buttons: "Just update post details" vs "Reschedule (re-publish)". | Dragging a published post asks for confirmation. |
| 4.8 | `src/lib/calendar/CalendarEvent.svelte` | Add hover-revealed "Preview" icon (opens `/posts/{id}` in new tab or a read-only modal). | Hover shows preview affordance. |

### Phase 5 — Composer UX polish (postiz-inspired)

**Goal**: Make the composer feel postiz-grade.

| # | File | Change | Acceptance |
|---|---|---|---|
| 5.1 | `src/lib/composer/ComposerModal.svelte` | Add `askClose` confirmation: on Escape/backdrop click, if content is non-empty, show `modals.areYouSure("Discard draft?")`. | Accidental Escape doesn't lose drafts. |
| 5.2 | `src/lib/composer/MediaUpload.svelte` | Replace simulated `setInterval` progress with `XMLHttpRequest.upload.onprogress` for real progress. | Upload progress reflects actual bytes uploaded. |
| 5.3 | `src/lib/composer/ComposerModal.svelte` | When creating a new post, auto-append active signature to content (postiz-style `onlyValues: [{content: '\n' + signature.content}]`). | New posts start with signature appended. |
| 5.4 | `src/lib/composer/AiAssistant.svelte` | Show **all** validation errors as per-platform toasts on submit (currently only first error shows). Format: `"X (@handle): post is too long"`. | Submit with 2 invalid platforms shows 2 toasts. |
| 5.5 | `src/lib/composer/SchedulePicker.svelte` | Replace native HTML date/time inputs with a popover-style picker (postiz `DatePicker.tsx`-inspired). Locale-aware format. | Date picker opens as popover, doesn't get clipped at bottom of modal. |
| 5.6 | `src/lib/composer/ComposerModal.svelte` | Add `ThreadFinisher`-style generic "Add comment or post" row button (postiz-style) — currently only X has thread support. Per-row delay input. | LinkedIn/FB composers can add a first-comment row. |
| 5.7 | `src/lib/stores/composer.svelte.ts` | Add `prefilledMedia: Media[]` field — for repurpose flow to pre-fill media from the source feed post. | Repurpose from feed opens composer with media pre-filled. |

### Phase 6 — Replace native confirm/prompt with `modals.areYouSure`

**Goal**: Unify destructive-action UX.

13+ call sites to migrate:
- `src/routes/posts/+page.svelte:158` (bulk delete)
- `src/routes/posts/+page.svelte:195` (bulk reschedule via 2 sequential `prompt()` calls — replace with a `RescheduleModal`)
- `src/routes/calendar/+page.svelte:61, 264`
- `src/routes/automation/+page.svelte:83`
- `src/routes/kanban/+page.svelte:141, 154`
- `src/routes/settings/developer/+page.svelte`
- `src/routes/settings/signatures/+page.svelte`
- `src/routes/settings/profile/+page.svelte`
- `src/routes/settings/webhooks/+page.svelte`
- `src/lib/rss/RssFeedCard.svelte`
- `src/lib/composer/PostSetModal.svelte`
- `src/lib/channels/ChannelCard.svelte`

| # | File | Change | Acceptance |
|---|---|---|---|
| 6.1 | All 13 call sites above | Replace `confirm()` with `await modals.areYouSure({ title, body, confirmLabel, danger })`. Replace `prompt()` with a small `PromptModal` component (new) using the same modal manager. | No native browser dialogs anywhere in the app. |
| 6.2 | `src/lib/components/PromptModal.svelte` (new) | Reusable prompt modal with input field, validation, confirm/cancel. | Used by bulk-reschedule flow. |
| 6.3 | `src/routes/posts/+page.svelte:195` (bulk reschedule) | Replace 2 sequential `prompt()` calls with a single `RescheduleBulkModal` showing date+time picker + preview of affected posts. | Bulk reschedule has a real UI. |

### Phase 7 — Posts CRUD polish (group endpoint + soft delete)

**Goal**: Close the remaining CRUD gaps vs Postiz.

| # | File | Change | Acceptance |
|---|---|---|---|
| 7.1 | `src/api/posts.rs` | Add `GET /api/posts/group/{group_id}` handler — returns all posts sharing the group_id (for thread editing). | Frontend can fetch a thread group for editing. |
| 7.2 | `src/lib/api/posts.ts` | Add `postsApi.getGroup(groupId)` method. | Composer edit-mode can fetch siblings. |
| 7.3 | `src/lib/composer/ComposerModal.svelte` | In `openEdit`, fetch the group first; if multiple posts, show tab strip to switch between thread parts. | Editing a thread shows all parts. |
| 7.4 | `migrations/028_posts_soft_delete.sql` (new) | Add `posts.deleted_at TIMESTAMPTZ` column. Update all `list_posts`/`get_post_by_id`/`get_calendar_posts` queries to filter `WHERE deleted_at IS NULL`. | Deleted posts are recoverable. |
| 7.5 | `src/api/posts.rs::delete` | Change hard cascade-delete to soft-delete (`UPDATE posts SET deleted_at = NOW() WHERE id = $1 OR group_id = (SELECT group_id FROM posts WHERE id = $1)`). | Delete is reversible. |
| 7.6 | `src/lib/api/posts.ts` | Add `postsApi.undelete(id)` method + admin "Trash" page. (Deferred to v22 — just add the API method now.) | Method exists. |

---

## Part D — Implementation Status

### D.1 What this iteration fixes (v21)

**Backend** (in `src/db/queries.rs`, `src/db/models.rs`, `src/services/posts.rs`, `src/api/feed.rs`, `src/api/posts.rs`, `src/api/mod.rs`, `src/error.rs`, `migrations/027_*.sql`):

- ✅ Calendar 500 root cause (drop dead-weight engagement columns)
- ✅ `PostState::Idea` variant added
- ✅ `sanitize_content` UTF-8 panic fix
- ✅ `PUT /api/feed/{id}` (update feed post)
- ✅ `POST /api/feed/{id}/repurpose` (convert feed post to Social Forge post)
- ✅ `GET /api/posts/group/{group_id}` (for thread editing)
- ✅ `posts.deleted_at` soft-delete column + migration 028
- ✅ `AppError::Database` includes error code

**Frontend** (in `src/lib/composer/ComposerModal.svelte`, `src/lib/stores/composer.svelte.ts`, `src/lib/calendar/*`, `src/routes/calendar/+page.svelte`, `src/lib/api/posts.ts`, `src/lib/api/feed.ts`, `src/routes/feed/+page.svelte`, `src/lib/components/PromptModal.svelte`):

- ✅ ComposerModal edit-mode bug fix (calls `postsApi.update()` when editing)
- ✅ Recurring schedule wired into payload
- ✅ Calendar error UI: skeleton + Retry button + toast
- ✅ Past-cell greying + drop disable
- ✅ Tag-colored chip top bar
- ✅ Per-post error `!` badge with tooltip
- ✅ URL sync (`?display=&date=`) via `replaceState`
- ✅ "Just update vs Reschedule" modal on published-post drag
- ✅ `askClose` confirmation on composer close
- ✅ Real upload progress (XMLHttpRequest)
- ✅ Per-platform validation error toasts (show all, not just first)
- ✅ Signature auto-append on new post
- ✅ Feed Repurpose button calls `POST /api/feed/{id}/repurpose` (real backend call)
- ✅ Feed Edit modal (calls `PUT /api/feed/{id}`)
- ✅ Replace 13 native `confirm()`/`prompt()` calls with `modals.areYouSure` + new `PromptModal`
- ✅ Bulk reschedule modal (replaces 2 sequential `prompt()` calls)

### D.2 What is deferred to v22

| Item | Why deferred |
|---|---|
| `intervalInDays` single-row recurring model (postiz-style) | Schema migration + client virtualization — larger lift, do in dedicated sprint. |
| `POST /posts` with `type: 'draft'\|'schedule'\|'now'\|'update'` discriminator | API redesign — breaking change. |
| Transactional outbox for publishes | Needs `publish_outbox` table + `SELECT FOR UPDATE SKIP LOCKED` drain loop. Worth it but non-trivial. |
| Idempotency keys on `provider.publish()` | Needs provider-side support; not all providers accept idempotency keys. |
| Per-platform task queues (Temporal-style) | Out of scope — single-instance tokio is fine for solo deployment. |
| CopilotKit in-composer AI | Out of scope — needs CopilotKit subscription + backend integration. |
| AI Generator (NDJSON streaming) | Out of scope — needs agent graph backend. |
| AI Image with 14 style chips | Out of scope — needs image-gen provider. |
| Generic thread builder (per-row delay for all providers, not just X) | Phase 5.6 — included in v21. |
| Full i18n (15 locales) | Out of scope — single-user English-only is fine. |
| Split god modules (`queries.rs` 2599 lines, `integrations.rs` 1433 lines, `onboard.rs` 1612 lines) | Pure refactor — do in v22 with no behavior change. |
| `post_engagement_for_posts` table (proper engagement FK) | Phase 1 Option 3 — deferred per recommendation. |
| Mantine-style date picker popover | Phase 5.5 — included in v21 (custom Svelte popover, not Mantine). |
| `Sets` auto-open before composer | Minor; defer. |
| Drop unused `teams`/`subscriptions` schema | Drop in v22 migration after confirming no frontend dependency. |
| Convert all `query_as::<_, T>` to `query_as!` | Needs `.sqlx` cache regeneration — separate task. |

### D.3 Architectural preferences preserved

All v21 changes respect the social-forge architectural preferences:
- ✅ Single Rust binary (no new services, no Temporal, no Redis)
- ✅ SvelteKit SPA embedded via `rust-embed`
- ✅ Single-user assumption (`DEFAULT_USER_ID` unchanged)
- ✅ No external queue (in-process tokio scheduler retained)
- ✅ No microservices
- ✅ No new third-party frontend libs (Svelte 5 runes + existing TipTap + existing custom UI)

---

## Appendix — File Reference

### Critical files modified in v21

**Backend (Rust)**:
- `src/db/queries.rs` — calendar SQL fix, `list_posts` match arms, group fetch
- `src/db/models.rs` — `PostState::Idea` variant
- `src/services/posts.rs` — UTF-8 truncation fix
- `src/api/feed.rs` — `PUT` + `POST /repurpose` endpoints, comments cache fix
- `src/api/posts.rs` — `GET /group/{group_id}` endpoint, soft-delete in `delete`
- `src/api/mod.rs` — register new routes
- `src/error.rs` — error code in Database response
- `migrations/027_feed_repurpose.sql` (new)
- `migrations/028_posts_soft_delete.sql` (new)

**Frontend (SvelteKit)**:
- `src/lib/composer/ComposerModal.svelte` — edit-mode branch, recurring payload, askClose, per-platform error toasts, signature auto-append
- `src/lib/composer/MediaUpload.svelte` — real upload progress
- `src/lib/composer/SchedulePicker.svelte` — popover-style date picker
- `src/lib/stores/composer.svelte.ts` — `prefilledMedia`, `presetDate` for edit
- `src/lib/calendar/CalendarEvent.svelte` — tag-colored bar, error `!` badge
- `src/lib/calendar/MonthView.svelte` / `WeekView.svelte` / `DayView.svelte` — past-cell greying, drop disable
- `src/lib/calendar/utils.ts` — `isPast()` helper
- `src/lib/stores/calendar.svelte.ts` — past-cell recompute interval
- `src/routes/calendar/+page.svelte` — error retry UI, URL sync, "Just update vs Reschedule" modal trigger
- `src/lib/api/posts.ts` — `getGroup`, `undelete` methods
- `src/lib/api/feed.ts` — `update`, `repurpose` methods
- `src/routes/feed/+page.svelte` — Edit modal, Repurpose button calls backend
- `src/lib/components/PromptModal.svelte` (new) — reusable prompt modal
- `src/lib/components/RescheduleBulkModal.svelte` (new) — bulk reschedule UI
- 13 files replacing native `confirm()`/`prompt()`

### Reference files (postiz-app, for inspiration)

| Concern | Path |
|---|---|
| Calendar grid + chip | `apps/frontend/src/components/launches/calendar.tsx` |
| Calendar SWR + URL sync | `apps/frontend/src/components/launches/calendar.context.tsx` |
| Composer modal | `apps/frontend/src/components/new-launch/manage.modal.tsx` |
| Modal manager (Zustand) | `apps/frontend/src/components/layout/new-modal.tsx` |
| Posts controller | `apps/backend/src/api/routes/posts.controller.ts` |
| Posts service | `libraries/nestjs-libraries/src/database/prisma/posts/posts.service.ts` |
| Temporal workflow | `apps/orchestrator/src/workflows/post-workflows/post.workflow.v1.0.5.ts` |
| Prisma schema | `libraries/nestjs-libraries/src/database/prisma/schema.prisma` |
| Delete dialog | `libraries/react-shared-libraries/src/helpers/delete.dialog.tsx` |

---

**End of audit & plan. Implementation begins next.**
