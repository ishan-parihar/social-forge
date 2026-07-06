# Social Forge — Full-Scale Audit Report (v15)

**Date:** 2026-07-06  
**Scope:** Frontend (`frontend/src/`), Backend (`src/`), MCP (`src/mcp/`), Services (`src/services/`), Config  
**Method:** Read-only code audit, no files modified

---

## Executive Summary

| Category | Count | Impact |
|---|---|---|
| **FIX** (broken) | 18 | Blocks user workflows |
| **REFINE** (poor UX) | 18 | Erodes trust, adds friction |
| **UPGRADE** (missing) | 8 | Would significantly improve solo-founder experience |
| **YAGNI** (dead code) | 12 | Maintenance burden, confusion |
| **Total findings** | **56** | |

---

## FIX — Broken Functionality (18 items)

### F-1. Dashboard engagement totals are mislabeled and wrong
- **File:** `frontend/src/routes/+page.svelte:20-29, 44, 143-173`
- **Current:** Engagement totals computed from 5 most-recent published posts, labeled as "7d"
- **Fix:** Replace with `feedApi.analytics(7)` which returns real 7-day totals from `external_posts JOIN post_engagement`

### F-2. Dashboard "Today's Schedule" badge shows platform name in state-colored pill
- **File:** `frontend/src/routes/+page.svelte:250`
- **Current:** `badge-{post.state}` background with `{post.integration_name}` text — confusing
- **Fix:** Use `<Badge state={post.state} />` for the state, show platform name separately

### F-3. Dashboard alerts don't deep-link to filtered views
- **File:** `frontend/src/routes/+page.svelte:215-230`
- **Current:** All links go to bare `/posts` — no state filter
- **Fix:** Change to `/posts?state=error`, `/posts?state=draft`; update `posts/+page.svelte` to read URL params

### F-4. No logout button anywhere
- **File:** `frontend/src/routes/+layout.svelte` — sidebar has no logout
- **Current:** `auth.logout()` defined but never called; user can't end session from UI
- **Fix:** Add "Log out" button in sidebar footer

### F-5. Search page "Import Feed" button has no text when idle
- **File:** `frontend/src/routes/search/+page.svelte:188`
- **Current:** `{importing ? "Importing..." : ""}` — renders empty button
- **Fix:** Change to `"Import Feed"`

### F-6. Theme toggle doesn't actually change Tailwind colors
- **Files:** `tailwind.config.js:21-30`, `app.css:5-42`, `theme.svelte.ts`
- **Current:** Tailwind config hardcodes hex colors; CSS variables in `app.css` only affect `body` background
- **Fix:** Convert Tailwind semantic colors to CSS variables: `surface: "var(--bg-card)"` etc.

### F-7. User-selected timezone is ignored almost everywhere
- **Files:** `+page.svelte:248`, `posts/+page.svelte:135,162`, `WeekView.svelte:55`, `calendar/utils.ts:76-79`
- **Current:** `timezone.formatDateTime()` exists but `toLocaleString()` is used instead
- **Fix:** Replace all date formatting with `timezone.format*()` calls

### F-8. Post detail page shows raw integration UUID
- **File:** `frontend/src/routes/posts/[id]/+page.svelte:142`
- **Current:** `{post.integration_id}` renders UUID; `integration_name` available but unused
- **Fix:** Use `post.integration_name`

### F-9. RSS feed form submit button is unstyled
- **File:** `frontend/src/lib/rss/RssFeedForm.svelte:117`
- **Current:** `btn btn-primary btn-md` classes are scoped to `Button.svelte` — no styling applies
- **Fix:** Use `<Button>` component instead of raw `<button>`

### F-10. Realtime store doesn't subscribe to comment/dm events
- **File:** `frontend/src/lib/stores/realtime.ts:38-46`
- **Current:** Missing `comment_received`, `dm_received` from event subscription list
- **Fix:** Add both events to the subscription array

### F-11. Keyboard shortcut "g d" navigates to non-existent `/dashboard`
- **File:** `frontend/src/lib/stores/keyboard.svelte.ts:76`
- **Current:** `goto('/dashboard')` — route doesn't exist (dashboard is at `/`)
- **Fix:** Change to `goto('/')`

### F-12. Composer has duplicate `page-enter` class
- **Files:** `posts/new/+page.svelte:372`, `posts/[id]/+page.svelte:125`, `tags/+page.svelte:117`
- **Fix:** Remove duplicate `page-enter`

### F-13. Composer action bar is NOT sticky
- **File:** `frontend/src/routes/posts/new/+page.svelte:383-405`
- **Current:** Action buttons scroll off-screen as user scrolls down
- **Fix:** Wrap in `sticky top-0 z-20 bg-background` container or sticky footer

