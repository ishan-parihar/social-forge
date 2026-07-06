# Social Forge v20 — Implementation Gaps Audit & Upgrade Plan

**Date:** 2026-07-06
**Status:** Plan ready for implementation
**Builds on:** v18 (modal-based UX) + v19 (Postiz gap closure)

---

## Executive Summary

A post-v19 user audit identified 6 implementation gaps. Deep investigation found that **2 of them share a single root cause** (misplaced `impl` blocks — comment replies and DMs are never dispatched to provider implementations), and the analytics bug is caused by a **frontend/backend contract mismatch** (wrong field names). The plan below addresses all 6 gaps in 8 phases, prioritized by impact.

---

## Problem Audit (6 findings)

### Problem 1: Feed has no CRUD for Posts

**Current state:** The feed shows imported external posts with "View original" + "Hide" buttons. "Hide" hard-deletes the row.

**What's missing:**
- ❌ "Repurpose" button — create a Social Forge post inspired by a feed item (composer store already supports `prefilledContent`)
- ❌ Save/bookmark feed items for later
- ⚠️ "Hide" permanently deletes instead of soft-hiding (re-import brings it back)

**Fix:** Add "Repurpose" button (2-line change), add `hidden_at` column for soft-hide, add `saved_at` column for bookmarks.

---

### Problem 2: Comments Dashboard is not efficacious

**Root cause:** `reply_to_comment`, `send_dm`, `get_dm_conversations`, `get_dm_messages` are defined as **inherent methods** (inside `impl ProviderType { … }`) instead of **trait overrides** (inside `impl SocialProvider for ProviderType { … }`). The backend holds providers as `Arc<dyn SocialProvider>`, so trait dispatch always hits the default impls (which return errors/empty vecs). **This is the same root cause as Problem 3 (DMs).**

**What's missing:**
- ❌ Reply is broken (silently fails for every provider)
- ❌ `comment_received` event is never broadcast (frontend subscription is dead code)
- ❌ Status filter (`new`/`resolved`) is declared but never rendered in the UI
- ❌ No search
- ❌ No bulk actions
- ❌ No engagement metrics on comments (likes, reply count)

**Fix:** Move 4 methods from inherent blocks to trait impl blocks across 7 provider files (mechanical code move). Broadcast `comment_received` after cache refresh. Render status filter. Add search.

---

### Problem 3: DMs are always empty

**Root cause:** Same as Problem 2 — DM methods are in inherent blocks, trait dispatch hits defaults.

**What's missing:**
- ❌ DM methods never dispatched (same fix as comments)
- ❌ Frontend defaults to `integrations[0]` without filtering to DM-capable providers
- ❌ `dm_received` event never broadcast
- ❌ `ConversationResponse` missing `platform` field

**Fix:** Same trait-block move as Problem 2. Filter integration dropdown to DM-capable providers. Add `platform` field to response.

---

### Problem 4: Analytics always shows 0

**Root cause:** Multiple compounding bugs:

- **Bug A (critical):** Frontend type expects `total_posts` and `total_impressions`, but backend returns `posts_with_engagement` and `total_views`. Both fields are `undefined`, so `undefined > 0` is `false` → card never renders.
- **Bug B:** `/api/feed/analytics` ignores the `days` query param — always returns lifetime totals.
- **Bug C:** `posts_by_provider` returns `provider_name` (e.g. "X (Twitter)") but frontend filters by `provider_identifier` (e.g. "x") — filter never matches.
- **Bug D:** Top-posts list doesn't LEFT JOIN `analytics_cache` — engagement fields are always null.
- **Bug E:** Only 9 of ~30 providers implement `fetch_engagement()` — 21 providers never populate the engagement table.

**Fix:** Fix the contract mismatch (align field names). Add `days` param to the query. Fix the provider filter. LEFT JOIN analytics_cache on the posts list. (Extending all 21 providers is a larger follow-up.)

---

### Problem 5: App icons are emojis, not real logos

**Current state:** Two parallel systems — `ProviderIcon.svelte` uses emoji glyphs (🐦 📘 📷), `providers.ts` uses text abbreviations ("X" "R" "in"). No SVG logo assets exist.

**Fix:** Create SVG logo files for each provider, replace `ProviderIcon.svelte` emoji map with `<img>` lookup, unify the two icon systems.

---

