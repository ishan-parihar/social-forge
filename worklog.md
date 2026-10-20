# Worklog — social-forge multi-agent log (§8 protocol)

Before starting work: read this file top to bottom. After finishing: append a
section using the template in AGENTS.md §8 — never overwrite. One-liners can
also be appended via `scripts/kb log "<entry>"` (append-only, the same file).

---
Task ID: kb-adoption-2026-10-17
Agent: omp (memory-wire session)
Task: Adopt the project-kb documentation genome (map-in-place, wiring social-forge's existing conventions to the tooling).

Work Log:
- Surveyed the doc tree: the dated outline succession (OUTLINE-2026-10-16 → -17) already implements dated-frozen + supersede informally; AGENTS.md §9 is the regression ledger; the outline's standing notes are the closed list.
- Mapped roles in `kb.toml`: plans/audits → docs/planning, register → TODO.md (new, seeded from OUTLINE-2026-10-17), decisions → docs/decisions (new), archive → docs/archive (new, tombstone convention), worklog → this file; ledger/closed/regressions/workflows/state/feed/runbook deferred to their native mechanisms (`native:` notes).
- Seeded TODO.md: V1a–V1d/C1/C2/R1 open, P6.1–P6.3 + MCP-identity rejected-on-evidence, B12/WA-inbox blocked with unblock conditions.
- Repaired AGENTS.md §0.6: added the OUTLINE-16/17, TODO.md, worklog.md, docs/decisions rows; replaced the "doesn't exist" rule with the tombstone rule.
- Fixed §8's worklog path (was a foreign machine path, `/home/z/my-project/worklog.md`).
- Copied `scripts/kb` (stdlib-only, single file); `.gitignore` gains `*.lock`.

Stage Summary:
- Artifacts: kb.toml, TODO.md, worklog.md, docs/decisions/README.md, docs/archive/INDEX.md, scripts/kb, AGENTS.md §0.6+§8 edits, .gitignore.
- Verification: `cargo check --lib --bin social-forge` 0 errors (docs-only diff; clippy/tests unaffected); `scripts/kb lint` clean; `scripts/kb doctor` healthy.
- Known follow-ups: §9.4 still references the old machine's `scripts/cargo-env.sh` (left as-is — outside this task's scope); AGENTS.md tool-count assert sites (×3) unchanged — no tool surface touched.

---
Task ID: genome-migration-2026-10-19
Agent: omp (genome migration session)
Task: Migrate social-forge from the pre-genome kb.toml to the genome's own config, and retire the legacy project-kb CLI.

Work Log:
- Read the legacy kb.toml and mapped every role it declared. The role→path map moves to genome.toml unchanged, except the retired `closed` role (same standing-notes list as `ledger`, so `ledger` already carries its meaning) and `dated_pattern`, which moves from [paths] to [core] where the genome reads it.
- The legacy file declared no `asks` and no `records_index`. Both are declared now, so the module set does not silently change: `asks` is a `native:` pointer (this repo has no operator-questions register; open questions live in TODO.md's open/blocked rows), and `records_index` points at the AD store's own README. `records.zones = {}` keeps `genome rec emit` from writing generated zones into that prose.
- `modules = ["core", "records", "agents"]` — the legacy kb.toml set, declared explicitly so the software profile's `core, records` does not silently drop the agents module.
- Added the genome runtime ignores to .gitignore (`*.genome-lock`, `*.genome-tmp`, `.genome/`), matching the fleet convention; the repo's existing `*.lock` did not cover `*.genome-lock`.
- Archived `scripts/kb` to `docs/archive/kb` with a tombstone at the original path and a row in `docs/archive/INDEX.md`: it is a vendored CLI that reads only kb.toml, which this repo no longer has, so it had become a dead second copy of the role map.
- Repointed the live doc references at the genome verbs: AGENTS.md §0.6 (the TODO.md and docs/decisions/ rows), TODO.md's header, docs/decisions/README.md. The dated and frozen docs (docs/planning/*, docs/archive/*, docs/kb/SESSION_2026-10-19.md, TODO.md's row evidence cells) keep `scripts/kb` where it records what was run at the time. This worklog's own header line still names `scripts/kb log` because the worklog is an append-only role: editing a line trips `genome doctor`'s append-only check, so the correction is this entry instead.

Stage Summary:
- Artifacts: genome.toml (was kb.toml), scripts/genome re-pinned to 2026.10.19-3, scripts/kb archived to docs/archive/kb, .gitignore, AGENTS.md, TODO.md, docs/decisions/README.md, docs/archive/INDEX.md, worklog.md.
- Verification: `scripts/genome lint --all` 0 errors / 4 warnings (the same 4 W010s as before the migration; W507 gone with the retired `closed` role). `scripts/genome doctor` healthy. `genome rec emit` is a no-op.

