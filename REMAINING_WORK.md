# Remaining Work — Social Forge

> **Last updated:** 2026-06-29
> **Source:** Consolidated from PLAN_REMAINING_WORK, PLAN_MCP_CLI_PARITY, PLAN_COMMENT_DM_AUTOMATION, PLAN_CAROUSEL_SOCIALFORGE_INTEGRATION, IMPLEMENTATION_PLAN

---

## Executive Summary

Social-forge has a solid foundation: 21+ platform providers, MCP + CLI + REST API triple interface, scheduler, media pipeline, comment/DM/automation infrastructure, and a functional SvelteKit frontend. This document captures everything remaining to reach production-ready status, ordered by impact.

**Estimated total:** ~20-25 hours of implementation work.

---

## P0 — Critical Data Flow Bugs

These are bugs where user-facing features silently lose data.

### 1. Frontend not sending media in post create API call
**Impact:** Media uploaded through the web UI is silently lost when posts are published.
**File:** `frontend/src/routes/posts/new/+page.svelte`
**Fix:** Add `media: mediaItems.length > 0 ? mediaItems : undefined` to the `submit()` API call. The backend already accepts `media` in `CreatePostRequest` and the scheduler already parses it — only the frontend is missing the wire-up.

### 2. Frontend not sending first_comment in post create API call
**Impact:** First comments set in the composer are silently discarded.
**File:** `frontend/src/routes/posts/new/+page.svelte`
**Fix:** Ensure `first_comment` is included in the `submit()` payload. The backend and scheduler already handle it end-to-end.

### 3. Frontend not sending per-channel content overrides
**Impact:** Per-provider content customization in the composer has no effect.
**File:** `frontend/src/routes/posts/new/+page.svelte`
**Fix:** When `providerOverride` has entries, include them as the `overrides` field in the API call. Backend already supports `overrides: HashMap<String, PostOverride>` and creates per-integration posts.

---

## P1 — Missing Core Workflows

### 4. Calendar "create post on date click"
**Impact:** Users must navigate away from the calendar to the composer to create a post.
**Files:**
- `frontend/src/routes/calendar/+page.svelte` — add `onDateClick` handler
- `frontend/src/routes/posts/new/+page.svelte` — read `?date=` query param on mount, pre-fill schedule picker

### 5. `find_slot` should use `posting_times` from integrations
**Impact:** Auto-schedule ignores optimal posting times, always adds 2h to last post.
**File:** `src/api/posts.rs` — `find_slot` handler
**Fix:** Query the integration's `posting_times` JSONB, find the next available slot matching one of those times, skip already-occupied slots.

### 6. Recurring posts not sent from frontend
**Impact:** The recurring post feature exists in the backend but the composer never sends the recurring config.
**File:** `frontend/src/routes/posts/new/+page.svelte`
**Fix:** After creating a post, if `recurring` state is set, call `POST /api/posts/{id}/repeat` with `interval_days` and `end_date`.

### 7. Post editing from calendar detail panel
**Impact:** Calendar PostDetail shows info but can't edit inline.
**File:** `frontend/src/lib/calendar/PostDetail.svelte`
**Fix:** Add "Edit" button that navigates to `/posts/{id}` (the existing detail page has edit capability).

---

## P2 — Media Pipeline (Carousel Integration)

### 8. Provider `resolve_media_url()` for URL-dependent platforms
**Impact:** Instagram, Facebook, Threads require publicly accessible URLs but `/api/media/{id}` is only accessible from the server's own network.
**Files:**
- `src/social/mod.rs` — add `resolve_media_url()` default method to `SocialProvider` trait
- `src/social/instagram.rs`, `src/social/facebook.rs`, `src/social/threads.rs` — override to prepend `config.app_url` for relative URLs
- `src/services/posts.rs` — call `resolve_media_url()` before `provider.publish()`

### 9. `create_carousel_post` composite MCP tool
**Impact:** AI agents must manually orchestrate upload → stage → publish for carousels.
**File:** `src/mcp/tools_posts.rs`
**Fix:** Add a tool that accepts `local_paths: Vec<String>` + `content` + `integration_ids`, batch-uploads all paths, stages the post with returned media URLs, and returns staged post IDs.

### 10. `social-forge carousel` CLI command
**Impact:** CLI users have no carousel shortcut.
**File:** `src/cli/mod.rs`, `src/cli/run.rs`
**Fix:** Add `Carousel` subcommand that wraps the composite MCP tool.

### 11. Per-provider media validation
**Impact:** Users get cryptic API errors when violating platform media limits.
**Files:** Each provider's `publish()` implementation
**Fix:** Add `validate_media()` to `SocialProvider` trait: IG max 10 items, no mixed media types; X max 4 images; etc.