### Problem 6: No campaign/kanban for content pipeline

**Current state:** No kanban/board/pipeline route exists. `group_id` is used for thread/repeat batches only — no campaigns table, no pipeline stages, no ideation workflow.

**What's missing:**
- ❌ Campaign entity (name, description, color, dates, goals)
- ❌ Kanban board view with drag-and-drop columns
- ❌ Pipeline stages beyond draft/queued/published (no "idea" or "in_review" stage)
- ❌ Idea capture UX (quick-add without full composer)
- ❌ Campaign management UI

**Fix:** Add `campaigns` table + `campaign_id` FK on posts. Extend `post_state` with `'idea'`. Add `/kanban` route with board component. Add "Save as Idea" button in composer.

---

## Implementation Plan (8 Phases)

### Phase 1 — Fix provider trait dispatch (CRITICAL — unblocks comments + DMs)

**Why first:** This single mechanical fix unblocks comment replies, DM listing, DM reading, and DM sending across 7 provider files. Highest ROI.

**Deliverables:**
- Move `reply_to_comment`, `send_dm`, `get_dm_conversations`, `get_dm_messages` from inherent `impl ProviderType { … }` blocks into `impl SocialProvider for ProviderType { … }` blocks for:
  - `src/social/x.rs`
  - `src/social/linkedin.rs`
  - `src/social/instagram.rs`
  - `src/social/bluesky.rs`
  - `src/social/mastodon.rs`
  - `src/social/youtube.rs`
  - `src/social/instagram_standalone.rs`
  - `src/social/reddit.rs` (also fix `send_dm` signature mismatch)
- No logic changes — pure code move.

**Effort:** 1 iteration (mechanical, but touches 8 files)

---

### Phase 2 — Fix analytics contract mismatch (CRITICAL — analytics shows 0)

**Why second:** The field-name mismatch is the reason analytics always shows 0. A 10-line frontend fix will make the existing data visible.

**Deliverables:**
- **Frontend:** Fix `frontend/src/lib/api/feed.ts` — align type with actual backend response (`posts_with_engagement` instead of `total_posts`, `total_views` instead of `total_impressions`).
- **Frontend:** Fix `frontend/src/routes/analytics/+page.svelte` — update field references.
- **Backend:** Add `days: Option<i32>` to `AnalyticsQuery` in `src/api/feed.rs` + pass cutoff to `get_engagement_summary`.
- **Backend:** Fix `posts_by_provider` query to select `provider_identifier` instead of `provider_name`.
- **Backend:** LEFT JOIN `analytics_cache` on `list_posts` / `list_posts_search` so engagement fields populate.

**Effort:** 1 iteration

---

### Phase 3 — Feed CRUD: Repurpose + soft-hide + bookmark

**Deliverables:**
- **Frontend:** Add "Repurpose" button to feed post cards → `composer.openCreate(undefined, undefined, post.text)`
- **Migration 025:** Add `hidden_at TIMESTAMPTZ NULL` and `saved_at TIMESTAMPTZ NULL` to `external_posts`
- **Backend:** Change `DELETE /api/feed/{id}` to soft-hide (`UPDATE … SET hidden_at = NOW()`) instead of hard delete
- **Backend:** Add `WHERE hidden_at IS NULL` to feed list queries
- **Backend:** Add `POST /api/feed/{id}/save` and `DELETE /api/feed/{id}/save` endpoints
- **Frontend:** Add "Save" / "Saved" toggle button. Add "Saved" filter chip.

**Effort:** 1 iteration

---

### Phase 4 — Comments dashboard upgrade

**Deliverables:**
- **Backend:** Broadcast `comment_received` event after `refresh_all_comments` completes in `src/feed/mod.rs`
- **Frontend:** Render the `statuses` filter row (All / New / Resolved) — currently declared but never rendered
- **Frontend:** Add search input (client-side filter on comment text/author for now; backend `?q=` can follow)
- **Frontend:** Add bulk-select checkboxes + "Resolve all selected" button
- **Migration 026:** Add `like_count`, `reply_count`, `parent_comment_id` to `cached_comments`
- **Backend:** Surface these in `CommentItem` response

**Effort:** 1 iteration

---

### Phase 5 — DMs dashboard fix