---
Task ID: red-team-web-dashboard-2026-10-19
Agent: omp (red-team + visual iteration session)
Task: Exhaustive red-team of the web dashboard (composer, calendar and scheduling, campaign planning, analytics, research, read-only activity log), fix every defect found, and iterate the visual layer with browseros-neo against a populated instance.

Work Log:
- Four read-only scouts audited the surfaces in parallel (composer/scheduling, calendar/campaigns, analytics/research/activity, and a cross-cutting API-contract diff of 124 axum routes against 102 frontend call sites). Their reports were cross-checked against the source before any fix; every claim that did not hold was dropped.
- The operator's own database had 1 post, 0 campaigns, 0 tags, 0 analytics - so the visual pass could only ever see empty states. Built an isolated seeded instance (a copy of the real DB plus a crafted dataset) on its own port, which is what made the populated-state defects visible: a bar that drew no comparison, a count clipped by its own bar, a date a day early, a metric wearing the wrong name.
- Extended the repo's probe with the website-design skill's craft checks (broken words, font fallback, collapsed items, doc basics, type/color sprawl, section repetition) rather than running two probes, and taught it to report 404 URLs instead of a count, to break advisories down by control and by width, and to measure the EFFECTIVE tap target (a checkbox inside its own label is judged by the label).
- The composer was the worst surface: "Post Now" could never publish (it created a draft and the publish endpoint refused drafts); clearing a field did not clear it; the rich-text editor's HTML was stored and published verbatim; every Reddit and Pinterest post failed at publish because the composer sent `{integrationId: {target_ids}}` where the providers read flat provider-named keys; a photo with no caption could not be composed at all; the thread builder created duplicate threads from an edit screen; a thread's other parts were never saved.
- The calendar put posts on the wrong day: `toCalendarEvent` sliced the UTC timestamp instead of converting it into the user's zone, and the API's range end was midnight of the end date, so the last day of every month and week was empty. Both are fixed and the conversion is proven in the browser (03:30Z renders as 23:30 in America/New_York).
- The analytics surface counted the wrong things: the "vs previous period" delta was a count rendered with a `%`; the metric labels never matched X or Bluesky, so those users read 0 comments and 0 shares forever and their best posts were EXCLUDED from the research ranking; the 90-day window was a no-op because `external_posts.created_at` was declared INTEGER epoch but written as RFC 3339 TEXT, and SQLite orders every INTEGER before every TEXT; the Activity page CRASHED on "Load more" because the cursor was inclusive and the list is a keyed `{#each}`.
- The visual pass took 4056 tap-target advisories down to ~1000 by putting a 44px target on every control the probe named - the sidebar's thirteen links were 36px, the bell 32px, the dropdown trigger 16px, the feed's text buttons had no padding at all. The remaining advisories are the calendar's time-grid chips, whose size is the grid's design.
- Fixed 44px targets without moving a single glyph, and without changing the visual weight of any row: the target grows around the control, and where the control was a checkbox inside a label, the label carries the target and the box stays 16px.

Stage Summary:
- Artifacts: 12 commits on master, pushed to github/master (23f6e67). Frontend: lib/api/{posts,campaigns,integrations,automation,calendar,analytics}.ts, lib/{providers,calendar/types,calendar/ListView,calendar/CalendarHeader,calendar/MonthView,calendar/PostDetail,calendar/PostHoverToolbar,composer/ComposerModal,composer/platforms,composer/SchedulePicker,composer/wall-clock,composer/PerPlatformCharCount,ui/Dropdown,ui/TopBar,ui/EmptyState,notifications/NotificationBell}.svelte, routes/{+layout,posts,posts/new,calendar,campaigns,campaigns/[id],analytics,research,activity,feed,kanban,tags,settings/notifications}. Backend: api/{posts,campaigns,calendar,activity,analytics,notifications,feed,integrations}.rs, db/{queries,models,types}.rs, social/{mod,x}.rs, scheduler/mod.rs. Migrations: 0007_external_posts_epoch.sql. New tests: composer_contract_test.rs, external_posts_window_test.rs, counting_rule_test.rs, plus calendar_range_test.rs and kanban_stage_test.rs extended. Probe: frontend/scripts/probe.mjs gained 404 capture, per-control advisories and the skill's craft checks.
- Verification: 481 lib tests + 35 integration tests pass; `cargo clippy --lib --bins --all-features -- -D warnings` 0 errors; `svelte-check` 0 errors / 0 warnings; `pnpm build` clean; the probe over 13 routes x 3 widths x 2 themes reports 0 FAIL, 0 console errors, 0 404s. Every fix that could be reproduced was reproduced against the live API before and after (the draft publish, the Reddit settings shape, the inclusive calendar range, the campaign dates and metric, the caption-less post, the activity cursor, the emoji count).
- Known follow-ups: the remaining ~1000 tap-target advisories are the calendar's hour-grid chips (21px) and a handful of text buttons whose width is under 44; both are structural, and the month view's chips are already inside a 44px cell. The `live_*` test binaries still hit the real provider APIs and fail intermittently on rate limits - that is environmental, not a regression.

