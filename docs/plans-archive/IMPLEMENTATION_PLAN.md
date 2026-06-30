# Implementation Plan: Scheduling & Content Management at Scale

## Current State Summary

Social-forge already has a **surprisingly complete** implementation:

**Backend (fully functional):**
- Full post CRUD API with multi-channel support (`integration_ids: Vec<Uuid>`)
- 30s scheduler with retry (3 attempts, token refresh, rate-limit backoff)
- Thread support (group_id + sequence)
- Recurring posts (interval_days + end_date, up to 100 copies)
- Calendar API (date-range queries)
- SSE real-time events (post_created, post_published, post_failed, post_deleted)
- Media upload/management API
- Tag system, signatures, RSS automation, webhooks
- 30+ social providers with unified `SocialProvider` trait

**Frontend (SvelteKit 5 SPA, functional):**
- Post composer with channel selector, rich text (TipTap), schedule picker, auto-schedule, recurring, thread finisher, first comment, AI assist, media upload, per-provider editors, tags, post sets
- Calendar with 4 views (month/week/day/list), drag-drop rescheduling, duplicate, delete, stats modal
- Post list with state filters and pagination
- Channel management, analytics, media library, settings

---

## Critical Gaps (Ordered by Impact)

### P0 — Data Flow Breaks (Posts fail silently or lose data)

| # | Gap | Impact | Location |
|---|-----|--------|----------|
| 1 | **Media not sent from frontend to backend** | Media uploaded but never attached to post | `frontend/src/routes/posts/new/+page.svelte` submit() |
| 2 | **Media not passed to provider during publish** | Scheduler builds `PostContent { media: vec![] }` always | `src/scheduler/mod.rs` line 180 |
| 3 | **Per-channel content overrides not sent** | `providerOverride` Map exists in frontend but never included in API call | `frontend/src/routes/posts/new/+page.svelte` submit() |
| 4 | **first_comment not published after main post** | Stored in DB but scheduler never calls `provider.comment()` | `src/scheduler/mod.rs` publish_post() |

### P1 — Missing Core Workflows

| # | Gap | Impact | Location |
|---|-----|--------|----------|
| 5 | **No "create post on date click" in calendar** | Users must navigate away to composer; postiz opens modal on date click | `frontend/src/routes/calendar/+page.svelte` |
| 6 | **posting_times not used for slot finding** | `find_slot` just adds 2h to last post; ignores per-integration optimal times | `src/api/posts.rs` find_slot handler |
| 7 | **No post editing from calendar detail panel** | PostDetail shows info but can't edit content inline | `frontend/src/lib/calendar/PostDetail.svelte` |
| 8 | **Recurring posts not sent from frontend** | `recurring` state exists in composer but never included in submit() | `frontend/src/routes/posts/new/+page.svelte` |

### P2 — UX Polish (Postiz Parity)

| # | Gap | Impact | Location |
|---|-----|--------|----------|
| 9 | **No bulk operations** | Can't select multiple posts for batch schedule/delete | Calendar + post list |
| 10 | **No draft auto-save** | Composer state lost on navigation | Composer page |
| 11 | **Post detail uses prompt() for rescheduling** | Should use proper date picker | `frontend/src/routes/posts/[id]/+page.svelte` |
| 12 | **No "Post Now" button in composer** | Only "Publish" (which creates draft if no schedule) | Composer page |

---

## Implementation Plan

### Phase 1: Fix Data Flow (P0 bugs — ~2 hours)

These are the most critical: posts are being created without media and published without attachments.

#### 1.1 Send media from frontend to backend

**File:** `frontend/src/routes/posts/new/+page.svelte`

Change `submit()` to include `media` in the API call:
```typescript
const r = await postsApi.create({
  integration_ids: selectedIntegrations,
  content,
  title: title || undefined,
  scheduled_at: scheduledAt || undefined,
  tag_ids: selectedTagIds,
  first_comment: firstComment || undefined,
  media: mediaItems.length > 0 ? mediaItems : undefined,  // ADD THIS
});
```

**File:** `frontend/src/lib/api/posts.ts`

Update the `create` type signature to accept `media`:
```typescript
create: (d: { ...; media?: MediaItem[] }) => ...
```

#### 1.2 Pass media to provider during publish

**File:** `src/scheduler/mod.rs` — `publish_post()`

Replace `media: vec![]` with actual media from the post's JSONB:
```rust
let media: Vec<MediaAttachment> = post.media
    .as_ref()
    .and_then(|v| serde_json::from_value(v.clone()).ok())
    .unwrap_or_default();

let content = PostContent {
    content: post.content.clone(),
    media,
    settings: post.settings.clone(),
};
```

#### 1.3 Send per-channel content overrides

**File:** `frontend/src/routes/posts/new/+page.svelte`

When `providerOverride` has entries, send per-integration content:
```typescript
// If any per-channel overrides exist, create separate posts per channel
if (providerOverride.size > 0) {
  // For channels with overrides, send individual creates
  // For channels without overrides, use global content
}
```

**Backend already supports this** — `create_posts_for_integrations` creates one post per integration_id with the same content. To support per-channel overrides, add a new field to `CreatePostRequest`:

```rust
pub struct CreatePostRequest {
    // ... existing fields ...
    pub overrides: Option<HashMap<Uuid, PostOverride>>,  // per-integration content
}

pub struct PostOverride {
    pub content: Option<String>,
    pub media: Option<serde_json::Value>,
    pub settings: Option<serde_json::Value>,
}
```

Then in `create()`, when building posts for each integration, check if an override exists.

