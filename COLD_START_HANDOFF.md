# Cold-Start Handoff Document — Social Forge Frontend UI/UX Agent

> **Purpose**: This document gives a new AI agent everything it needs to pick up the work of auditing and upgrading the visual-layer frontend UI/UX of social-forge. It is written to be self-contained — the agent should not need to read any other file first (though pointers to deeper context are provided).
>
> **Last updated**: 2026-07-08 (after v24)
> **Current HEAD**: `d75a54e` on `origin/master`
> **Frontend status**: `pnpm build` succeeds, `svelte-check` 0 errors, 0 hardcoded hex colors, 0 native `confirm()`/`prompt()`, 0 TODO/FIXME.

---

## 1. What Social Forge Is

Social Forge is a **single-user, self-hosted social media management platform** for solo founders. Think of it as a personal digital-marketing-agency command center — one person managing many social accounts across 30+ platforms (X, Reddit, LinkedIn, Facebook, Instagram, YouTube, Threads, TikTok, Bluesky, Mastodon, Pinterest, Discord, Slack, Telegram, WhatsApp, WordPress, Medium, Dev.to, Hashnode, GitHub, etc.).

**The product vision**: A solo founder should be able to plan, create, schedule, publish, and analyze social media content across all their channels from a single dashboard — with AI assistance, campaign management, and a content calendar that rivals professional tools like Buffer, Hootsuite, or Postiz.

**The technical architecture** (non-negotiable — see AGENTS.md §0.5):
- Single Rust binary (axum HTTP server + rmcp MCP server + in-process scheduler + SSE broadcaster + background tasks).
- PostgreSQL is the only external dependency (runs in Docker).
- SvelteKit 5 (Svelte 5 runes) frontend embedded into the binary via `rust-embed`.
- SSE for realtime (not SWR polling).
- Token encryption at rest (AES-256-GCM).
- No Redis, no Temporal, no microservices, no multi-user auth.

---

## 2. The Frontend Stack

| Aspect | Technology | Version |
|--------|-----------|---------|
| Framework | SvelteKit 2 + Svelte 5 (runes) | svelte ^5.25.8 |
| Styling | Tailwind CSS 3 | ^3.4.17 |
| Rich text | TipTap 3 (StarterKit + Link + Placeholder) | ^3.23.4 |
| State | Svelte 5 rune class singletons ($state, $derived, $effect) | — |
| Realtime | SSE via EventSource (/api/events) | — |
| Build | Vite 6 + @sveltejs/adapter-static (SPA fallback) | vite ^6.3.1 |
| Type checking | svelte-check 4 | ^4.4.8 |
| Package manager | pnpm | — |

**No additional frontend dependencies should be added** unless absolutely necessary. The architectural preference is "no new third-party frontend libs." If a feature requires one, document why and get user approval first.

---

## 3. Design System State (as of v24)

### Color Token System

The design system uses **CSS variables + Tailwind semantic tokens**. Every color is theme-aware (dark/light). The system is defined in two files:

**`frontend/tailwind.config.js`** — defines the Tailwind color names:
- `brand` (50–900, indigo palette — the primary brand color)
- `surface`, `surface-hover` — card backgrounds
- `background`, `background-input` — page + input backgrounds
- `line`, `line-hover` — borders
- `muted`, `muted-dark` — secondary text
- `content`, `content-secondary` — primary text
- `success`, `warning`, `error`, `info` — semantic status colors (v22 Phase 3)
- `radius-sm`, `radius-md`, `radius-lg` — border radius scale