---
Task ID: red-team-web-dashboard-2026-10-19-continued
Agent: omp (same session, continued)
Task: The remaining findings from the four scout reports, and the second visual iteration.

Work Log:
- The composer was still the worst surface after the first pass. Fixed, in order of how badly they hurt: the provider settings shape (every Reddit/Pinterest post failed at publish); the update path dropping the per-channel body and settings; the thread builder creating duplicate threads from an edit screen; a thread's other parts never being saved; a photo with no caption being impossible to compose; the character count the composer showed (UTF-16 units) differing from the one the server enforced (UTF-8 bytes), and X's weighted rule existing only in the browser.
- The analytics surface had four more: the "vs previous period" delta was a count rendered with a `%`; the metric labels never matched X or Bluesky, so those users read 0 comments and 0 shares forever AND their best posts were excluded from the research ranking; the 90-day window was a no-op because `external_posts.created_at` was written as RFC 3339 TEXT into an INTEGER column (SQLite orders every INTEGER before every TEXT, so `created_at >= ?` was always true); the Activity page crashed on "Load more" because the cursor was inclusive and the list is a keyed `{#each}`.
- The campaign surface had four: the end date was a day early (a bare `YYYY-MM-DD` is a calendar date, and `new Date()` parses it as UTC midnight); the progress bar measured the post count against an engagement target; the bar clipped its own count at the rounded end; and emptying a field did not empty it (`|| undefined` is "omitted", and `COALESCE` cannot tell "keep" from "clear").
- A second sweep of the rendered pages found three more that only looking would catch: the channel-mix bar's number was clipped by its own bar exactly when the bar was widest; the research "best time to post" bars were all full width because there was no maximum to scale against; and the dashboard's "Channel Performance (30d)" card was fed a 7-day window.
- The tap-target pass went from 4056 advisories to under 800. The probe learned to measure the EFFECTIVE target (an input inside its own label is judged by the label) and the fixes learned to put the target on the LABEL, so a 16px checkbox in a dense grid still has a thumb-sized hit area.
- Every fix that could be reproduced was reproduced against the live API, before and after: the draft publish, the Reddit settings shape, the inclusive calendar range, the campaign dates and metric, the caption-less post, the activity cursor, the emoji count, the engagement sort, and the campaign field clearing.

Stage Summary:
- Artifacts: 6 more commits on master, pushed to github/master (3121a25). New: tests/counting_rule_test.rs (4 tests). Migrations: none beyond 0007. Frontend: lib/api/{campaigns,media,posts}.ts, lib/composer/{ComposerModal,MediaUpload,platforms}.svelte, lib/calendar/{MonthView,WeekView,DayView,ListView}.svelte, lib/{ui/Modal,media/MediaPopover,composer/MusicPicker,onboarding/GettingStarted}.svelte, routes/{+layout,+page,posts,posts/new,calendar,campaigns,campaigns/[id],analytics,research,activity,feed,kanban,tags,settings/developer,settings/notifications}. Backend: api/{posts,campaigns,activity,notifications,feed,integrations}.rs, db/{queries,models,types}.rs, social/{mod,x}.rs, scheduler/mod.rs, mcp/tools_campaigns.rs.
- Verification: 481 lib tests + 38 integration tests pass; `cargo clippy --lib --bins --all-features -- -D warnings` 0 errors; `svelte-check` 0 errors / 0 warnings; `pnpm build` clean; the probe over 13 routes x 3 widths x 2 themes reports 0 FAIL, 0 console errors, 0 404s.
- Known follow-ups: the remaining tap-target advisories are the calendar's hour-grid chips (21px, structural - the grid IS the design) and a few text buttons whose WIDTH is under 44. The `live_*` test binaries hit the real provider APIs and fail intermittently on rate limits; that is environmental.

---
Task ID: red-team-web-dashboard-2026-10-19-final
Agent: omp (same session, final verification)
Task: The last tap-target pass and the final gate.