#### 1.4 Publish first_comment after main post

**File:** `src/scheduler/mod.rs` — after successful publish in `publish_post()`

```rust
// After successful publish, post first_comment if present
if let Some(ref comment) = post.first_comment {
    if !comment.is_empty() {
        let comment_content = PostContent {
            content: comment.clone(),
            media: vec![],
            settings: serde_json::json!({}),
        };
        if let Err(e) = provider.comment(
            &access_token,
            &result.platform_post_id,
            None,  // no parent comment
            &comment_content,
        ).await {
            tracing::warn!("Failed to post first_comment for {}: {e}", post.id);
            // Don't fail the whole post for a comment failure
        }
    }
}
```

---

### Phase 2: Complete Core Workflows (P1 — ~3 hours)

#### 2.1 Calendar "create on date click"

**File:** `frontend/src/routes/calendar/+page.svelte`

Add `onDateClick` handler to MonthView that navigates to composer with pre-filled date:
```typescript
function handleDateClick(date: string) {
  goto(`/posts/new?date=${date}`);
}
```

**File:** `frontend/src/routes/posts/new/+page.svelte`

On mount, read `?date=` query param and pre-fill the schedule picker:
```typescript
import { page } from "$app/stores";
onMount(() => {
  const dateParam = $page.url.searchParams.get("date");
  if (dateParam) {
    scheduledAt = `${dateParam}T09:00:00.000Z`;
  }
});
```

#### 2.2 Smart slot finding using posting_times

**File:** `src/api/posts.rs` — `find_slot` handler

Current logic: last scheduled + 2 hours. New logic:
1. Get integration's `posting_times` JSONB (array of `{time: minutes_from_midnight}`)
2. Find the next available slot that matches one of those times
3. Skip slots that already have a post scheduled

```rust
// Pseudocode for improved find_slot:
let posting_times = integration.posting_times; // e.g., [120, 400, 700] minutes
let existing_posts = queries::get_scheduled_posts_after(db, user_id, integration_id, Utc::now()).await?;
let existing_dates: HashSet<_> = existing_posts.iter()
    .map(|p| (p.scheduled_at.date_naive(), p.scheduled_at.time().hour() * 60 + p.scheduled_at.time().minute()))
    .collect();

// Walk forward from today, checking each posting_time slot
for day_offset in 0..30 {
    let date = Utc::now().date_naive() + chrono::Duration::days(day_offset);
    for &minutes in &posting_times {
        let slot = date.and_hms_opt(minutes / 60, minutes % 60, 0).unwrap();
        if slot > Utc::now().naive_utc() && !existing_dates.contains(&(date, minutes)) {
            return Ok(slot);
        }
    }
}
```

#### 2.3 Inline editing from calendar PostDetail

**File:** `frontend/src/lib/calendar/PostDetail.svelte`

Add an "Edit" button that either:
- (Simple) navigates to `/posts/{id}` (existing detail page with edit)
- (Better) opens a mini-editor inline in the slide-out panel

The simple approach is sufficient for now since `/posts/[id]` already has edit capability.

#### 2.4 Send recurring config from frontend

**File:** `frontend/src/routes/posts/new/+page.svelte`

After creating the post, if `recurring` is set, call the repeat endpoint:
```typescript
if (recurring && r.data?.posts?.[0]?.id) {
  await postsApi.repeat(r.data.posts[0].id, recurring.intervalDays, recurring.endDate);
}
```

---

### Phase 3: UX Polish (P2 — ~2 hours)

#### 3.1 Bulk operations

Add checkbox selection to calendar events and post list:
- Select multiple → show floating action bar with "Delete All" / "Reschedule All"
- Backend: add `POST /api/posts/bulk-delete` and `POST /api/posts/bulk-schedule`

#### 3.2 Draft auto-save

Use `localStorage` to persist composer state:
```typescript
// On any state change, debounce-save to localStorage
$effect(() => {
  clearTimeout(autoSaveTimer);
  autoSaveTimer = setTimeout(() => {
    localStorage.setItem('composer_draft', JSON.stringify({
      content, title, selectedIntegrations, scheduledAt, mediaItems, firstComment
    }));
  }, 1000);
});

// On mount, restore from localStorage
onMount(() => {
  const saved = localStorage.getItem('composer_draft');
  if (saved) { /* restore state */ }
});
```

#### 3.3 Proper date picker for rescheduling

Replace `prompt()` in post detail with the same `SchedulePicker` component used in the composer.

#### 3.4 "Post Now" button

Add a second button in the composer that calls `postsApi.create()` without `scheduled_at`, then immediately calls `POST /api/posts/{id}/publish` on the first returned post.

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

## Files to Modify

| File | Changes |
|------|---------|
| `frontend/src/routes/posts/new/+page.svelte` | Send media, overrides, recurring; read ?date param; add "Post Now" |
| `frontend/src/lib/api/posts.ts` | Update create() type to include media field |
| `src/scheduler/mod.rs` | Parse media JSONB → MediaAttachment vec; publish first_comment |
| `src/api/posts.rs` | Add overrides to CreatePostRequest; improve find_slot with posting_times |
| `frontend/src/routes/calendar/+page.svelte` | Add onDateClick → navigate to composer |
| `frontend/src/lib/calendar/PostDetail.svelte` | Add "Edit" button linking to /posts/{id} |
| `frontend/src/lib/calendar/MonthView.svelte` | Wire onDateClick callback |

## Estimated Total: ~7 hours

- Phase 1: 2h (critical bug fixes)
- Phase 2: 3h (core workflow completion)
- Phase 3: 2h (UX polish)