**`frontend/src/app.css`** — defines the CSS variable values per theme:
- `:root.dark` — dark theme (default): bg #0b0e14, card #131720, border #1e2435, text #e8edf5
- `:root.light` — light theme: bg #f8fafc, card #ffffff, border #e2e8f0, text #1e293b
- Status colors use `--success-rgb: 34 197 94` (space-separated triplet for Tailwind's `rgb(var(--x) / <alpha-value>)` syntax)

**Rules**:
1. **NEVER use hardcoded hex colors** in Tailwind classes. Use semantic tokens (`bg-surface`, `text-muted`, `border-line`, `text-error`, etc.).
2. **NEVER use `bg-indigo-600`** or similar Tailwind color names for UI elements. Use `bg-brand-500` instead.
3. **Provider brand colors** (X=#000000, Reddit=#FF4500, LinkedIn=#0A66C2, etc.) in `ProviderIcon.svelte` and `providers.ts` are **data values** (JS objects), not CSS classes — these are fine as hex strings.
4. **`<style>` blocks** in `.svelte` files must use `var(--*)` CSS variables, not hardcoded hex.
5. The grep command to verify: `grep -rn "bg-\[#\|text-\[#\|border-\[#" frontend/src/ --include="*.svelte"` should return 0 matches.

### UI Primitives (`frontend/src/lib/ui/`)

15 primitives exist:

| Primitive | Purpose | Notes |
|-----------|---------|-------|
| `Button.svelte` | Primary/secondary/ghost/danger buttons | Uses CSS vars in `<style>` — rethemes correctly |
| `Badge.svelte` | State badges (draft/queued/published/error/idea) + generic variants (success/warning/error/info) | Dual-mode: `state=` or `variant=` |
| `Card.svelte` | Surface card with optional header/footer snippets | padding prop: none/sm/md/lg |
| `EmptyState.svelte` | Icon + title + description + action button | Used by posts/feed/kanban/campaigns |
| `PageHeader.svelte` | Title + subtitle + actions slot | Standardizes route headers |
| `Tabs.svelte` | Segmented control with bindable value | Used by settings/campaign detail |
| `Skeleton.svelte` | Shimmer placeholder | width/height/rounded props |
| `Tooltip.svelte` | Hover/focus tooltip | position prop: top/bottom/left/right |
| `Avatar.svelte` | Image with fallback initial | xs/sm/md/lg sizes |
| `Pagination.svelte` | Windowed page numbers + prev/next | maxVisible prop |
| `StatCard.svelte` | Number + label + optional trend delta | color prop: default/success/warning/error/info |
| `Icon.svelte` | Inline-SVG icon registry (~30 icons) | Lucide-style |
| `Spinner.svelte` | Loading spinner | — |
| `Modal.svelte` | Basic modal (largely superseded by ModalManager) | — |
| `Dropdown.svelte` | Click-outside dropdown | — |

**Missing primitives** (not yet built — build as needed):
- `DataTable.svelte` — sortable/filterable/paginated table (posts list, webhooks, API keys all duplicate this)
- `FilterBar.svelte` — search + filter pills + sort dropdown
- `Switch.svelte` — toggle switch (settings pages use checkboxes)
- `Checkbox.svelte` — styled checkbox
- `Popover.svelte` — floating content (calendar mini-cal, date pickers)
- `CommandPalette.svelte` — exists in `components/` but could be moved to `ui/`

### Component Library (`frontend/src/lib/components/`)

5 higher-level components:
- `CommandPalette.svelte` — Cmd+K command launcher (21 commands, fuzzy search, recent commands)
- `ModalManager.svelte` — modal stack with z-index, escape, askClose
- `Toast.svelte` — toast notifications
- `ShortcutsModal.svelte` — keyboard shortcuts help
- `CommentsThread.svelte` — feed comment thread viewer

### Stores (`frontend/src/lib/stores/`)

8 Svelte 5 rune-class singletons:
- `calendar.svelte.ts` — view, currentDate, selectedDate, listState, listPage, nowTick (2min interval for past-cell recompute)
- `composer.svelte.ts` — open, mode (create/edit), presetDate, editingPostId, prefilledContent, prefilledMedia
- `keyboard.svelte.ts` — keyboard shortcut registration
- `modals.svelte.ts` — modal stack manager with `areYouSure()` promise helper
- `realtime.ts` — SSE client (EventSource), subscribes to 12 event types + `lagged`
- `theme.svelte.ts` — dark/light toggle, persisted to localStorage
- `timezone.svelte.ts` — user's selected timezone (used by calendar + dashboard)
- `toast.ts` — toast queue

### Providers (`frontend/src/lib/providers.ts`)

Central metadata for all 31 providers: label, color, icon (text), charLimit. Also includes:
- `providerMeta(provider)` — returns the metadata object
- `providerLabel(provider)` / `providerColor(provider)` / `providerIcon(provider)` / `providerCharLimit(provider)` — convenience accessors
- `platformPostUrl(provider, platformPostId)` — constructs "Manage on platform" URLs (v23-6)

---

## 4. Current Route Map (28 routes)

### Main routes
| Route | Purpose | Key state |
|-------|---------|-----------|
| `/` | Dashboard — stat cards, engagement, adherence, cadence, recent events, today's schedule, needs attention, quick actions | 8 API calls on load, realtime listeners |
| `/calendar` | Calendar — month/week/day/list views, drag-and-drop, filters (channel, campaign, tag), bulk actions | calendarState store, URL sync |
| `/kanban` | Kanban board — 4 columns (Ideas/Drafts/Scheduled/Published), drag-between-columns, campaign filter, quick-add | realtime listeners |
| `/posts` | Posts list — checkbox selection, filter tabs, search, sort, bulk reschedule/delete, pagination | — |
| `/posts/[id]` | Post detail — read-only view with engagement metrics | — |
| `/feed` | Imported feed — infinite scroll, media handling, view original, manage on platform, repurpose, hide, bookmark | — |
| `/campaigns` | Campaign list — grid of cards with progress bars, status filter, create modal | realtime listeners |
| `/campaigns/[id]` | Campaign detail — Overview/Posts/Settings tabs | — |
| `/channels` | Channel management — connect flows for 31 providers, page picker, time slots | polling for OAuth status |
| `/analytics` | Analytics — per-provider charts (cache-first) | — |
| `/search` | Search — searches feed posts | — |
| `/comments` | Comments — aggregated comment inbox | — |
| `/dms` | DMs — direct message conversations | — |
| `/automation` | Automation rules — CRUD for auto-reply rules | platform + channel selector |
| `/media` | Media library — grid view, upload, delete | — |
| `/tags` | Tags — CRUD for colored tags | — |

### Settings routes (single sidebar, 8 sub-routes)
| Route | Purpose |
|-------|---------|
| `/settings` | General — user info, auth method |
| `/settings/profile` | Brand Profile — brand name, tone, audience, content pillars, keywords, hashtag sets, avoid topics, posting frequency (synced to backend) |
| `/settings/rss` | RSS Autopost — CRUD for RSS feeds with AI summary toggle |
| `/settings/signatures` | Signatures — CRUD with per-provider defaults |
| `/settings/notifications` | Notifications — push prefs, quiet hours, timezone |
| `/settings/developer` | Developer — API key management |
| `/settings/webhooks` | Webhooks — CRUD + delivery history |
| `/settings/mcp` | MCP & CLI — static config display |

---

## 5. What's Been Done (v22 → v24 Summary)

### v22 (7-phase refactor — 8 commits)
- **Phase 1**: 5 CRITICAL backend bugs fixed (SSE auth bypass, kanban state-transition validation, kanban realtime broadcast, manual publish idempotency, and_hms_opt panic) + 3 quick wins (stuck-publish sweep, automation NIL UUID, SSE lagged event signal).
- **Phase 2**: 3 HIGH backend bugs fixed (scheduler timeout abort, permit inside spawn, circuit breaker CAS) + posting-infra upgrade (X Idempotency-Key, publish_outbox schema, workflow versioning schema).
- **Phase 3**: 9 new UI primitives built, 113 hardcoded hex colors migrated to semantic tokens, Button.svelte + CalendarEvent.svelte `<style>` blocks fixed, Badge.svelte upgraded.
- **Phase 4**: Sidebar dedup (8 settings → 1), collapse-to-icon-rail, Cmd+K command palette, active-link `startsWith` fix.
- **Phase 5**: Dashboard timezone fix, providerIcon/providerColor bug fix, StatCard primitive, semantic token migration.
- **Phase 6**: Campaign model expansion (status, progress_metric, audience_persona, content_pillars, budget, kpi_targets, soft-delete), kanban quick-add fix (require channel), campaign filter fix, campaign edit form wired.
- **Phase 7**: Monday/Sunday calendar consistency, alt-text fix, timezone fixes in SchedulePicker + bulk-reschedule + drag-drop, tag-color chip.

### v23 (9 deferred items)
- **v23-1**: 4 new analytics endpoints (engagement, adherence, cadence, recent-events) + events_log table + `send_and_log` broadcaster.
- **v23-2**: Transactional outbox drain loop (runs every 30s, retries failed DB writes after successful platform publish).
- **v23-3**: Campaign detail pages (`/campaigns` list + `/campaigns/[id]` with Overview/Posts/Settings tabs).
- **v23-4**: Calendar filters (integration_ids, campaign_id) in backend + frontend.
- **v23-5**: `postsApi.update` accepts `tag_ids` + `first_comment` (was silently dropped on edit).
- **v23-6**: Feed "Manage on platform" affordance (`platformPostUrl()` helper + link in feed).
- **v23-7**: LinkedIn/Reddit/Slack Idempotency-Key headers.
- **v23-8**: Per-cell `useInterval` greyscale flip (2min nowTick in calendar state).
- **v23-9**: CSRF multi-origin support (comma-separated `FRONTEND_URL`).

### v24 (8 final polish items)
- **v24-1**: `POST /api/posts/{id}/unschedule` endpoint (ComposerModal saveAsDraft properly transitions to draft).
- **v24-2**: Calendar filter bar UI (channel + campaign dropdowns in CalendarHeader).
- **v24-3**: Dashboard 3 new widgets (Scheduled vs Actual, Posting Cadence, Recent Events).
- **v24-4**: Brand profile sync to backend (migration 036, `/api/profile` endpoints, AiAssistant reads brand context).
- **v24-5**: Generic thread builder (`delay_minutes` in PostContent, scheduler sleeps between thread parts).
- **v24-6**: Underline toolbar button + RichTextEditor CSS variable migration.
- **v24-7**: X weightedLength char counting (emoji/CJK = 2, ASCII = 1).
- **v24-8**: 4 new platform previews (X, Reddit, Threads, Bluesky).

### Total impact
- 28 commits, 155 files changed, 4381 insertions, 538 deletions.
- 0 hardcoded hex colors. 0 native `confirm()`/`prompt()`. 0 TODO/FIXME. 0 svelte-check errors.
- 6 new migrations (031–036), all additive, all auto-applied on startup.
- 4 new backend endpoints (engagement, adherence, cadence, recent-events, profile, unschedule).
- 4 new frontend routes (`/campaigns`, `/campaigns/[id]`).

---

## 6. What Remains (Frontend Visual/UX Gaps)

These are the items the new agent should evaluate and implement. They are prioritized by impact.

### Tier 1 — High-impact UX gaps

1. **Kanban re-architecture** — The kanban is still a thin state-grouped post list. Missing: swimlanes (by campaign/channel), WIP limits, card cover images, drag-to-reorder within column, sub-states (ready/in_review/blocked) UI, due dates on cards, priority indicators on cards, card tags visible, card activity log. The backend schema exists (migration 034 — `kanban_sort_order`, `kanban_substate`, `due_date`, `priority`) but the frontend doesn't use these fields yet.

2. **Composer Mantine-style date picker popover** — The SchedulePicker still uses native `<input type="date">` + `<input type="time">`. Postiz uses a Calendar popover with locale-aware format. A custom Svelte popover (no Mantine dep) should be built.

3. **TipTap Mention extension** — `@` mentions with a suggestion popup wired to `/api/integrations/{id}/mentions`. The backend endpoint exists. Requires either installing `@tiptap/extension-mention` or building a lightweight custom mention trigger.

4. **Frontend thread builder UI** — The backend supports per-row `delay_minutes` (v24-5) but the composer doesn't have a UI for it. The existing `ThreadFinisher.svelte` is X-only and just splits content. A generic `ThreadBuilder.svelte` with per-row delay input + per-row content editing is needed.

5. **Dashboard sparkline charts** — The engagement and cadence widgets return per-day data (`by_day` arrays) but the dashboard doesn't render sparkline charts. A lightweight inline SVG sparkline (no Chart.js dep) would make the widgets feel premium.

### Tier 2 — Polish

6. **Mini-calendar in sidebar** — A small month calendar for jumping to dates. Click a date → `calendarState.currentDate` updates. Highlight days with posts.

7. **Campaign analytics endpoint + UI** — The campaign detail page's Analytics tab is empty. A `GET /api/campaigns/{id}/analytics` endpoint returning per-campaign engagement totals, post counts by state, and progress towards goal is needed.

8. **Onboarding flow polish** — The `GettingStarted.svelte` widget exists but isn't tied to a checklist API. The onboarding modal is shown once and dismissed.

9. **Notification panel** — `NotificationBell.svelte` exists but the dropdown is minimal. No mark-all-read, no filter by type, no settings deep-link.

10. **Global search in sidebar** — The search input in the sidebar (when expanded) was planned but not built. `/search` exists as a route but isn't surfaced in the sidebar.

### Tier 3 — Nice-to-have

11. **Full kanban card preview on hover** — hover a card → popover shows full content + media + scheduled_at + tags.
12. **Keyboard shortcuts** — `g c` for goto-calendar, `g p` for goto-posts, etc. The `keyboard.svelte.ts` store exists but only `Cmd+Enter`/`Cmd+S` in composer and `?` for shortcuts modal are wired.
13. **Audit log UI** — `post_activity` table (migration 033) records every state change but there's no UI for it.
14. **Channel mix analysis** — pie chart of posts by channel for the last 30 days.
15. **Best posting times** — based on historical engagement data.

---

## 7. How to Work on This Codebase

### Golden Rules (from AGENTS.md)

1. **ALWAYS PUSH COMMITS AFTER EACH ITERATION.** Commit + push after each meaningful unit of work. Don't batch 5 phases into one commit.
2. **NEVER break the build.** Before committing: `pnpm build` must succeed, `svelte-check --threshold error` must be 0 errors.
3. **NEVER use hardcoded hex colors.** Use semantic tokens.
4. **NEVER use native `confirm()`/`prompt()`.** Use `modals.areYouSure()` or inline modals.
5. **NEVER add new npm dependencies** without user approval.
6. **ALWAYS use Svelte 5 runes** (`$state`, `$derived`, `$effect`, `$props`, `$bindable`). NOT Svelte 4 syntax.
7. **ALWAYS read the worklog** at `/home/z/my-project/worklog.md` before starting.
8. **ALWAYS append to the worklog** after finishing (don't overwrite).

### Build & Verify Commands

```bash
# Install deps (first time only)
cd frontend && pnpm install

# Build (must succeed before commit)
pnpm build

# Type check (must be 0 errors)
pnpm exec svelte-check --threshold error

# Dev server (port 3000)
pnpm dev

# Verify no hardcoded hex
grep -rn "bg-\[#\|text-\[#\|border-\[#\|ring-\[#\|placeholder-\[#" frontend/src/ --include="*.svelte" | wc -l
# Should be 0

# Verify no native confirm/prompt
grep -rn "confirm(\|prompt(" frontend/src/ --include="*.svelte" | grep -v "<!--" | grep -v "//" | wc -l
# Should be 0
```

### Git Protocol

```bash
# After each iteration:
git add <specific files>
git commit -m "v25 <phase>: <summary>"
git push origin master
```

### File Organization

- **Routes**: `frontend/src/routes/` — SvelteKit file-based routing. Each `+page.svelte` is a page, `+layout.svelte` is a layout.
- **Reusable components**: `frontend/src/lib/ui/` for primitives, `frontend/src/lib/components/` for higher-level, `frontend/src/lib/calendar/` for calendar, `frontend/src/lib/composer/` for composer.
- **API clients**: `frontend/src/lib/api/` — one `.ts` file per backend module (posts.ts, feed.ts, campaigns.ts, analytics.ts, etc.).
- **Stores**: `frontend/src/lib/stores/` — Svelte 5 rune class singletons.
- **Providers**: `frontend/src/lib/providers.ts` — central provider metadata.

### Styling Conventions

- Use Tailwind classes with semantic tokens: `bg-surface`, `text-muted`, `border-line`, `text-error`, `bg-brand-500`, etc.
- For `<style>` blocks, use `var(--*)` CSS variables: `var(--bg-card)`, `var(--text-muted)`, `var(--brand)`, `rgb(var(--error-rgb) / 0.15)`.
- For provider brand colors in JS data (not CSS), hex strings are fine: `{ x: '#000000', reddit: '#FF4500' }`.
- Border radius: use `rounded-sm`, `rounded-md`, `rounded-lg` (tokens), not `rounded-[8px]`.
- Opacity: use Tailwind's `/` syntax: `bg-success/20`, `text-error/80`, `border-warning/30`.

---

## 8. Reference Files (Read These First)

| File | Purpose | Lines |
|------|---------|-------|
| `AGENTS.md` | Development protocol, golden rules, architecture, security | ~635 |
| `SOCIAL_FORGE_AUDIT_AND_PLAN_v22.md` | Full audit + 7-phase refactor plan | ~1825 |
| `UPGRADE_GUIDE_v24.md` | Migration guide + deployment verification checklist | ~450 |
| `frontend/tailwind.config.js` | Color token system definition | ~50 |
| `frontend/src/app.css` | CSS variable definitions (dark/light themes) | ~200 |
| `frontend/src/routes/+layout.svelte` | Main app shell (sidebar, command palette, theme toggle) | ~320 |
| `frontend/src/routes/+page.svelte` | Dashboard (12 widgets, 8 API calls, realtime) | ~490 |
| `frontend/src/lib/composer/ComposerModal.svelte` | The post composer (edit mode, media, scheduling, AI) | ~1000 |
| `frontend/src/routes/calendar/+page.svelte` | Calendar page (4 views, drag-drop, filters, bulk actions) | ~530 |
| `frontend/src/routes/kanban/+page.svelte` | Kanban board (4 columns, drag, quick-add, campaign filter) | ~400 |
| `frontend/src/routes/campaigns/[id]/+page.svelte` | Campaign detail (Overview/Posts/Settings tabs) | ~330 |
| `frontend/src/lib/providers.ts` | Provider metadata + platformPostUrl helper | ~160 |

---

## 9. The Product Vision (What "World-Class" Means)

The user wants social-forge to function as a **fully functional platform that can perform the functions of high-octave digital marketing agencies for solo founders**. Concretely, this means:

### Content Planning
- **Calendar** that rivals Postiz/Buffer: week-default view, drag-to-reschedule, past-cell greying, tag-colored chips, per-post error indicators, "+N more" expander, list view with state filter, URL sync, mini-calendar. ✅ (mostly done)
- **Kanban** that rivals Linear/Trello: swimlanes, WIP limits, card cover images, drag-to-reorder, sub-states, due dates, priority, card preview on hover, card activity log. ⚠️ (backend ready, frontend needs work)
- **Campaigns** that rival a marketing-strategic-dashboard: goal progress, audience persona tracking, content pillar tracking, funnel visualization, budget tracking, posting cadence vs goal. ⚠️ (list + detail pages exist, analytics tab empty)

### Content Creation
- **Composer** that rivals Postiz: full-screen modal, two-column (editor + preview), per-channel override, rich text (bold/italic/underline/mention/link/image/emoji), thread builder with per-row delay, first comment, media upload with real progress + alt text, AI assistant with brand context, per-platform char count (X weighted), per-platform live preview, Mantine-style date picker, auto-schedule, repeat, askClose confirmation, pre-flight validation. ⚠️ (most done, missing: mention, date picker popover, thread builder UI)

### Content Publishing
- **Scheduler** that is reliable: atomic claim, circuit breaker, retry with backoff, idempotency keys, transactional outbox, stuck-publish recovery, per-provider concurrency. ✅ (done in v22-v24)
- **Feed** that surfaces imported posts: infinite scroll, media handling, view original, manage on platform, repurpose, hide, bookmark, comments. ✅ (done)

### Analytics
- **Dashboard** that is a command center: stat cards with trend deltas, engagement with sparklines, channel performance with brand colors, adherence rate, posting cadence with streak, recent events, today's schedule (timezone-aware), needs-attention inbox, quick actions. ⚠️ (most done, missing: sparkline charts, audience growth, engagement-rate per channel)

### UX Quality
- **Navigation** that is fast: collapsible sidebar, Cmd+K command palette, global search, active-link highlighting, keyboard shortcuts. ✅ (mostly done, missing: global search in sidebar, keyboard shortcuts beyond Cmd+K)
- **Theming** that is cohesive: dark/light mode, semantic tokens, 0 hardcoded hex, consistent spacing/typography/radius. ✅ (done)
- **Realtime** that is instant: SSE for post state changes, kanban stage changes, campaign changes, SSE lagged-event signal. ✅ (done)

---

## 10. Agent Instructions

1. **Start by reading** `AGENTS.md` (the golden rules), this document (the context), and the worklog (what's been done).
2. **Run `pnpm build` + `svelte-check`** to verify the current state is clean.
3. **Pick a Tier 1 item** from §6 and implement it. Commit + push after each iteration.
4. **Use the design system** — don't invent new colors, spacing, or patterns. Use the existing primitives (`Card`, `EmptyState`, `PageHeader`, `Tabs`, `StatCard`, `Badge`, `Button`, `Skeleton`, `Tooltip`, `Avatar`, `Pagination`).
5. **Test your work** — `pnpm build` + `svelte-check` must pass before every commit.
6. **Append to the worklog** at `/home/z/my-project/worklog.md` after each iteration.
7. **Respect the architectural preferences** — no new deps, no native dialogs, no hardcoded hex, Svelte 5 runes, semantic tokens.
8. **When in doubt, ask the user** — don't guess on product direction. The user has strong opinions about UX.

---

*End of handoff document. The next agent should be able to start work immediately after reading this + AGENTS.md.*
