# Remaining Work Implementation Plan

## Executive Summary

This plan covers the remaining work to bring social-forge to production-ready status. The comment/DM/automation infrastructure is complete; this plan focuses on fixing data flow bugs, completing core workflows, and adding UX polish.

---

## Current State Assessment

### Completed (This Session)
- ✅ SocialProvider trait extension with DM methods
- ✅ Generic comment/DM MCP tools (11 tools)
- ✅ Automation engine (rules, cooldowns, AI responses)
- ✅ Platform-specific tools: X/Twitter, LinkedIn, Instagram, YouTube, Bluesky, Mastodon
- ✅ MCP-CLI parity for all comment/DM/automation commands
- ✅ Database migration for automation rules

### Remaining Work (from IMPLEMENTATION_PLAN.md)

#### P0 — Data Flow Breaks (Critical Bugs)
| # | Gap | Impact | Files |
|---|-----|--------|-------|
| 1 | Media not sent from frontend to backend | Media uploaded but never attached to post | `frontend/src/routes/posts/new/+page.svelte` |
| 2 | Media not passed to provider during publish | Scheduler builds `PostContent { media: vec![] }` always | `src/scheduler/mod.rs` |
| 3 | Per-channel content overrides not sent | `providerOverride` Map exists but never included in API call | `frontend/src/routes/posts/new/+page.svelte` |
| 4 | first_comment not published after main post | Stored in DB but scheduler never calls `provider.comment()` | `src/scheduler/mod.rs` |

#### P1 — Missing Core Workflows
| # | Gap | Impact | Files |
|---|-----|--------|-------|
| 5 | No "create post on date click" in calendar | Users must navigate away to composer | `frontend/src/routes/calendar/+page.svelte` |
| 6 | posting_times not used for slot finding | `find_slot` just adds 2h to last post | `src/api/posts.rs` |
| 7 | No post editing from calendar detail panel | PostDetail shows info but can't edit inline | `frontend/src/lib/calendar/PostDetail.svelte` |
| 8 | Recurring posts not sent from frontend | `recurring` state exists but never included in submit() | `frontend/src/routes/posts/new/+page.svelte` |

#### P2 — UX Polish
| # | Gap | Impact | Files |
|---|-----|--------|-------|
| 9 | No bulk operations | Can't select multiple posts for batch schedule/delete | Calendar + post list |
| 10 | No draft auto-save | Composer state lost on navigation | Composer page |
| 11 | Post detail uses prompt() for rescheduling | Should use proper date picker | `frontend/src/routes/posts/[id]/+page.svelte` |
| 12 | No "Post Now" button in composer | Only "Publish" (creates draft if no schedule) | Composer page |

---

## Implementation Plan

### Phase 1: Fix Data Flow (P0 — ~2 hours)

**Priority**: Critical — posts are being created without media and published without attachments.

#### 1.1 Send media from frontend to backend
**File**: `frontend/src/routes/posts/new/+page.svelte`
- Update `submit()` to include `media` in the API call
- Add `media` field to `CreatePostRequest` type

#### 1.2 Pass media to provider during publish
**File**: `src/scheduler/mod.rs` — `publish_post()`
- Parse media JSONB → `Vec<MediaAttachment>`
- Replace `media: vec![]` with actual media from post

#### 1.3 Send per-channel content overrides
**File**: `frontend/src/routes/posts/new/+page.svelte`
- When `providerOverride` has entries, send per-integration content
- Add `overrides` field to `CreatePostRequest`

**File**: `src/api/posts.rs`
- Add `PostOverride` struct
- Handle per-integration content in `create()`

#### 1.4 Publish first_comment after main post
**File**: `src/scheduler/mod.rs` — after successful publish
- Check if `first_comment` exists and is non-empty
- Call `provider.comment()` with the comment content
- Log warning on failure (don't fail whole post)

### Phase 2: Complete Core Workflows (P1 — ~3 hours)

#### 2.1 Calendar "create on date click"
**File**: `frontend/src/routes/calendar/+page.svelte`
- Add `onDateClick` handler to MonthView
- Navigate to composer with pre-filled date

**File**: `frontend/src/routes/posts/new/+page.svelte`
- Read `?date=` query param on mount
- Pre-fill schedule picker

#### 2.2 Smart slot finding using posting_times
**File**: `src/api/posts.rs` — `find_slot` handler
- Get integration's `posting_times` JSONB
- Find next available slot matching one of those times
- Skip slots that already have a post scheduled

#### 2.3 Inline editing from calendar PostDetail
**File**: `frontend/src/lib/calendar/PostDetail.svelte`
- Add "Edit" button that navigates to `/posts/{id}`

#### 2.4 Send recurring config from frontend
**File**: `frontend/src/routes/posts/new/+page.svelte`
- After creating post, if `recurring` is set, call repeat endpoint

### Phase 3: UX Polish (P2 — ~2 hours)

#### 3.1 Bulk operations
- Add checkbox selection to calendar events and post list
- Show floating action bar with "Delete All" / "Reschedule All"
- Backend: add `POST /api/posts/bulk-delete` and `POST /api/posts/bulk-schedule`

#### 3.2 Draft auto-save
- Use `localStorage` to persist composer state
- Debounce-save on state change
- Restore on mount

#### 3.3 Proper date picker for rescheduling
- Replace `prompt()` in post detail with `SchedulePicker` component

#### 3.4 "Post Now" button
- Add second button in composer
- Call `postsApi.create()` without `scheduled_at`
- Immediately call `POST /api/posts/{id}/publish`

---

## Execution Order

```
Phase 1 (P0 — do first, these are bugs):
  1.1 → 1.2 → 1.4 → 1.3  (media flow end-to-end, then first_comment, then overrides)

Phase 2 (P1 — core features):
  2.4 → 2.1 → 2.2 → 2.3  (recurring is trivial, then calendar UX, then smart slots)

Phase 3 (P2 — polish):
  3.4 → 3.2 → 3.3 → 3.1  (post now is quick, auto-save, picker, bulk is biggest)
```

## Estimated Total: ~7 hours

- Phase 1: 2h (critical bug fixes)
- Phase 2: 3h (core workflow completion)
- Phase 3: 2h (UX polish)

## Testing Plan

### Unit Tests
1. Test media parsing from JSONB
2. Test first_comment posting logic
3. Test slot finding with posting_times

### Integration Tests
1. Test end-to-end media flow (upload → create → publish)
2. Test per-channel content overrides
3. Test calendar date click → composer flow

### Manual Testing
1. Upload media → create post → verify media attached
2. Set first_comment → publish → verify comment posted
3. Click date in calendar → verify composer opens with date
4. Test "Post Now" button
5. Test bulk operations

## Deployment Checklist

- [ ] All P0 bugs fixed
- [ ] All P1 workflows complete
- [ ] All P2 polish items done
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Manual testing complete
- [ ] Documentation updated
- [ ] Service restarted