Work Log:
- The last named controls carry a 44px target: the dashboard's header buttons, its alert links and its "Needs Attention" rows; the activity log's "View post" link and "Load more"; the posts list's search field and row buttons; the Getting Started checklist's rows; and every calendar view's select checkbox (the label carries the target, the box stays 16px).
- The advisory count is now 398, from 4056 at the start. What remains is the calendar's hour-grid chips (21px, and the grid IS the design), the tag pills (their width is their text), and a few inputs in narrow cells.
- One finding was NOT a defect: the "Update existing login" overlay in the 375px light-mode screenshot is the BROWSER's password-manager UI. The live DOM has no such element and no dialog role. Confirmed rather than assumed.

Stage Summary:
- Verification: 481 lib tests + 38 integration tests pass (519 total, 0 failed); `cargo clippy --lib --bins --all-features -- -D warnings` 0 errors; `svelte-check` 0 errors / 0 warnings; `pnpm build` clean; `scripts/genome doctor` healthy; the probe over 13 routes x 3 widths x 2 themes reports 0 FAIL, 0 console errors, 0 404s, and 398 advisories.
- Artifacts: 21 commits on master, all pushed to github/master (43d16c3).

---
Task ID: red-team-web-dashboard-2026-10-20-iteration-2
Agent: omp (same session, second red-team iteration)
Task: A second red-team over the code the first iteration changed, plus four new surfaces: the MCP tool surface, the scheduler/publisher path, the Svelte 5 rune usage, and the media pipeline.

