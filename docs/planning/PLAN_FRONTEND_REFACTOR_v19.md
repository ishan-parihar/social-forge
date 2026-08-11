# Social Forge v19 — Post-v18 Gap Analysis & Upgrade Plan

**Contrast audit:** Social Forge v18 (SvelteKit + Rust/Axum) vs Postiz-app (NextJS + NestJS)
**Goal:** Close the remaining UX gaps identified in the post-v18 contrast audit, while preserving Social Forge's architectural preferences (single-user, single-binary, triple-interface parity, security-first, realtime-first, YAGNI).
**Date:** 2026-07-06
**Status:** Plan ready for implementation
**Builds on:** v18 (8 phases, commits d545a8a→029a656, all on origin/master)

---

## Executive Summary

v18 re-architected the calendar and posting flows from page-based to modal-based, matching Postiz's core UX. This v19 plan closes the **remaining 10 gaps** identified in a fresh post-v18 contrast audit of postiz-app. Each gap was filtered against Social Forge's 6 architectural preferences (AGENTS.md §0.5):

- ❌ **Skipped (violates single-user):** Customer/group organization (#12) — Postiz's multi-tenant customer feature has no place in a solo-founder app.
- ❌ **Skipped (YAGNI — heavy dep):** CopilotKit AI assistant (#6) — Social Forge already has its own AiAssistant component; adding CopilotKit (1.10.6 + runtime endpoint) doubles the AI surface for marginal gain.
- ❌ **Skipped (dormant in Postiz too):** Merge/Separate post groups (#4) — zero references in Postiz's own codebase; not worth porting.
- ⚠️ **Deferred (heavy dep):** Uppy media uploader (#7) — 10+ Uppy packages + storage plugin selection is a large lift; v19 improves the existing MediaUpload incrementally instead.
- ✅ **Adopted (10 phases):** The remaining gaps, prioritized by impact and effort below.

---

## Part A — Post-v18 Gap Analysis (10 items)

| # | Gap | Postiz has it? | v18 has it? | Effort | Adopt? |
|---|---|---|---|---|---|
| 1 | Per-post statistics modal with charts | ✅ Full | ❌ No | Medium | ✅ Yes |
| 2 | TimeTable per-channel time presets + calendar ghost slots | ✅ Full | ❌ No | Medium | ✅ Yes |
| 3 | Onboarding flow (2-step post-signup modal) | ✅ Full | ❌ No | Low | ✅ Yes |
| 4 | Calendar list view with state filter + pagination | ✅ Full | ⚠️ Partial | Medium | ✅ Yes |
| 5 | Per-channel ⋯ context menu | ✅ Full (10 items) | ❌ No | Medium | ✅ Yes |
| 6 | Media library upgrade (search, reorder, alt text) | ✅ Full (Uppy) | ⚠️ Basic | Medium | ✅ Yes (no Uppy) |
| 7 | Settings page restructure (sidebar tabs) | ✅ Full (8 tabs) | ⚠️ Flat | Medium | ✅ Yes |
| 8 | Mobile OAuth route + responsive improvements | ⚠️ Partial | ⚠️ Partial | Low-Medium | ✅ Yes |
| 9 | AI Generator (bulk post generation) | ✅ Full | ❌ No | High | ✅ Yes (flagship) |
| 10 | Touch-device DnD fallback for calendar | ❌ No (Postiz also HTML5-only) | ❌ No | Medium | ✅ Yes (differentiator) |

**Explicitly skipped:**
- Customer/group organization — violates single-user preference
- CopilotKit — YAGNI (we have AiAssistant)
- Merge/Separate — dormant in Postiz itself
- Full Uppy adoption — YAGNI (heavy dep for marginal gain over our MediaUpload)

---

## Part B — Implementation Plan (10 Phases)

### Phase 1 — Per-Post Statistics Modal with Charts

**Why:** Postiz's `StatisticsModal` fetches `/analytics/post/{id}?date={7|30|90}` and renders per-metric line charts (impressions, likes, comments) + a short-link click table. Social Forge v18 has a `PostStatsModal` that's a bare slide-in panel with no charts. This is high perceived value for a content-management tool.

**Deliverables:**
- **Backend:** `GET /api/posts/{id}/analytics?days=7|30|90` — returns per-metric time series from `post_engagement` table (already populated by the feed refresher). Shape: `[{ label: "likes", data: [{date, total}], percentageChange, average }]`
- **Frontend:** Upgrade `lib/calendar/PostStatsModal.svelte`:
  - Date-range selector (7 / 30 / 90 days)
  - Grid of metric cards (likes, comments, shares, impressions) — each with a mini line chart
  - Chart library: inline SVG sparkline (no Chart.js dep — YAGNI for 4 charts per modal)
  - Short-link click table (if short links exist for this post)
  - Empty-state message when no analytics data
- Migrate from slide-in panel to ModalManager modal (Phase 0 foundation)

**Files:** ~2 backend + ~1 frontend rewrite
**Effort:** 1 iteration

---

### Phase 2 — TimeTable Per-Channel Time Presets + DayView Ghost Slots

**Why:** Postiz lets each channel have preset posting times (e.g., "always post at 9am and 3pm"). These show as "ghost slots" in DayView — empty drop targets at the preset times — so the user sees when their channel "usually" posts. This is a distinctive calendar UX feature that aids scheduling decisions.

**Deliverables:**
- **Backend:** Add `posting_times: INTEGER[]` column to `integrations` table (minutes-of-day in UTC). New endpoints: `PUT /api/integrations/{id}/posting-times` (save array), existing `GET /api/posts/find-slot` enhanced to return the next preset time if any exist.
- **Frontend:**
  - `lib/channels/TimeTableModal.svelte` — slot editor modal (add HH:MM, see sorted list, delete individual slots). Opened from the per-channel ⋯ menu (Phase 5).
  - `lib/calendar/DayView.svelte` — merge preset times with real posts. For each preset time with no real post, render a ghost slot (dashed border, empty drop target). Clicking a ghost slot opens the composer with that time preset.
- **Migration:** `024_integration_posting_times.sql`

**Files:** ~1 migration + ~2 backend + ~2 frontend
**Effort:** 1 iteration

---

### Phase 3 — Onboarding Flow (2-Step Post-Signup Modal)

**Why:** Postiz's `OnboardingModal` has 2 steps: (1) connect channels, (2) watch tutorial. Social Forge v18 has a `GettingStarted` widget (persistent checklist) and a first-run `OnboardingModal` (welcome only). This phase adds the structured 2-step flow that guides new users from signup to first post.

**Deliverables:**
- Upgrade `lib/onboarding/OnboardingModal.svelte`:
  - Step 1: Channel connect — reuse `ChannelSelector` component in a grid, "Continue" button
  - Step 2: Quick tutorial — embed a YouTube iframe or a static "how to" guide, "Get Started" button
  - Step indicator: "1. Connect Channels → 2. Quick Tour"
  - Skip button on each step
- Gate on `localStorage.getItem('social-forge-onboarded')` (already exists)
- On completion, route to `/calendar` (the v18 default view)

**Files:** ~1 frontend rewrite
**Effort:** 0.5 iteration

---

### Phase 4 — Calendar List View with State Filter + Pagination

**Why:** Social Forge v18's `/posts` page has search/sort/channel-filter/bulk, but the calendar's own list view is basic. Postiz's calendar has a 4th display mode ("list") with server-side pagination, state filter (all/scheduled/draft/published), and date grouping. This phase adds that to the calendar itself.

**Deliverables:**
- `lib/calendar/ListView.svelte` — upgrade:
  - Group posts by date (YYYY-MM-DD), sort ascending, render date headers
  - State filter segmented control (All / Scheduled / Draft / Published)
  - Page navigation (◀ Page N of M ▶)
  - Empty-state messages per filter
- `lib/stores/calendar.svelte.ts` — add `listPage`, `listTotalPages`, `listState` state
- **Backend:** The v18 `GET /api/posts` already supports `?state=` + pagination — reuse it. No backend changes needed.
- Calendar↔list toggle already exists in `CalendarHeader.svelte` — verify it works with the new list view.

**Files:** ~2 frontend
**Effort:** 0.5 iteration

---

### Phase 5 — Per-Channel ⋯ Context Menu

**Why:** Postiz's per-channel menu has 10 items (create post here, copy ID, reconnect, settings, change avatar/nickname, time slots, enable/disable, delete). Social Forge v18's channel management is all on the `/channels` page with no quick actions from the calendar sidebar. This phase adds a ⋯ menu to each channel in the sidebar.

**Deliverables:**
- `lib/channels/ChannelContextMenu.svelte` — dropdown menu with:
  - "Create post for this channel" → `composer.openCreate(undefined, [channelId])`
  - "Copy channel ID"
  - "Reconnect" (if `refresh_needed`) → link to `/channels`
  - "Edit time slots" → opens TimeTableModal (Phase 2)
  - "Disable" / "Enable"
  - "Delete" (with confirm, refuses if posts exist)
- Wire into the sidebar channel list (currently rendered in `+layout.svelte` — add the ⋯ button next to each channel)
- Auto-reposition if near viewport bottom (Postiz pattern)

**Files:** ~1 new + ~1 modified
**Effort:** 1 iteration

---

### Phase 6 — Media Library Upgrade (No Uppy)

**Why:** Postiz uses Uppy (10+ packages) for media uploads with compression, drag/paste, library search, drag-to-reorder, and per-asset alt text. Social Forge v18's `MediaUpload` is basic (file input + library button). This phase upgrades MediaUpload incrementally WITHOUT adopting Uppy (YAGNI — Uppy is too heavy for a solo-founder tool).

**Deliverables:**
- `lib/composer/MediaUpload.svelte` — upgrade:
  - Drag-to-reorder: use native HTML5 DnD on the media thumbnail strip (no library)
  - Per-asset alt text: click a thumbnail → small inline edit popover
  - Clipboard paste: `onpaste` handler on the editor area → upload pasted images
  - Progress bar: per-file upload progress (already have `uploading` state, just surface it better)
- `lib/media/MediaLibraryModal.svelte` — new:
  - Searchable grid of existing media assets (server-side `?q=` on `/api/media`)
  - Pagination
  - Click to insert into composer
- Wire "Choose from library" button (already exists from v17 Phase 5 Y-12 fix) to open MediaLibraryModal

**Files:** ~2 new + ~1 modified
**Effort:** 1 iteration

---

### Phase 7 — Settings Page Restructure (Sidebar Tabs)

**Why:** Postiz's settings page is a 2-column layout with a left sidebar of tabs (Global, Teams, Webhooks, Auto Post, Sets, Signatures, Developers, Approved Apps). Social Forge v18's settings are flat pages with breadcrumbs. This phase restructures to the sidebar-tab pattern for better navigation.

**Deliverables:**
- `routes/settings/+layout.svelte` — new: 2-column layout with left sidebar of tabs
- Tabs (filtered for single-user — no Teams, no Approved Apps):
  - General (timezone, theme, date format) — existing
  - Brand Profile — existing
  - RSS Autopost — existing
  - Signatures — existing
  - Notifications — existing
  - Developer (API keys, MCP config) — existing
  - Webhooks — existing
  - MCP & CLI — existing
- Each tab renders its existing page content in the right pane
- Active tab highlighted; URL syncs to `/settings/{tab}`
- Mobile: sidebar collapses to a dropdown

**Files:** ~1 new + ~8 modified (convert each settings page to a tab content component)
**Effort:** 1 iteration

---

### Phase 8 — Mobile OAuth Route + Responsive Improvements

**Why:** Postiz has a `/provider/add` mobile-only route that redirects OAuth via `postiz://` deep link for native browser handoff. Social Forge v18's mobile sidebar (from v17 Phase 4 U-8) works but the composer modal is cramped on mobile. This phase adds the mobile OAuth route and improves composer responsiveness.

**Deliverables:**
- `routes/auth/mobile/+page.svelte` — new: mobile OAuth bridge page
  - Detects mobile via user-agent
  - Redirects to provider OAuth URL with `redirect_uri` pointing back to this page
  - On callback, posts to `window.opener` or redirects to app
- `lib/composer/ComposerModal.svelte` — responsive improvements:
  - On `< lg`: single column, preview pane collapses to a toggle button (show/hide)
  - On `< sm`: footer actions become a bottom sheet (Post Now + Schedule stack vertically)
  - Channel selector: 2-col grid on mobile (was 3-col)
- `lib/calendar/WeekView.svelte` — horizontal scroll on mobile (already has `overflow-x-auto` on the grid? verify)

**Files:** ~1 new + ~3 modified
**Effort:** 1 iteration

---

### Phase 9 — AI Generator (Bulk Post Generation) — FLAGSHIP

**Why:** Postiz's `GeneratorComponent` lets the user enter a topic + format + tone, then streams NDJSON progress stages (agent, research, find-category, generate-hook, generate-content, upload-pictures, post-time) and pipes the result into the composer. This is Postiz's flagship AI feature. Social Forge v18 has an `AiAssistant` (single-post generation) but no bulk generator.

**Deliverables:**
- **Backend:** `POST /api/posts/generate` — streaming endpoint (SSE or NDJSON):
  - Input: `{ topic, format: 'one_short'|'one_long'|'thread_short'|'thread_long', tone: 'personal'|'company', add_pictures: bool }`
  - Uses the existing AI provider (OpenAI/Anthropic via `src/ai/`)
  - Emits progress stages as SSE events
  - Returns `{ hook, content[], date, media[]? }`
- **Frontend:**
  - `lib/composer/GeneratorModal.svelte` — new:
    - Topic textarea
    - Format select (One short / One long / Thread short / Thread long)
    - Tone select (Personal / Company)
    - "Add pictures?" checkbox
    - Generate button → streams progress stages with shimmer animation
    - On completion, calls `composer.openCreate(undefined, [], content[0])` with the generated content
  - "Generate Posts" button in the calendar sidebar (next to "Create Post")
- **Security:** AI API key stays server-side (already the case from v13 Phase 3.1)

**Files:** ~1 backend + ~1 frontend
**Effort:** 1-2 iterations

---

### Phase 10 — Touch-Device DnD Fallback for Calendar

**Why:** Both Postiz and Social Forge v18 use HTML5 native drag-and-drop, which doesn't fire on most mobile browsers. This means drag-to-reschedule is desktop-only. This phase adds a pointer-events fallback so the calendar works on touch devices — a differentiator vs. Postiz.

**Deliverables:**
- `lib/calendar/CalendarEvent.svelte` — add `ontouchstart` handler:
  - On `touchstart`: capture the event ID + original touch position
  - On `touchmove`: show a floating ghost chip following the finger
  - On `touchend`: hit-test against day/hour cells using `document.elementFromPoint()`
  - If hit: call `onDrop(eventId, dateStr, hour)` (same as HTML5 DnD)
  - If miss: cancel (animate ghost back)
- `lib/calendar/WeekView.svelte` + `DayView.svelte` — no changes needed (the touch handler calls the same `onDrop` callback)
- Visual feedback: highlight the hovered cell during `touchmove` (same ring-2 ring-indigo-500 as HTML5 DnD)

**Files:** ~1 modified (CalendarEvent.svelte)
**Effort:** 1 iteration

---

## Part C — Sequencing & Dependencies

```
Phase 1 (Stats Modal) ────────────────── independent
Phase 2 (TimeTable) ───┬──> Phase 5 (Channel Menu) ── uses TimeTableModal
                       │
Phase 3 (Onboarding) ──┼────────────────────────────── independent
Phase 4 (List View) ───┼────────────────────────────── independent
Phase 6 (Media Lib) ───┼────────────────────────────── independent
Phase 7 (Settings) ────┼────────────────────────────── independent
Phase 8 (Mobile) ──────┼────────────────────────────── independent
Phase 9 (AI Generator) ┼────────────────────────────── independent (flagship)
Phase 10 (Touch DnD) ──┴────────────────────────────── independent
```

- **Phase 2 → Phase 5 dependency:** The channel ⋯ menu (Phase 5) opens the TimeTableModal (Phase 2). Do Phase 2 first.
- **All other phases are independent** — can be done in any order.
- **Phase 9 (AI Generator) is the flagship** — highest user-visible value but also highest effort.

**Recommended order:** 3 → 4 → 1 → 2 → 5 → 6 → 7 → 8 → 10 → 9

(Quick wins first, flagship last — builds momentum and each phase is independently shippable.)

---

## Part D — What We Explicitly Skip (Architectural Filters)

| Postiz feature | Skip reason |
|---|---|
| Customer/group organization (#12) | Violates single-user preference (AGENTS.md §0.5.1) — no multi-tenant, no customer grouping |
| CopilotKit AI assistant (#6) | YAGNI — Social Forge has its own AiAssistant component; CopilotKit is a heavy dep (1.10.6 + runtime endpoint) that doubles the AI surface |
| Merge/Separate post groups (#4) | Dormant in Postiz itself (zero references in their codebase) — not worth porting |
| Full Uppy adoption (#7) | YAGNI — 10+ Uppy packages + storage plugin selection is too heavy for a solo-founder tool; v19 Phase 6 improves MediaUpload incrementally instead |
| Teams tab in settings (#8 subset) | Violates single-user preference — no team collaboration |
| Approved Apps tab (#8 subset) | YAGNI — no OAuth app directory for a solo-founder tool |
| Polotno image editor (#7 subset) | YAGNI — too heavy for occasional image editing |
| AI image/video generation (#7 subset) | Deferred — depends on backend AI pipeline not yet built; can be added later as a Phase 9 extension |
| Per-platform fonts (Chirp, Charter, SFNS) | YAGNI — bundle size not worth the fidelity (already skipped in v18 Phase 4) |
| Full mobile-responsive calendar grid | Deferred — both Postiz and Social Forge have this gap; the 136px+7×1fr grid horizontally scrolls on mobile. A full mobile calendar rewrite is a future v20 effort |

---

## Part E — Success Metrics

After Phase 10, the following should be true:

1. **Per-post statistics** show line charts for likes/comments/shares/impressions over 7/30/90 days.
2. **Per-channel time presets** show as ghost slots in DayView.
3. **New users** see a 2-step onboarding modal (connect channels → quick tour) on first login.
4. **Calendar list view** has state filter (all/scheduled/draft/published) + server-side pagination.
5. **Each channel** in the sidebar has a ⋯ menu with quick actions (create post, edit times, disable, delete).
6. **Media upload** supports drag-to-reorder, alt text, clipboard paste, and a searchable library modal.
7. **Settings page** is a 2-column sidebar-tab layout (not flat pages with breadcrumbs).
8. **Mobile OAuth** works via a dedicated bridge route; composer is usable on mobile.
9. **AI Generator** generates bulk posts from a topic + format + tone prompt, streaming progress.
10. **Calendar drag-and-drop** works on touch devices (not just desktop).
11. **Zero regression** — all v18 functionality intact; `cargo check`, `cargo test`, `svelte-check`, `vite build` all pass.
12. **Architectural preferences intact** — still single-user, single-binary, triple-interface, security-first, realtime-first, YAGNI.

---

## Part F — Risk Register

| Risk | Mitigation |
|---|---|
| AI Generator (Phase 9) requires streaming SSE | Social Forge already has SSE infrastructure (`/api/events`); reuse the pattern for the generator stream |
| Touch DnD (Phase 10) may conflict with scroll | Use a long-press threshold (200ms) before starting drag; cancel if finger moves >10px before threshold |
| Settings restructure (Phase 7) breaks deep links | Keep all existing `/settings/{tab}` URLs working; the new layout is additive |
| TimeTable (Phase 2) migration on existing DB | `posting_times` column is nullable; existing integrations get NULL (no presets) — backward compatible |
| Media library (Phase 6) search needs backend support | `GET /api/media?q=` already exists from v17; verify it supports search |
| Statistics modal (Phase 1) needs engagement data | `post_engagement` table already populated by feed refresher (v17 Phase 2 B-3 pattern); data exists |

---

**End of plan. Implementation can begin with Phase 3 (Onboarding) — the quickest win.**