**Deliverables:**
- **Frontend:** Filter integration dropdown to DM-capable providers (`x`, `instagram`, `linkedin`) — don't default to a non-DM provider
- **Backend:** Add `platform` field to `ConversationResponse` so the platform icon renders
- **Frontend:** Fix the misleading empty-state message
- **Backend (future):** Add a DM poller (mirroring `refresh_all_comments`) that broadcasts `dm_received` — deferred for now since it requires provider-specific polling logic

**Effort:** 0.5 iteration

---

### Phase 6 — Real SVG platform logos

**Deliverables:**
- Create `frontend/static/icons/platforms/` directory
- Add SVG logo files for all 29 providers in `PROVIDERS` map (x.svg, reddit.svg, linkedin.svg, facebook.svg, instagram.svg, threads.svg, bluesky.svg, mastodon.svg, youtube.svg, tiktok.svg, pinterest.svg, discord.svg, slack.svg, telegram.svg, whatsapp.svg, github.svg, wordpress.svg, medium.svg, devto.svg, hashnode.svg, vk.svg, kick.svg, skool.svg, lemmy.svg, gmail.svg, drive.svg, google.svg, google-my-business.svg, instagram-standalone.svg, linkedin-page.svg)
- Rewrite `ProviderIcon.svelte` to use `<img src="/icons/platforms/{provider}.svg">` with emoji fallback for unknown providers
- Update `providers.ts` — the `icon` field becomes optional (SVG is the primary icon; `icon` is the fallback)

**Effort:** 1 iteration (SVG sourcing + component rewrite)

---

### Phase 7 — Campaign + Kanban board

**Deliverables:**
- **Migration 027:** Add `campaigns` table (`id, user_id, name, description, color, start_date, end_date, goal, created_at`)
- **Migration 028:** Add `campaign_id UUID NULL REFERENCES campaigns(id)` to `posts` table. Extend `post_state` enum with `'idea'`.
- **Backend:** CRUD for campaigns (`GET/POST/PUT/DELETE /api/campaigns`)
- **Backend:** `PATCH /api/posts/{id}/stage` — change post state (for kanban drag-and-drop)
- **Frontend:** New `/kanban` route with board component:
  - Columns: Ideas → Drafts → Scheduled → Published
  - Cards: post title + channel icon + campaign color
  - Drag-and-drop between columns (calls `PATCH /api/posts/{id}/stage`)
  - "Quick Add Idea" input at the top of the Ideas column
- **Frontend:** "Save as Idea" button in ComposerModal (creates post with state='idea', no scheduled_at)
- **Frontend:** Wire "Group by Campaign" in posts list to use campaign names instead of UUIDs
- **Sidebar:** Add "Kanban" nav item under "Publish"

**Effort:** 2 iterations (backend schema + API + frontend board component)

---

### Phase 8 — Calendar sophistication polish

**Deliverables:**
- Calendar event cards: show campaign color stripe if post has a `campaign_id`
- Calendar sidebar: list campaigns with post counts, click to filter
- Composer: campaign selector dropdown (assign post to a campaign)
- Posts list: campaign column in the flat view
- Dashboard: "Active Campaigns" widget showing campaign progress (posts scheduled vs published)

**Effort:** 1 iteration

---

## Sequencing

```
Phase 1 (trait dispatch) ──┬──> Phase 4 (comments upgrade)
                           └──> Phase 5 (DMs fix)
Phase 2 (analytics fix) ────── independent
Phase 3 (feed CRUD) ────────── independent
Phase 6 (SVG logos) ────────── independent
Phase 7 (kanban) ─────┬──────> Phase 8 (calendar polish)
                      └── depends on campaigns table from Phase 7
```

**Recommended order:** 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8

Phases 1 and 2 are critical bug fixes (highest impact, lowest effort). Phases 3-5 are feature completions. Phase 6 is polish. Phase 7-8 are new features.

---

## What We Skip (Architectural Filters)

| Item | Skip reason |
|---|---|
| Extending all 21 providers with `fetch_engagement()` | Too large for one iteration — do incrementally per provider as needed |
| DM background poller | Requires provider-specific polling logic; deferred until DM trait fix is verified working |
| Per-provider fonts (Chirp, Charter) | YAGNI — bundle size not worth it |
| Full mobile-responsive kanban | Deferred — kanban is desktop-first; mobile can use the posts list |

---

**End of plan. Implementation begins with Phase 1 (trait dispatch fix).**