### F-14. DMs page has duplicate type definitions shadowing imports
- **File:** `frontend/src/routes/dms/+page.svelte:8-51`
- **Current:** Local interfaces shadow imported API types — type drift risk
- **Fix:** Delete local interfaces, use imported types

### F-15. MediaUpload "Choose from library" never renders in composer
- **File:** `frontend/src/routes/posts/new/+page.svelte:546-549`
- **Current:** `onInsertUrl` prop not passed — "Choose from library" button hidden
- **Fix:** Pass `onInsertUrl` handler or refactor MediaUpload to handle library internally

### F-16. Composer `fetchTargets` error toast has redundant nested Error check
- **File:** `frontend/src/routes/posts/new/+page.svelte:190`
- **Fix:** Simplify `e instanceof Error ? (e instanceof Error ? e.message : String(e))` to `e instanceof Error ? e.message : String(e)`

### F-17. Dashboard `alertCount` counts errors twice (dead logic)
- **File:** `frontend/src/routes/+page.svelte:31`
- **Current:** `stats.error + upcoming.filter(p => p.state === 'error').length` — second term always 0
- **Fix:** Simplify to `stats.error`

### F-18. Posts list doesn't read URL params for initial filter
- **File:** `frontend/src/routes/posts/+page.svelte:73`
- **Current:** `filter = $state("all")` — ignores `?state=error` in URL
- **Fix:** Read `$page.url.searchParams.get('state')` in `onMount`

---

## REFINE — Poor UX, Missing Polish (18 items)

### R-1. Dashboard lacks a "needs attention" inbox
- Aggregating: unread comments, unread DMs, failed posts, expiring tokens in one place

### R-2. Dashboard doesn't surface token-expiry alerts
- `Integration.refresh_needed` field exists but never checked on dashboard

### R-3. Today's Schedule capped at 5 with no "view all" link
- `+page.svelte:46` slices to 5; no overflow indicator

### R-4. Composer ignores per-provider char counters
- `lib/composer/providers/` directory + `CharCountEditor.svelte` are dead code
- Per-platform 280/3000/2200 char limits not shown

### R-5. ChannelSelector shows misleading clock emoji on every channel
- `🕐` implies best-time computed; actually not

### R-6. RSS feed badge says "published"/"draft" instead of "Enabled"/"Disabled"
- `RssFeedCard.svelte:92` — semantic mismatch

### R-7. Calendar `handleDuplicate` loses media, tags, first comment
- Only copies content + title + schedule

### R-8. Six different hardcoded provider label/icon maps scattered across pages
- Should be centralized in `ProviderIcon.svelte` or a shared helper

### R-9. ListView has dead pagination props
- `page`, `totalPages`, `onPageChange` never passed by calendar page

### R-10. WeekView header hardcodes "GMT"
- Should show user's selected timezone abbreviation

### R-11. Search is client-side over only 100 posts
- `feedApi.list(undefined, undefined, undefined, 100)` — older posts invisible

### R-12. Toast notifications can be lost on initial page load
- Race condition: `toast()` fires before `Toast.svelte` mounts its listener

### R-13. Comments page subscribes to wrong realtime events
- Subscribes to `post_published`/`post_created`; should subscribe to `comment_received`

### R-14. No duplicate button on posts list page
- Calendar has duplicate; posts list doesn't

### R-15. No bulk actions on posts list
- Calendar has bulk select + delete + reschedule; posts list doesn't

### R-16. Settings pages have no breadcrumb / back navigation
- No "← Settings" link on sub-pages

### R-17. Posts list `page` variable name conflicts with Svelte convention
- `let page = $state(1)` — rename to `currentPage`

### R-18. Composer `postNow` missing `submitting = false` on early return (partial)
- Actually works due to `finally` block, but error path skips draft cleanup — correct behavior, but confusing code

---

## UPGRADE — Missing Features (8 items)

### U-1. Unified global search across posts, comments, DMs, media
- Currently search only covers 100 most-recent feed posts (client-side)
- Need backend `?q=` parameter + tabbed UI

### U-2. Drafts inbox on dashboard
- Show 5 most-recent drafts with click-to-edit

### U-3. Per-platform character counter in composer
- Wire up existing `CharCountEditor` dead code

### U-4. Composer keyboard shortcuts (Cmd+Enter, Cmd+S)
- Currently only global shortcuts; no composer-specific ones

### U-5. Duplicate button on posts list
- (Also listed as R-14)

### U-6. Bulk actions on posts list
- (Also listed as R-15)

### U-7. Onboarding checklist persistence
- Track progress in localStorage; show persistent "Getting started" widget

