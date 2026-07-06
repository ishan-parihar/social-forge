# Social Forge v18 — Frontend UX Refactor Plan

**Contrast audit:** Social Forge (SvelteKit + Rust/Axum) vs Postiz-app (NextJS + NestJS)
**Goal:** Re-architect the calendar and posting flows to match Postiz's modal-based UX while preserving Social Forge's architectural preferences (single-user, single-binary, triple-interface parity, security-first, realtime-first, YAGNI).
**Date:** 2026-07-06
**Status:** Plan ready for implementation

---

## Executive Summary

Social Forge's frontend is well-structured but uses a **page-based navigation model** for the two most frequent workflows — creating and editing posts. Every "new post" or "edit post" action triggers a full route change, tears down the calendar's loaded state, and forces a network refetch on return. Postiz-app has solved this with a **modal-based composer** that opens in-place, stacks with other modals, and preserves the calendar context underneath.

This plan ports Postiz's three foundational UX patterns to Social Forge:

1. **Global modal store with stacking** — one `<ModalManager>` at the layout level, opened from anywhere, stackable, with `askClose` confirmation.
2. **Single `<ComposerModal>` for both create and edit** — the calendar passes a preset date or an existing post group; no route changes; the composer IS the editor.
3. **Week-view as the calendar default**, persisted in localStorage, with hour-precision drag-and-drop and a safety modal for already-published posts.

Plus four additive upgrades: per-platform live preview pane, global+per-channel content override model, drag-to-create, and posts-list inline quick-edit.

**What we explicitly do NOT adopt from Postiz** (per AGENTS.md §0.5):
- ❌ Team collaboration, roles, impersonation, customer groups (violates single-user)
- ❌ Temporal.io, Redis, microservices (violates single-binary)
- ❌ SWR polling (violates realtime-first — we use SSE)
- ❌ CopilotKit / Polotno / Web3 / browser extension (YAGNI for solo-founder)
- ❌ Per-platform fonts (Chirp, Charter, SFNS) — nice-to-have, not worth the bundle size
- ❌ Mantine (we already have Tailwind + a small UI primitive set; adding Mantine doubles the CSS surface)

---

## Part A — Contrast Audit Findings

### A.1 Navigation Model

| Aspect | Social Forge (current) | Postiz | Verdict |
|---|---|---|---|
| New post flow | `goto('/posts/new')` — full page nav | `modal.openModal({children: <AddEditModal date={…}/>})` — modal | **Port Postiz** |
| Edit post flow | `goto('/posts/[id]')` — limited inline edit page | Same modal as create, hydrated from existing data | **Port Postiz** |
| Calendar → composer | Page nav with `?date=` query param | Modal with date prop | **Port Postiz** |
| Posts list → edit | Page nav to `/posts/[id]` | Modal (no list page in Postiz — calendar is the list) | **Hybrid: keep list, use modal for edit** |
| Modal stacking | Not supported (each overlay is hand-rolled, no stack) | Stackable with zIndex 200+index, Escape closes topmost | **Port Postiz** |
| "Are you sure" on close | None — backdrop/escape dismisses instantly | `askClose: true` → confirm dialog | **Port Postiz** |

### A.2 Calendar