### 12. Update AGENTS.md with new media tools
**File:** `AGENTS.md`
**Fix:** Document `posts_media_upload_from_path`, `posts_media_upload_batch`, `posts_media_upload_from_url` tools and the `media upload-batch` CLI command.

---

## P3 — Comment/DM/Automation Frontend

### 13. Comment management page (`/comments`)
**Impact:** No web UI for viewing/replying to comments across platforms.
**Route:** `frontend/src/routes/comments/+page.svelte`
**Features:** Unified inbox, filter by platform/post/date, inline reply, mark as read.

### 14. DM management page (`/dms`)
**Impact:** No web UI for reading/sending DMs.
**Route:** `frontend/src/routes/dms/+page.svelte`
**Features:** Conversation threading, send DM from UI, media attachments.

### 15. Automation rules UI (`/automation`)
**Impact:** Users must use MCP/CLI to manage automation rules.
**Route:** `frontend/src/routes/automation/+page.svelte`
**Features:** Create/edit rules, test with sample triggers, view execution logs, toggle on/off.

---

## P4 — CLI Parity (Remaining Gaps)

### 16. Posts CLI commands
**Commands:** `posts create`, `posts publish`, `posts list`, `posts get`, `posts delete`
**Priority:** Medium — MCP tools exist, CLI wrappers needed.

### 17. Posts `find-slot` CLI command
**Command:** `posts find-slot [--integration-id <id>]`
**Priority:** Low.

### 18. Media `list` CLI command
**Command:** `media list [--limit N]`
**Priority:** Low — MCP tool exists.

---

## P5 — UX Polish

### 19. Draft auto-save in composer
**Fix:** Use `localStorage` to persist composer state with debounce-save. Restore on mount.

### 20. Proper date picker for rescheduling
**File:** `frontend/src/routes/posts/[id]/+page.svelte`
**Fix:** Replace `prompt()` with the `SchedulePicker` component used in the composer.

### 21. "Post Now" button in composer
**Fix:** Add second button that calls `postsApi.create()` without `scheduled_at`, then immediately calls `POST /api/posts/{id}/publish`.

### 22. Bulk operations in calendar/post list
**Fix:** Checkbox selection, floating action bar with "Delete All" / "Reschedule All". Backend: `POST /api/posts/bulk-delete` and `POST /api/posts/bulk-schedule`.

---

## Completed Work (Reference)

These items were marked as incomplete in the original plans but have been verified as done:

| Item | Evidence |
|------|----------|
| Scheduler passes media to providers | `src/scheduler/mod.rs` parses `post.media` into `Vec<MediaAttachment>` |
| first_comment published after main post | `src/scheduler/mod.rs` calls `provider.comment()` after successful publish |
| Backend supports per-channel overrides | `src/api/posts.rs` has `overrides: HashMap<String, PostOverride>` |
| MCP media upload from path/batch/URL | Committed `883879e` — `src/mcp/tools_media.rs` |
| CLI `media upload-batch` command | Committed `883879e` — `src/cli/mod.rs` |
| MCP bridge for new media tools | Committed `883879e` — `src/cli/mcp_bridge.rs` |
| SocialProvider trait DM methods | Implemented in `src/social/mod.rs` |
| Generic comment/DM/automation MCP tools | `src/mcp/tools_comments.rs`, `tools_dm.rs`, `tools_automation.rs` |
| Platform-specific: X, LinkedIn, Instagram reply/DM | Commits `d209fef`, `09c6a62`, `6884da1` |
| Platform-specific: YouTube, Bluesky, Mastodon reply | Commit `6884da1` |
| CLI commands for X/LinkedIn/Instagram DMs | Commit `07c3738` |
| CLI commands for YouTube/Bluesky/Mastodon | Commit `281b930` |
| CLI commands for Generic Comment/DM/Automation | Commit `281b930` |
| Automation engine (rules, cooldowns, AI) | `src/services/automation.rs` |
| Automation DB migration | `migrations/017_automation.sql` |
| Content splitter | `src/services/content_splitter.rs` |
| Staging tool | `src/services/staging.rs` |

---

## Execution Order

```
Phase 1 (P0 — fix data loss bugs, ~1 hour):
  1 → 2 → 3  (frontend media, first_comment, overrides)

Phase 2 (P1 — core workflows, ~3 hours):
  4 → 5 → 6 → 7  (calendar click, smart slots, recurring, edit)

Phase 3 (P2 — media pipeline, ~4 hours):
  8 → 9 → 10 → 11 → 12  (URL resolution, carousel tool, CLI, validation, docs)

Phase 4 (P3 — frontend UI, ~6 hours):
  13 → 14 → 15  (comments, DMs, automation pages)

Phase 5 (P4 — CLI parity, ~2 hours):
  16 → 17 → 18  (posts CLI, find-slot, media list)

Phase 6 (P5 — polish, ~4 hours):
  19 → 20 → 21 → 22  (auto-save, date picker, post now, bulk ops)
```