### U-8. Mobile-responsive sidebar
- Add hamburger toggle for screens < lg

---

## YAGNI — Dead Code, Unused Components (12 items)

### Y-1. `lib/ui/Card.svelte` — never imported (0 references)
- **Action:** Delete

### Y-2. `lib/analytics/AnalyticsTable.svelte`, `AnalyticsSummaryCards.svelte`, `AnalyticsCharts.svelte` — never imported
- **Action:** Delete all three

### Y-3. `lib/composer/ProviderEditor.svelte` + entire `lib/composer/providers/` directory — never used
- 12 files: ProviderEditor + index.ts + 10 provider editors + CharCountEditor
- **Action:** Either delete (YAGNI) OR wire into composer for per-platform char limits (preferred — see R-4/U-3)

### Y-4. `lib/developer/WebhookForm.svelte` — never imported
- **Action:** Delete

### Y-5. `lib/api/developer.ts` — duplicated webhook methods (6 methods, all unused)
- **Action:** Delete webhook methods; keep only API key methods

### Y-6. `lib/api/webhooks.ts:30` — `get` method never called
- **Action:** Delete

### Y-7. `lib/api/auth.ts:8-9` — `logout` method never called
- **Action:** Wire up F-4 (logout button) — don't delete

### Y-8. `lib/calendar/utils.ts:53-55` — `dateKey` is exact duplicate of `formatDateKey`
- **Action:** Delete `dateKey`

### Y-9. `routes/dms/+page.svelte:62` — dead `platforms` array
- **Action:** Delete

### Y-10. `routes/dms/+page.svelte:8-51` — duplicate type definitions
- **Action:** Delete, use imported types (same as F-14)

### Y-11. `lib/calendar/ListView.svelte` — dead pagination props + footer
- **Action:** Either wire pagination or delete dead props

### Y-12. `lib/composer/MediaUpload.svelte` — `onInsertUrl` prop + MediaPopover unreachable
- **Action:** Either wire up (F-15) or delete dead code

---

## Backend Audit (from code knowledge, not sub-agent)

### FIX
| # | File | Issue |
|---|---|---|
| B-1 | `src/api/comments.rs:99-105` | `resolve` endpoint is a stub — doesn't persist resolved state to DB |
| B-2 | `src/api/dms.rs:138-149` | `get_messages` uses any integration's token instead of the selected one |
| B-3 | `src/api/comments.rs:41-88` | Fetches comments live (50 sequential network calls per page load) — needs caching |
| B-4 | `src/social/mod.rs:448-454` | Most providers don't implement `get_post_comments` — returns empty silently |

### REFINE
| # | File | Issue |
|---|---|---|
| B-5 | `src/services/posts.rs:51-68` | `sanitize_content` hard-caps at 2000 chars — should respect per-provider `max_content_length()` |
| B-6 | `src/api/feed.rs` | No search parameter — frontend can only do client-side search on 100 posts |

### YAGNI
| # | File | Issue |
|---|---|---|
| B-7 | `src/cli/platforms/` | 32 platform shims — verify each is still used by `cli/run.rs` dispatch |
| B-8 | `src/mcp/mod.rs` | McpJsonValue wrapper (198 call sites) — audit said to shrink; deferred as too risky |
| B-9 | `src/services/posts.rs` | `sanitize_content` HTML tag stripper is hand-rolled — could use `ammonia` crate but YAGNI if it works |

---

## Recommended Fix Priority

### Tier 1 — Immediate (blocks user workflows)
1. F-13: Sticky composer action bar
2. F-1, F-2, F-3: Dashboard engagement numbers + alert deep links
3. F-4: Logout button
4. F-9: RSS form button unstyled
5. F-11: Keyboard shortcut to non-existent route
6. F-5: Empty import button text

### Tier 2 — High value (erodes trust)
7. F-7: Timezone rendering everywhere
8. F-8: UUID shown instead of integration name
9. F-10: Realtime comment/dm events
10. F-6: Theme toggle doesn't work
11. R-1, R-2, R-3: Dashboard needs-attention inbox + token alerts + view-all
12. R-7: Duplicate loses media/tags

### Tier 3 — Quick wins (dead code removal)
13. Y-1, Y-2, Y-4, Y-8, Y-9: Delete unused components/helpers
14. Y-5, Y-6: Delete unused API methods
15. F-12, F-16, F-17: Minor code cleanup

### Tier 4 — Feature upgrades
16. U-1: Global search with backend `?q=` param
17. U-2: Drafts inbox on dashboard
18. U-3: Per-platform char counter (wire up dead code)
19. R-14, R-15: Duplicate + bulk actions on posts list
20. U-8: Mobile responsive sidebar