| Aspect | Social Forge | Postiz | Verdict |
|---|---|---|---|
| Default view | `month` (hardcoded, not persisted) | `week` (persisted in cookie, overridable via `?display=`) | **Port Postiz** |
| Views | month, week, day, list | week, day, month, list | **Match (just change default)** |
| Drag precision | Date-only (hour silently preserved) | Hour-precision in week/day, day-precision in month | **Port Postiz** |
| Drag visual feedback | None (no dragover highlight) | `opacity: 0` on source, drop target highlights | **Port Postiz** |
| Touch fallback | None (HTML5 DnD doesn't fire on mobile) | None either (Postiz also HTML5-only) | **Add pointer-events fallback** |
| Published-post drop | Blocked with toast | Safety modal: "Just update" vs "Reschedule" | **Port Postiz** |
| Drag-to-create | No (click navigates away) | No (Postiz also click-to-create) | **Add drag-to-create as enhancement** |
| Undo after drop | None | None | **Add 5-second undo toast** |
| Empty-slot click | `goto('/posts/new?date=…')` | Opens composer modal with date prop | **Port Postiz** |
| Event click | Opens right-side `PostDetail` panel → Edit navigates to page | Opens composer modal in edit mode | **Port Postiz (skip side panel, go straight to modal)** |

### A.3 Composer

| Aspect | Social Forge | Postiz | Verdict |
|---|---|---|---|
| Container | Full page (`/posts/new`, 683 lines) | Full-screen modal (`max-w-[1400px]`, 703 lines) | **Port Postiz** |
| Layout | Single column, vertically stacked | Two-column: editor left, live preview right (580px fixed) | **Port Postiz** |
| Multi-channel model | `providerOverride: Map<integrationId, html>` — single content field per channel | `global: Values[]` + `internal: Internal[]` + `current` — shared thread + per-channel overrides that clone-from-global on first divergence | **Port Postiz's model (cleaner)** |
| Channel tab strip | Segmented control: "🌐 Global" + one button per integration | `SelectCurrent` pill strip with pink dot for diverged channels + tiny X to remove | **Port Postiz (better signal)** |
| Per-platform preview | Plain-text snippet per channel (27 lines) | Per-platform chrome (IG carousel, X tweet, LinkedIn article, etc.) with character-crop highlighting | **Port Postiz** |
| Per-platform settings | None (settings are global) | `withProvider` HOC + portal'd settings forms, CSS-switched | **Port Postiz (for platforms that need it)** |
| Scheduling | Inline checkbox + date/time inputs + auto-schedule button | Footer date-popover (Mantine Calendar + TimeInput) | **Port Postiz's footer popover** |
| Primary actions | Sticky header: Post Now (green) + Schedule (indigo) | Footer: Save Draft + Schedule (hover → Post Now) | **Port Postiz's footer** |
| Cancel/back | `goto('/calendar')` — always calendar, never back | `askClose` confirm → closes modal, stays on calendar | **Port Postiz** |
| Draft auto-save | localStorage, 1500ms debounce | Zustand store (in-memory) + explicit Save Draft | **Keep our localStorage auto-save (better)** |
| Keyboard shortcuts | Cmd+Enter (post now), Cmd+S (save draft) | Escape (close topmost modal) | **Keep ours + add Escape confirm** |
| AI assistant | Side panel (toggle) | Floating CopilotPopup | **Keep side panel (no CopilotKit dep)** |
| Media upload | Drag-drop + library button | Uppy Dashboard | **Keep ours (Uppy is heavy)** |
| Thread finisher | Bottom of page | Inline in editor | **Keep ours** |
| First comment | Bottom of page | Per-platform setting | **Keep ours** |

### A.4 State Management

| Aspect | Social Forge | Postiz | Verdict |
|---|---|---|---|
| Calendar state | `calendarState` store (view + currentDate + selectedDate) — not persisted | React Context + SWR, persisted in cookie | **Port: persist view + currentDate to localStorage** |
| Composer state | Local component state + `providerOverride` Map | Zustand `useLaunchStore` (global + internal + current) | **Port: upgrade our Map to the global/internal/current shape** |
| Modal state | None (each overlay is local `$state`) | Zustand `useModalStore` (stack with zIndex) | **Port: build Svelte modal store** |
| Server state | SSE realtime + onMount fetch | SWR with refreshInterval | **Keep SSE (architectural preference)** |

### A.5 Modal Infrastructure

| Aspect | Social Forge | Postiz | Verdict |
|---|---|---|---|
| Reusable shell | `Modal.svelte` (53 lines) — used by 8 files | Custom `ModalManager` (429 lines) — used everywhere | **Port the manager, keep our shell** |
| Hand-rolled overlays | 6+ (PostDetail, PostStatsModal, OnboardingModal, post-detail tag picker, keyboard cheat-sheet, mobile sidebar) | None (everything goes through ModalManager) | **Migrate all 6 to ModalManager** |
| Stackable | No | Yes (zIndex 200+index) | **Port** |
| Escape behavior | Each overlay handles its own | Topmost-only, with `askClose` confirm | **Port** |
| Promise-based confirm | `confirm()` browser dialog | `areYouSure()` returns `Promise<boolean>` | **Port (better UX than browser confirm)** |

---

## Part B — Implementation Plan (8 Phases)

### Phase 0 — Foundation: Global Modal Store + ModalManager (BLOCKING)

**Why first:** Every subsequent phase depends on being able to open stacked modals from anywhere. This is the keystone.

**Deliverables:**
- `frontend/src/lib/stores/modals.svelte.ts` — Svelte 5 runes store with:
  - `stack: ModalEntry[]` where `ModalEntry = { id, component, props, options }`
  - `open(component, props, options)` → returns modal id
  - `close(id)`, `closeAll()`, `closeCurrent()`
  - `areYouSure({ title, message, confirmLabel, cancelLabel })` → `Promise<boolean>` (opens a confirm modal on top of the stack)
  - Options: `{ title, closeOnClickOutside, closeOnEscape, withCloseButton, askClose, size, fullScreen }`
- `frontend/src/lib/components/ModalManager.svelte` — renders the stack at the layout level:
  - Each modal gets `zIndex: 200 + index`
  - Backdrop click → `askClose` confirm if set, else close
  - Escape → close topmost (with `askClose` confirm if set)
  - Body scroll lock when stack non-empty
  - Only topmost modal is interactive (lower modals get `pointer-events-none`)
- Migrate `Modal.svelte` to read from the store (backwards-compatible: `<Modal open={…}>` still works for local-only modals)
- Migrate the 6 hand-rolled overlays:
  - `calendar/PostDetail.svelte` → registered as modal `post-detail`
  - `calendar/PostStatsModal.svelte` → registered as modal `post-stats`
  - `onboarding/OnboardingModal.svelte` → registered as modal `onboarding`
  - `routes/posts/[id]/+page.svelte` tag picker → registered as modal `tag-picker`
  - `+layout.svelte` keyboard cheat-sheet → registered as modal `shortcuts`
  - `+layout.svelte` mobile sidebar → registered as modal `mobile-sidebar`
- Wire `<ModalManager />` into `+layout.svelte` (authenticated branch only)

**Files touched:** ~10 new + ~8 modified
**Estimated effort:** 1 iteration
**Architectural check:** ✅ Single-user (no per-user modal perms), ✅ Single-binary (pure frontend), ✅ Triple-interface (modals are UI-only; CLI/MCP unaffected), ✅ Security (modals are client-side, no new attack surface), ✅ Realtime (modals can subscribe to SSE like any component), ✅ YAGNI (we need this for the composer modal)

---

### Phase 1 — Calendar: Week Default + Hour-Precision DnD + Safety Modal

**Deliverables:**
- `calendar.svelte.ts`: default `view: "week"`, persist `view` + `currentDate` to localStorage (`social-forge-calendar-view`, `social-forge-calendar-date`)
- `WeekView.svelte` + `DayView.svelte`: include target hour in drop payload (`${dateStr}-${hour}`), parse in `handleDrop`
- All views: add `ondragenter`/`ondragleave` highlight class (`ring-2 ring-indigo-500`) on drop targets
- `calendar/+page.svelte` `handleDrop`: after reschedule, show a 5-second undo toast (`toast.success("Rescheduled", { action: "Undo", onAction: () => revertReschedule(id, oldDate) })`)
- `calendar/+page.svelte` `handleDrop`: if `post.state === 'published'`, instead of blocking with a toast, open a modal:
  - Title: "This post is already published"
  - Body: "What do you want to do?"
  - Buttons: "Just update the post details" | "Reschedule the post" | "Cancel"
  - "Just update" → `PUT /api/posts/{id}/date` with `action: 'update'`
  - "Reschedule" → `PUT /api/posts/{id}/date` with `action: 'reschedule'` (re-publishes)
  - "Cancel" → revert optimistic update
- Add pointer-events fallback for touch devices: on `touchstart` of an event chip, capture the touch; on `touchmove`, show a floating ghost; on `touchend`, hit-test against day cells. (This is a smaller scope than full pointer-events DnD — just enough to make reschedule work on mobile.)
- `CalendarHeader.svelte`: add a "Today" button that jumps to current week (already exists, just verify it works in week view)

**Backend changes:** `PUT /api/posts/{id}/date` needs to accept an `action` field (`'update' | 'reschedule'`). Currently it only reschedules. Add the `action` param and branch:
- `action: 'update'` → only change `scheduled_at`, don't re-publish
- `action: 'reschedule'` → change `scheduled_at` AND trigger a re-publish (or mark for re-publish)

**Files touched:** ~6 frontend + ~2 backend
**Estimated effort:** 1 iteration
**Architectural check:** ✅ all 6 preferences preserved

---

### Phase 2 — Composer Modal (THE BIG ONE)

**Deliverables:**
- `frontend/src/lib/stores/composer.svelte.ts` — composer store:
  ```ts
  {
    open: boolean,
    mode: 'create' | 'edit',
    presetDate: string | null,      // ISO date for create mode
    editingPostId: string | null,   // for edit mode
    presetIntegrationIds: string[], // for "create post for this channel"
  }
  ```
  - `openCreate(presetDate?, presetIntegrationIds?)`
  - `openEdit(postId)`
  - `close()` (with askClose confirm if dirty)
- `frontend/src/lib/composer/ComposerModal.svelte` — the modal:
  - Registered with ModalManager as `composer`
  - Two-column layout: editor left (flex-1), preview right (580px fixed, hidden on mobile)
  - Header: "Create Post" | "Edit Post" badge + close button
  - Left column: ChannelSelector → SelectCurrent tab strip → RichTextEditor → collapsible per-channel settings
  - Right column: `<PlatformPreviewPane>` (Phase 4)
  - Footer: tags, repeat, delete (edit mode only), date-popover, Save Draft, Schedule (hover → Post Now)
  - `askClose: true` — confirm before closing if content is non-empty
- `routes/posts/new/+page.svelte` — become a thin redirect: `onMount(() => composer.openCreate(url.searchParams.get('date')))` then `goto('/calendar')` (or wherever they came from). OR keep the page as a fallback for no-JS / direct-link access, but the primary flow is the modal.
- `routes/posts/[id]/+page.svelte` — become a read-only detail view with an "Edit" button that calls `composer.openEdit(id)`. Remove the inline content editor (the modal is the editor now).
- Wire all entry points to call `composer.openCreate()` or `composer.openEdit()` instead of `goto()`:
  - `+layout.svelte` keyboard `n` → `composer.openCreate()`
  - `routes/+page.svelte` dashboard buttons → `composer.openCreate()`
  - `routes/calendar/+page.svelte` empty-slot click → `composer.openCreate(date)`
  - `routes/calendar/+page.svelte` event click → `composer.openEdit(post.id)` (skip the side panel, go straight to modal — OR keep the side panel as a quick-view and add an "Edit" button that opens the modal)
  - `routes/posts/+page.svelte` row click → `composer.openEdit(post.id)`
  - `routes/posts/+page.svelte` duplicate button → `composer.openCreate()` with prefilled content (fetch the original, pass as `onlyValues`)
- Render `<ComposerModal />` at the `+layout.svelte` level (authenticated branch), reading from the composer store

**Files touched:** ~3 new + ~8 modified
**Estimated effort:** 2 iterations (modal shell + wire-up + edge cases)
**Architectural check:** ✅ all 6 preferences preserved. The composer modal is client-side only; CLI/MCP continue to use the REST API directly. SSE refreshes the calendar when `post_created` fires.

---

### Phase 3 — Multi-Channel Content Model Upgrade

**Why:** Our current `providerOverride: Map<integrationId, html>` is a single-content-string-per-channel model. Postiz's `global: Values[]` + `internal: Internal[]` + `current` model is strictly more powerful: it supports threads, per-channel media divergence, and a cleaner "clone on first divergence" UX.

**Deliverables:**
- `frontend/src/lib/stores/composer.svelte.ts` (extended) — add the content model:
  ```ts
  type Values = { id: string, content: string, delay: number, media: MediaItem[] }
  type Internal = { integrationId: string, values: Values[] }
  
  {
    // ... Phase 2 fields ...
    global: Values[],              // shared thread content
    internal: Internal[],          // per-channel overrides
    current: 'global' | string,    // current tab (integrationId or 'global')
  }
  ```
  - `addGlobalValue()` — append a new thread item
  - `setGlobalValueText(id, text)`
  - `setGlobalValueMedia(id, media[])`
  - `setGlobalDelay(id, delayMinutes)`
  - `removeGlobalValue(id)`
  - `reorderGlobal(fromId, toId)`
  - `addRemoveInternal(integrationId)` — toggle per-channel override; on first switch, clone `global` into `internal[integrationId]`
  - `setCurrent(tab)` — switch editor to 'global' or a specific integration
  - `reset()` — clear all on modal close
- `frontend/src/lib/composer/SelectCurrent.svelte` — pill tab strip:
  - "🌐 Global" pill (always present)
  - One pill per selected integration: provider icon + name + pink dot if `internal[integrationId]` exists + tiny X to remove (with confirm)
  - Click → `composer.setCurrent(tab)`
- `frontend/src/lib/composer/RichTextEditor.svelte` — bind to `current === 'global' ? global[0].content : internal[current][0].content`
- `frontend/src/lib/composer/ThreadFinisher.svelte` — upgrade to operate on `global[]` array (add/remove/reorder thread items, set per-item delay)
- `frontend/src/lib/composer/PerPlatformCharCount.svelte` — read from `current`'s values, not just `content`
- On submit, build the payload:
  ```ts
  {
    integration_ids: selectedIntegrations,
    content: global[0].content,        // primary content
    title, tag_ids, first_comment,
    media: global[0].media,
    overrides: Object.fromEntries(
      internal.map(i => [i.integrationId, { content: i.values[0].content, media: i.values[0].media }])
    ),
    thread: global.length > 1 ? global.map(v => ({ content: v.content, delay_minutes: v.delay })) : undefined,
    settings: { ...perChannelSettings },
  }
  ```
- Backend `POST /api/posts` already accepts `overrides` and `thread` — verify the shapes match, adjust if needed

**Files touched:** ~5 frontend + ~1 backend (verify shapes)
**Estimated effort:** 1 iteration
**Architectural check:** ✅ all 6 preferences preserved

---

### Phase 4 — Per-Platform Preview Pane

**Deliverables:**
- `frontend/src/lib/composer/PlatformPreviewPane.svelte` — right-column container:
  - Reads `current` from composer store
  - When `current === 'global'`: render `<GeneralPreview>`
  - When `current === integrationId`: render the platform-specific preview for that integration's provider
  - All previews are mounted but CSS-hidden except the current one (instant switching, no remount)
- `frontend/src/lib/composer/previews/GeneralPreview.svelte` — Twitter-like neutral card:
  - Avatar + name + verified badge + @handle
  - Content with mention highlighting (`@user` → purple) and hashtag highlighting (`#tag` → blue)
  - Character-crop: text beyond `providerCharLimit(provider)` is wrapped in `<mark class="bg-red-500/30 text-red-400">` with title "This text will be cropped"
  - Media grid: 1 image full-width, 2-3 side by side, 4+ in 2-col grid
  - Thread line: if `global.length > 1`, show thread items stacked with delay indicator
- `frontend/src/lib/composer/previews/XPreview.svelte` — X/Twitter chrome:
  - Reuse GeneralPreview styling (it's already X-like)
  - Add Chirp font? (YAGNI — skip for now, add later if requested)
- `frontend/src/lib/composer/previews/InstagramPreview.svelte` — IG card:
  - Avatar + name + "•••" menu
  - 585px-tall image area (carousel if multiple media)
  - Heart / Comment / Share / Save icon row
  - Caption with mention highlighting
- `frontend/src/lib/composer/previews/LinkedInPreview.svelte` — LinkedIn card:
  - Avatar + name + headline + "•••" menu
  - Content (article-style if long)
  - Reactions row (Like, Celebrate, Support, Love, Insightful, Funny)
- `frontend/src/lib/composer/previews/FacebookPreview.svelte` — FB card:
  - Avatar + name + "•••" menu
  - Content with background-color support (if `settings.background` set)
  - Like / Comment / Share row
- `frontend/src/lib/composer/previews/RedditPreview.svelte` — Reddit card:
  - Subreddit + author + time
  - Title (bold) + content
  - Upvote / Downvote / Comment / Share / Save row
- `frontend/src/lib/composer/previews/ThreadsPreview.svelte` — reuse GeneralPreview (Threads is IG's Twitter clone)
- `frontend/src/lib/composer/previews/BlueskyPreview.svelte` — reuse GeneralPreview (Bluesky is Twitter-like)
- `frontend/src/lib/composer/previews/MastodonPreview.svelte` — reuse GeneralPreview with federated handle
- `frontend/src/lib/composer/previews/DefaultPreview.svelte` — fallback (alias of GeneralPreview)
- Registry: `frontend/src/lib/composer/previews/index.ts` — `Map<provider, Component>`
- All previews read from the composer store (current values + integration metadata)
- **Security:** all content rendered with `{@html}` MUST go through `html_escape()` first (we already do this in RichTextEditor output; previews must do the same). No `dangerouslySetInnerHTML` equivalent — Svelte's `{@html}` is already the escape hatch, but we sanitize on input.

**Files touched:** ~12 new
**Estimated effort:** 1-2 iterations (one per preview, but they share patterns)
**Architectural check:** ✅ Security-first (escape all HTML), ✅ YAGNI (skip per-platform fonts, skip video previews for now)

---

### Phase 5 — Posts List: Inline Quick-Edit + Filter/Sort Upgrade

**Deliverables:**
- `routes/posts/+page.svelte` row click → `composer.openEdit(post.id)` (modal, not page nav)
- Add inline quick-edit: hover a row → click a "quick edit" pencil icon → row expands to a mini-composer (title + content + schedule) inline, no modal. Save calls `postsApi.update()`. Useful for typo fixes without opening the full modal.
- Filters (add to existing state tabs):
  - Tag filter (multi-select dropdown)
  - Channel filter (multi-select dropdown)
  - Date range (from-to date inputs)
  - Search input (server-side `?q=` on `/api/posts` — needs backend support)
- Sort dropdown: by scheduled date (default), by created date, by engagement (likes+comments+shares)
- Bulk actions (add to existing delete):
  - Bulk reschedule (with offset: "spread over X minutes")
  - Bulk duplicate
  - Bulk publish-now
  - Bulk move-to-draft
  - Bulk assign-tag
- Pagination: add page-size selector (10/20/50/100) + jump-to-page input
- Backend: `GET /api/posts` needs `?q=`, `?tag_ids=`, `?integration_ids=`, `?from=`, `?to=`, `?sort=` params. Currently only supports `?state=` and pagination.

**Files touched:** ~1 frontend + ~1 backend
**Estimated effort:** 1 iteration
**Architectural check:** ✅ all 6; triple-interface parity maintained (MCP/CLI can add search/filter params too)

---

### Phase 6 — Composer Quality-of-Life

**Deliverables:**
- Two-column responsive: on mobile (`< lg`), stack to single column with preview collapsible
- Sticky footer: tags + repeat + delete (edit mode) + date-popover + Save Draft + Schedule (hover → Post Now)
- Date-popover: click the date pill → opens a popover with a mini-calendar + time input + "✨ Auto-schedule" button (calls `postsApi.findSlot()`)
- AI assistant: keep as side panel toggle (don't make it floating — YAGNI on CopilotKit)
- Emoji picker: already have — verify it works in the modal context
- Post Sets: when opening the composer for create, if any sets exist, show a "Load from Set" button that opens the PostSetModal; on select, prefill the composer
- Media upload: keep existing MediaUpload, but add a "drag onto editor" affordance (drop image directly on RichTextEditor → uploads + inserts)
- Thread finisher: improve UX — show thread items as draggable cards with per-item delay input
- First comment: move from bottom-of-page to per-channel settings (only show for LinkedIn/Facebook)
- Music picker: keep Instagram-only, move to per-channel settings

**Files touched:** ~8 modified
**Estimated effort:** 1 iteration
**Architectural check:** ✅ all 6

---

### Phase 7 — Calendar Polish

**Deliverables:**
- Timezone awareness: calendar grid renders in user's selected timezone (already done for event times; verify the grid header and hour labels match)
- Today indicator: highlight current day column (week view) / current day cell (month view) / current hour row (week/day view)
- Mini-calendar in sidebar: small month calendar for quick date navigation (click a day → jumps to that week)
- Channel filter: checkbox list in sidebar to show/hide events per channel
- Search within calendar: search input that highlights matching events
- List view: add sort dropdown (by date / by channel / by state)
- Month view: "+N more" click → expands to day view for that day
- Empty-state: when no posts in view, show a friendly empty state with "Create your first post" button

**Files touched:** ~5 modified
**Estimated effort:** 1 iteration
**Architectural check:** ✅ all 6

---

### Phase 8 — YAGNI Cleanup Post-Refactor

**Deliverables:**
- Delete `routes/posts/new/+page.svelte` (replaced by ComposerModal) — or keep as a thin redirect for direct-link compatibility
- Delete `routes/posts/[id]/+page.svelte` inline edit code (replaced by ComposerModal) — keep as read-only detail view
- Delete `calendar/PostDetail.svelte` side panel (replaced by direct modal open) — OR keep as a quick-view that doesn't trigger a route change
- Delete `calendar/PostStatsModal.svelte` hand-rolled overlay (migrated to ModalManager)
- Delete hand-rolled overlays in `routes/posts/[id]/+page.svelte` (tag picker)
- Delete hand-rolled overlays in `+layout.svelte` (keyboard cheat-sheet, mobile sidebar)
- Audit for any remaining `goto('/posts/new')` or `goto('/posts/[id]')` calls and redirect to `composer.openCreate()` / `composer.openEdit()`

**Files touched:** ~5 deleted + ~3 modified
**Estimated effort:** 0.5 iteration
**Architectural check:** ✅ YAGNI satisfied

---

## Part C — Sequencing & Dependencies

```
Phase 0 (Modal Store) ─────┬──> Phase 1 (Calendar DnD)
                            ├──> Phase 2 (Composer Modal) ──┬──> Phase 3 (Content Model)
                            │                                ├──> Phase 4 (Preview Pane)
                            │                                ├──> Phase 6 (QoL)
                            │                                └──> Phase 8 (Cleanup)
                            └──> Phase 5 (Posts List)
                                                             └──> Phase 7 (Calendar Polish)
```

- **Phase 0 is blocking** — everything else needs the modal store.
- **Phase 2 is the central phase** — Phases 3, 4, 6, 8 all extend the composer modal.
- **Phase 1 and 5 are independent** — can be done in parallel with Phase 2.
- **Phase 7 is pure polish** — do last.

**Recommended order:** 0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

---

## Part D — What We Explicitly Skip (YAGNI + Architectural Filters)

| Postiz feature | Skip reason |
|---|---|
| Team collaboration, roles, impersonation | Violates single-user preference (AGENTS.md §0.5.1) |
| Customer groups, marketplace, payouts | Violates solo-founder preference (AGENTS.md §0.5.2) |
| Temporal.io workflow orchestration | Violates single-binary preference (AGENTS.md §0.5.3) |
| SWR polling | Violates realtime-first preference (AGENTS.md §0.5.6) — we use SSE |
| CopilotKit AI assistant | YAGNI — we have our own AiAssistant component |
| Polotno image editor | YAGNI — too heavy for solo-founder scope |
| Web3 / wallet auth | YAGNI — single-user APP_PASSWORD is simpler |
| Browser extension routes | YAGNI — out of scope |
| Per-platform fonts (Chirp, Charter, SFNS) | YAGNI — bundle size not worth the fidelity |
| Mantine UI library | YAGNI — we have Tailwind + small UI primitive set |
| `react-dnd` library | We'll use native HTML5 DnD (Postiz proves it's sufficient) + add pointer-events fallback for touch |
| Uppy file uploader | YAGNI — our MediaUpload is simpler and sufficient |
| `react-hook-form` + `class-validator` per-platform DTOs | We use Svelte 5 runes + manual validation; backend already validates via `validate_post()` |
| NestJS backend / Prisma | We use Rust/Axum + sqlx — non-negotiable |
| SweetAlert2 confirm dialogs | We'll build `areYouSure()` as a Svelte modal (consistent with our ModalManager) |
| 14-locale i18n | YAGNI — single-user app, English-only for now |

---

## Part E — Success Metrics

After Phase 8, the following should be true:

1. **No route change for create/edit post** — opening the composer from anywhere (calendar, dashboard, posts list, keyboard `n`) opens a modal, never navigates.
2. **Calendar default is week view** — persisted across reloads.
3. **Drag-and-drop has hour precision** in week/day views, with undo toast and published-post safety modal.
4. **Composer has live per-platform preview** — switching the channel tab instantly swaps the preview to that platform's chrome.
5. **Multi-channel content model supports threads + per-channel overrides** — clone-on-first-divergence pattern.
6. **All overlays go through ModalManager** — no hand-rolled `fixed inset-0` overlays remain.
7. **Posts list supports search, tag/channel/date filters, sort, and inline quick-edit.**
8. **Zero regression** — `cargo check`, `cargo test`, `svelte-check`, `vite build` all pass; no functional regressions in CLI/MCP/REST parity.
9. **Architectural preferences intact** — still single-user, single-binary, triple-interface, security-first, realtime-first, YAGNI.

---

## Part F — Risk Register

| Risk | Mitigation |
|---|---|
| Modal store introduces z-index/focus-trap bugs | Test with stacked modals (composer + confirm + tag picker); verify Escape closes topmost only |
| Composer modal is too dense on mobile | Responsive: stack to single column on `< lg`; preview pane collapses to a tab |
| Per-platform preview HTML is an XSS vector | All preview content goes through `html_escape()` before `{@html}`; never render raw user input |
| Drag-to-reschedule on touch devices is broken | Pointer-events fallback; if too complex, ship HTML5-only first and add touch in a follow-up |
| Deleting `/posts/new` and `/posts/[id]` breaks deep links | Keep the routes as thin redirects that open the modal |
| Backend `PUT /posts/{id}/date` action param is a breaking change | Add as optional param; default to current behavior (`'reschedule'`) |
| Posts list search needs backend `?q=` support | Add in Phase 5; coordinate with CLI/MCP to add the same param for parity |

---

## Part G — Implementation Notes

- **Follow AGENTS.md golden rules:** push after each phase, never break the build, security review every HTML change, runtime sqlx queries only.
- **Each phase is one commit** (or two for Phase 2 which is larger). Don't batch phases.
- **Test after each phase:** `cargo check --lib --bin social-forge`, `cargo test --lib`, `cd frontend && pnpm build && svelte-check`.
- **Update worklog.md after each phase** with what was done and any decisions that deviated from this plan.
- **If a phase reveals a deeper issue**, stop and document it before proceeding. Don't pile changes on top of a shaky foundation.

---

## Part H — Post-Implementation Audit

After Phase 8, run a fresh contrast audit against Postiz to verify:
- All 9 success metrics are met.
- No new UX regressions vs. the current v17 state.
- The frontend is now structurally closer to Postiz's modal-based workflow while remaining a SvelteKit app with Rust backend.
- Architectural preferences are intact (re-read AGENTS.md §0.5 and verify each one).

---

**End of plan. Implementation can begin with Phase 0.**