Work Log:
- **The MCP surface was the lossy copy of the REST surface.** Every MCP tool that changes state re-implemented the write, and the re-implementation dropped the safety half: `posts_update` WIPED the post (the write was unconditional, so an agent that sent only a new body erased every image and every provider setting - the Reddit subreddit, the Pinterest board - and the post then failed at publish with "Missing subreddit in settings"); `campaign_update` could never CLEAR a field (a two-state input mapped with `.map(Some)` turned "clear this" into "leave this alone", and the tool reported success); `posts_create` skipped the provider validation, so an agent could post 600 characters to X, get "success", and the post sat in `queued` until it failed at publish time; `media_delete` deleted from the local directory only, so on an S3/R2 deployment the row went and the OBJECT STAYED IN THE BUCKET; `posts_set_tags` re-implemented the SQL without the ownership check or the transaction; the automation tools had NO ownership check at all; `posts_publish` fabricated a schedule and dropped the live permalink; and the media responses omitted `alt` entirely.
- **The media pipeline.** Eight of the twelve Tier-1 providers could not publish media AT ALL: the stored URL is relative (`/api/media/<uuid>`) and a platform cannot fetch that, and `resolve_media_url`'s default returned the attachment unchanged with each provider opting in - eight did not, so they published with a relative URL and failed with an opaque network error. The media library's search was 100% broken: the SQL's placeholder order was `user_id, pattern, limit, offset` and the binds were `user_id, limit, offset, pattern`, so the LIKE pattern received the integer LIMIT; past page 1 the OFFSET received the pattern string and SQLite raised a datatype mismatch, which the error handler turned into a 500. A library pick was classified as a VIDEO (the mime type was re-derived from a URL with no extension). The alt text was never persisted - the column existed, the upload hard-coded `None`, and there was no endpoint to write it, so it lived in the browser tab and was DESTROYED by the first save. A downward drag inserted at the wrong index. Threads' media limit was 10 in the server and 20 in the composer. The composer refused a media-only post. The composer checked images and videos separately while the server also caps the TOTAL.
- **The reactivity.** The "Posting Targets" panel NEVER appeared: `$state(new Map())` is not a reactive Map (Svelte 5's `proxy()` returns a Map unchanged), so `integrationTargets.set(...)` notified nobody and the template's `{#if integrationTargets.has(intId)}` was a derived with ZERO dependencies - evaluated once at mount, never recomputed. A Reddit subreddit, a Pinterest board or a Facebook Page could not be picked, and the post then failed at publish with "Missing subreddit in settings". The autosave's dependency list did not cover its payload, so attaching an image and tagging a post and then typing nothing meant the draft was NEVER written. A duplicate key in the suggested hashtags (the LLM repeats itself) CRASHED the composer - a keyed `{#each}` with a repeated key is a hard throw in Svelte.
- **The scheduler.** A post could be PUBLISHED TWICE: the claim's UPDATE had no `state = 'queued'` guard, and the transaction is a deferred `BEGIN` (the pool never configured BEGIN IMMEDIATE), so the SELECT ran with no lock held and two claims could read the same id list. The loser flipped the row the winner had already moved to 'publishing' and published it too. The fix is a compare-and-set with `RETURNING id`, so the claim returns exactly the rows IT flipped.
- **The failures left no trace.** A failed manual publish ("Post Now") fired the webhook but NEITHER broadcast nor persisted the event, so it was permanently absent from `events_log` - it never appeared in the activity log, never in the dashboard's "Recent Activity" widget, and never triggered the eight routes subscribed to `post_failed`. The three redundant standalone `send()` calls before `send_and_log` pushed every event to every SSE client TWICE.
- **The browser and the server disagreed on the post body.** The browser did not decode entities and did not drop `<script>`/`<style>` bodies, so `&mdash;` was literal text in the composer and an em-dash in the published post - and the CHARACTER COUNT is measured on the undecoded string.
- **Bluesky silently dropped images.** The `return Ok(...)` was INSIDE the upload loop, so only the FIRST image was ever embedded - with no error anywhere, while the server accepted four and the composer offered four.
- The post-text rules moved to `frontend/src/lib/composer/post-text.ts`, a dependency-free module, and the parity test now RUNS THE REAL TYPESCRIPT under node. The test used to compare two hand-written copies - which is a second copy of the rule, and a second copy is exactly how the drift survived. Removing the `hellip` case from the TypeScript now fails the test.
- Every fix that could be reproduced was reproduced: the double-publish claim, the entity drift, the MCP wipe, the alt-text 404, the Bluesky image drop, the media-only draft, the tap targets.

Stage Summary:
- Verification: 481 lib tests + the integration targets pass; `cargo clippy --all-targets --all-features --locked -- -D warnings` 0 errors; `svelte-check` 0 errors / 0 warnings; `pnpm build` clean; the probe over 13 routes x 3 widths x 2 themes reports 0 FAIL and 0 console errors.
- Artifacts: new tests/scheduler_claim_test.rs (3 tests, one of which FAILS without the claim guard - verified by removing it) and the node-driven parity test in tests/html_to_text_test.rs. New: frontend/src/lib/composer/post-text.ts.
- Known follow-ups: the remaining tap-target advisories are the calendar's hour-grid chips (21px, and the grid IS the design), the tag pills (their width is their text), and a few inputs in narrow cells.

---
Task ID: red-team-web-dashboard-2026-10-20-final
Agent: omp (same session, final gate)
Task: The last visual pass, the last advisories, and the ship gate.

Work Log:
- The probe's FONT check was wrong, and it had been reporting a failure on every page. A webfont arrives as many faces - one per weight AND per unicode subset - and the browser loads a face only when some text needs it, so a family that IS working still has faces sitting at `unloaded`. The check compared the family against the set of LOADED faces, so Inter (whose latin face was loaded and whose cyrillic face was not) read as "never loaded". Verified in a real browser FIRST: the latin face was `loaded`, `document.fonts.check('12px Inter')` was true, and the woff2 request returned 200. A family is loaded when AT LEAST ONE of its faces is.
- "No default signature" is an empty result, not a missing resource. It was a 404, and the composer calls that endpoint on every mount - so every user without a signature saw a 404 in their console on every page. It returns `null` with a 200 now.
- The last tap-target advisories were the buttons that were ONE line short of 44px: `px-4 py-2` around a 20px line gives 36px, `px-3 py-1.5` gives 34px. The fix is systematic rather than per-control: every button and every button-shaped link carries `min-h-11`. The inline TEXT links were left alone - "View all (8)", "Full analytics", a Telegram username - because a 44px box around a sentence inside a row pushes the row apart, and they are sentences, not controls. 142 -> 126 advisories.
- The 375px pass: every one of the 13 routes has ZERO horizontal overflow, and the calendar's week grid is a 640px time grid inside a 335px scroll container with `overflow-x: auto` - which is the right pattern for a time grid on a phone, and was confirmed rather than assumed.

Stage Summary:
- Verification: 481 lib tests + 62 integration tests pass (0 failed); `cargo clippy --all-targets --all-features --locked -- -D warnings` 0 errors; `svelte-check` 0 errors / 0 warnings; `pnpm build` clean; the probe over 13 routes x 3 widths x 2 themes reports 0 FAIL, 0 console errors, 0 404s, and 126 advisories (from 4056 at the start of the first iteration).
- Artifacts: 6 commits on master, all pushed to github/master (4996f07). The tree is clean.
- Known follow-ups: the 126 remaining advisories are inline text links (sentences, not controls) and the calendar's hour-grid chips (21px - the grid IS the design). The `live_*` test binaries hit the real provider APIs and fail intermittently on rate limits; that is environmental.
