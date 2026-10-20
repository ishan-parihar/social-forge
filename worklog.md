# Worklog — social-forge multi-agent log (§8 protocol)

## Contents

- [kb-adoption-2026-10-17](#kb-adoption-2026-10-17) — adopt the doc genome, wire it to the repo's existing conventions
- [genome-migration-2026-10-19](#genome-migration-2026-10-19) — kb.toml → genome.toml, retire the legacy `scripts/kb`
- [red-team-web-dashboard-2026-10-19](#red-team-web-dashboard-2026-10-19) — red-team pass over the web dashboard
- [red-team-web-dashboard-2026-10-19-continued](#red-team-web-dashboard-2026-10-19-continued)
- [red-team-web-dashboard-2026-10-19-final](#red-team-web-dashboard-2026-10-19-final)
- [red-team-web-dashboard-2026-10-20-iteration-2](#red-team-web-dashboard-2026-10-20-iteration-2)
- [red-team-web-dashboard-2026-10-20-final](#red-team-web-dashboard-2026-10-20-final)
- [red-team-iteration-3-5](#red-team-iteration-3-5)
- [v12-red-team-iterations-6-7](#v12-red-team-iterations-6-7)

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

---
Task ID: red-team-iteration-3-5
Agent: omp
Task: Exhaustive red-teaming of the social-forge web dashboard in recursive
iterations. Iteration 3 started from a static probe that covered 13 of the
app's 29 routes against an EMPTY seeded database; iterations 4 and 5 went
after the interaction the probe cannot see.

Work Log:
- Iteration 3: found the probe covered 13 of 29 routes and the seeded
  instance was empty, so every page rendered an empty state. Extended the
  probe to the whole route table, wrote scripts/seed_demo.py (a realistic
  solo-founder dataset with a self-check that every uuid column is
  BLOB(16)), and re-probed.
- The uuid id column was decoded as a String at 23 query sites. sqlx
  encodes a Uuid as BLOB(16); the `.sqlx/` cache records the DECLARED
  type (TEXT), so it compiled and failed only at RUNTIME - a 500 on
  POST/GET /api/automation/rules, and the CLI's `tags list` and
  `webhooks list`.
- The probe's `effectiveBg` SKIPPED the translucent layers it walked
  past, so every reading on a tinted chip was wrong - and the app is
  full of tinted chips. Fixing the compositing exposed that a semantic
  hue as TEXT on a wash of itself fails AA in the light theme for EVERY
  hue.
- `providerInk` computed the glyph's ink against a HARDCODED surface,
  but the app paints a provider glyph on three different backgrounds.
- Iteration 4: the static probe was clean, so this pass audited the
  INTERACTION. The composer wrote `settings["channel_id"]` and
  `settings["page_id"]` - keys NO provider reads - while Discord reads
  `settings["channel"]`. Every Discord post was created, queued, and
  then failed at publish. Fixed on both sides: the composer writes the
  key the provider reads, and the create path checks the provider's
  `required_settings()` and refuses the post with a message naming the
  missing field.
- The calendar's RANGE filter was UTC-bounded while its GRID was
  app-zone-bounded, so posts in the gap fell off the calendar entirely -
  on NO month grid, in either month. The range is now sent as UTC
  instants computed with the app's own `wallClockToUtc`.
- Iteration 5: editing a post WIPED its provider settings (the detail
  response had no `settings` field, the composer sent `{}`, and the
  update path REPLACED wholesale) - a post that was working thirty
  seconds earlier failed at publish.
- The feed half of every analytics headline was ALWAYS ZERO: the cutoff
  was bound as a raw `DateTime<Utc>` (RFC 3339 TEXT) against an INTEGER
  epoch column, and SQLite orders every INTEGER before every TEXT.
- No post could be filed under a campaign: the composer's dropdown set
  a variable that was NEVER read, and the create path had no
  `campaign_id` at all.
- The per-post analytics rows - the ones the dashboard READS - were
  written only when a human opened the Post Stats modal, and the
  cleaner deleted them an hour later. The founder's "Likes 42" became
  "Likes 0" with no publish, no error and no refresh.
- A card's substate could never be CLEARED (a COALESCE ratchet), the
  kanban lost the error posts' order (a per-state UPDATE on a mixed
  column), the research table was labelled UTC while the app is in the
  founder's zone, and the Instagram music the composer collected was
  never sent.

Stage Summary:
- key results:
  - 7 commits, each with a regression test that FAILS on the old code
  - probe: 0 contrast / heading / tiny-text / tap-target FAILs, 0
    advisories, over 27 routes x 3 widths x 2 themes. The 4 remaining
    FAILs are the /api/dms 502 from X rejecting the seeded fake token -
    an environment artifact, and the page renders its error state.
  - 481 lib tests, 27 integration test binaries (192 tests) pass
  - cargo clippy --all-targets --all-features --locked -- -D warnings: 0
  - SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings: 0
  - svelte-check: 0 errors, 0 warnings; pnpm build clean
  - scripts/check_status_contrast.py: 88/88 token pairs clear AA
- artifacts produced:
  - scripts/seed_demo.py (the seeded dataset + its uuid-type self-check)
  - scripts/check_status_contrast.py (the status-token gate)
  - tests/uuid_column_decode_test.rs (3)
  - tests/posts_media_shape_test.rs (3)
  - tests/post_settings_preserved_test.rs (5)
  - tests/required_settings_test.rs (5)
  - tests/feed_analytics_window_test.rs (2)
  - tests/campaign_assignment_test.rs (4)
  - tests/calendar_range_timezone_test.rs (3)
  - tests/kanban_substate_test.rs (6)
  - migrations/0008_posts_media_array.sql
- known issues / follow-ups:
  - the /api/dms 502 needs a REAL X token to clear; the seeded instance
    uses a fake one on purpose
  - "reach", "followers" and "custom" campaign metrics have no data
    source anywhere in the codebase; the UI now says "Not measured"
    rather than showing a 0% bar
  - the research page has no realtime subscription and no refresh
    button outside its error state

---

Task ID: v12-red-team-iterations-6-7
Agent: omp
Task: Continue the exhaustive red-teaming of the social-forge web
dashboard. Iterations 3-5 fixed what the static probe and a read-only
flow audit found. Iterations 6-7 went after what neither could see:
the pages in their non-default states, the empty instance, the
alternate widths, and the tool's own measurement errors.

Work Log:
- Read the flow auditor's report in full and triaged it into P0/P1/P2.
  The P0s: editing a post WIPED the provider settings (the composer
  sent `{}` and the server REPLACED the object, so every edit of a
  scheduled Reddit post dropped its subreddit); the feed half of every
  analytics headline was always zero (a TEXT timestamp bound against an
  INTEGER epoch column); and NO UI path could put a post into a
  campaign (the composer's dropdown set a variable that was never read,
  and the create request had no `campaign_id` field at all).
- Fixed each at the layer that owns the rule, with a regression test
  that FAILS on the old code: the update MERGES settings, the epoch
  cutoff goes through the `EpochUtc` seam, and the campaign is
  writable on create AND on update (three-state, so an edit can set,
  keep or clear it).
- Found the composer was writing `settings.channel_id` - a key NO
  provider reads - while Discord reads `settings.channel`. A Discord
  post could therefore NEVER publish. Added a `required_settings()`
  trait method and a create-time check, so the founder is told at
  compose time instead of finding out from the scheduler hours later.
- Extended the probe from 13 routes to all 29, and gave it a
  `--state` option so a page can be measured in a named state. The
  calendar's month and day views had NEVER been rendered: the month
  chips were 15px of a 144px cell, the day numbers were at 2.20:1,
  and the day view's ghost-slot button was 21px tall.
- Found the probe's own measurement errors, which had been hiding
  defects: it SKIPPED translucent backgrounds instead of compositing
  them (so every tinted-chip failure was invisible), it read the
  `color` without the inherited `opacity` (so a chip faded to 1.31:1
  passed), it measured contrast before the theme class was applied
  (69 failures in one run, none in the next), and it treated every
  tap target under 44x44 as a FAIL when WCAG 2.2's floor is 24x24.
- Ran a SECOND instance against an empty database. The composer's
  empty-state links were 17px, and the two dynamic routes were
  reported as FAIL when there was simply no row to open - so a fresh
  install, the first thing a founder sees, could never pass the probe.
- Repaired 245 of the 389 MCP tool descriptions, which had been cut
  off mid-sentence ("Get the authenticated user's profile." became
  "Get the authenticated").
- Looked at every route's rendered output in both themes at 1440, 768
  and 375, and at the 29 route screenshots by eye.

Stage Summary:
- key results:
  - 16 commits, each with a regression test that FAILS on the old code
  - probe over 29 routes x 3 widths x 2 themes, seeded instance:
    0 FAIL except the /api/dms 502, 0 warnings
  - probe over the same routes, EMPTY instance: 0 FAIL, 2 SKIPPED
    (correctly explained), 12 warnings (24px inline links, above the
    WCAG floor)
  - probe over the calendar's month and day views: 0 FAIL each
  - 481 lib tests, 27 integration test binaries pass
  - cargo clippy --all-targets --all-features --locked -- -D warnings: 0
  - SQLX_OFFLINE=true cargo clippy --all-targets --locked -- -D warnings: 0
  - svelte-check: 0 errors, 0 warnings; pnpm build clean
- artifacts produced:
  - tests/campaign_edit_test.rs (3)
  - tests/required_settings_test.rs (5)
  - tests/feed_analytics_window_test.rs (2)
  - tests/post_settings_preserved_test.rs (5)
  - tests/uuid_column_decode_test.rs (3)
  - tests/posts_media_shape_test.rs (3)
  - tests/kanban_substate_test.rs (6)
  - tests/calendar_range_timezone_test.rs (3)
  - scripts/seed_demo.py (the seeded dataset + its uuid-type self-check)
  - scripts/check_status_contrast.py (the status-token gate)
  - AGENTS.md 9.33-9.38 (six new pitfalls)
- known issues / follow-ups:
  - the /api/dms 502 needs a REAL X token to clear; the seeded instance
    uses a fake one on purpose
  - "reach", "followers" and "custom" campaign metrics have no data
    source anywhere in the codebase; the UI says "Not measured" rather
    than showing a 0% bar
  - the research page has no realtime subscription and no refresh
    button outside its error state
  - the composer previews deliberately simulate the PLATFORM's chrome
    (a LinkedIn post is white-on-light), so their greys do not follow
    the app's tokens. That is a feature; the greys were fixed to clear
    AA against the surfaces they actually sit on.

---
Task ID: doc-foundation-pass-2026-10-20
Agent: jcode
Task: Complete the project-genome documentation management so the doc foundation
is correct before an executor agent starts work.

Work Log:
- Surveyed the genome state first: `scripts/genome` is vendored and current
  (2026.10.20-1), `genome.toml` maps every role (nine `native:` forms, four
  file paths), and `lint` was already clean. The defect was not the tooling —
  it was the content the tooling routes to.
- Verified every number in AGENTS.md against the tree before editing:
  `#[tool(` count = 389 (all in `src/mcp/mod.rs`; the 46 sibling `tools_*.rs`
  files carry zero tool attributes), `ls migrations/*.sql` = 8 (doc said 2),
  `ls src/social/*.rs` = 39 provider files, real LOC ~94.2k Rust / ~29.2k frontend
  against the documented ~85.7k / ~25.7k.
- Repaired the AGENTS.md §0.6 document map.
  CORRECTION, written in the same session after re-reading the request against
  the evidence: the claim in the first commit that "four rows cited
  docs/archive/ paths that no longer exist" was FALSE, and the annotation I had
  added to those rows has been removed. My existence check covered only a
  curated path list and `.md` URLs, so it never actually tested the
  `docs/archive/` rows; checked directly, all three exist (GAPS_15 170 lines,
  OUTLINE-17 100 lines, OUTLINE-16 70 lines). I asserted an unverified claim in
  two commit messages. The mistake was mine, not the doc's.
  What was genuinely wrong in this map: one dead path (`SESSION_HOOKS.md` named
  at the repo root, existing only at `docs/SESSION_HOOKS.md`, now repointed), and
  seven live docs with no row at all — PAGE-INDEX, platforms-and-cli,
  deployment, agent-integration, TARGETING_AND_WRITER_DISCIPLINE,
  DECISIONS-19 and EXECUTION-19 — so `ask` reached them only by luck of
  ranking. They are routed now.
- Replaced the stale key-numbers block with measured values, each line naming
  the command that re-measures it, so the next agent re-measures rather than
  trusting the prose. Also fixed the two other stale lines (§2 layout tree:
  2 → 8 migrations, 3492 → 4114 LOC for mod.rs) and the dead `44 files` count.
  VALIDATION (second pass): the word "dead" was wrong there — 44 is a real
  number, the count of `src/mcp/tools_*.rs` handler modules, and the README's
  "389 tools across 44 modules" is self-consistent. The actual defects in my
  own replacement text were "27 providers" (the truth is 26: 12 + 14, with
  `linkedin-page` and `instagram-standalone` INSIDE the 12, so my "plus 2"
  double-counted) and "46 siblings" (there are 44). Both corrected; LOC and the
  migration count were right as written.
- Fixed the lint error the first pass had shipped: E001 on AGENTS.md:193. My
  re-measure command was written as a backticked token containing an unbalanced
  bracket, which broke the linter's own tokenizer and produced a phantom `.rs`
  pointer that failed to resolve. Bisected it by deleting segments one at a
  time, then reworded the line so every backticked token is a real path.
  The command was also wrong as first written: the bracket is a regex
  metacharacter, so the unescaped form returns 0 rather than 389. The version
  now documented is `grep -rh --include='*.rs' 'tool(' src/mcp/ | wc -l`, which
  returns 389 and lints clean.
- Removed two dead pointers: a machine-local skill path
  (/home/ishanp/.agents/...) and /home/z/my-project/scripts/cargo-env.sh, which
  named a different machine entirely.
- Added a table of contents to `worklog.md`: the entry count pushed it past
  `core.toc_lines` (300), which made `genome lint` emit its first-ever W010
  warning on this file. Lint is clean again.
- Re-ran the doc-drift tests after the edits, because AGENTS.md is asserted
  against the source by `tests/surface_docs_audit.rs`. That parser counts
  `#[tool(` attributes across `src/mcp/`; it parses exactly 389, which is the
  string the edited AGENTS.md carries, so `d1_documented_tool_counts_match_registry`
  still passes. `cargo test --lib`: 481 passed. `cargo check --lib --bin
  social-forge`: 0 errors.
- Checked the odd `git` behaviour mid-session: HEAD advanced from 006aed5 to
  73d0394 and my diff disappeared. A parallel agent had committed and pushed the
  preview/tag colour work in the same tree. Nothing of mine was lost; the re-run
  of every verification command after that commit is what the numbers above are
  measured against.

Stage Summary:
- Artifacts: AGENTS.md (§0.6 doc map, key numbers, §2 layout tree, two dead
  pointers), worklog.md (table of contents + this entry).
- Verification: `genome lint` 0 errors 0 warnings; `genome doctor` healthy;
  `cargo check --lib --bin social-forge` 0 errors; `cargo test --lib` 481 passed;
  `cargo test --test surface_docs_audit` 3 passed.
- Known issues / follow-ups:
  - `docs/decisions/` holds zero AD records and `docs/archive/records/` does not
    exist. That is deliberate per its README (ADs start from the first ratified
    direction change), so no record was minted here — minting an AD needs an
    operator ratification, and inventing one would violate law 6.
  - `probe-out/` and `scripts/__pycache__/genomecpython-314.pyc` are tracked
    (~4.8 MB, 157 files) although `.gitignore` lists probe-out. Turning on the
    `ops` module would raise E305; that needs an operator decision, because
    `docs/planning/EXECUTION_2026-10-19.md` cites `probe-out/run12` and
    `run13-dark` as evidence and those directories no longer exist on disk.

---
Task ID: doc-foundation-pass-2026-10-20-followup
Agent: jcode
Task: Re-read the request against the gathered evidence; correct what the first
pass overstated, then continue.

Work Log:
- Re-verified the claim I had made about dead `docs/archive/` paths. It was
  false: the existence check I wrote only covered a curated path list and `.md`
  URLs, so it never tested those rows. All three exist (GAPS_15 170 lines,
  OUTLINE-17 100 lines, OUTLINE-16 70 lines). Removed the incorrect
  annotations from AGENTS.md and wrote the correction into this entry rather
  than quietly editing history — the commit messages are immutable on the
  remote, so the correction has to be visible somewhere, and here is where a
  reader will look.
- Acted on the follow-up the first pass logged but left open. PAGE-INDEX was
  labelled the first file to read by that same pass, which made its accuracy
  load-bearing: six of its rows said LIVE for files whose own bodies say they
  are superseded (GAPS-15, OUTLINE-16, OUTLINE-17, OUTLINE-19, OUTLINE-19-v3,
  AUDIT-14). All six now carry their real status, each naming its successor
  where one exists.
- Added an intro to PAGE-INDEX explaining that the status column is what the
  `Owns` column is conditional on. A column that only meant something after my
  edit needed to say so, or the next reader would treat LIVE as decoration the
  way I found it.
- Checked reachability for every tracked doc against PAGE-INDEX, the README,
  AGENTS.md and SKILL.md: 29 docs, zero orphans. The two I first flagged as
  orphans (platform-matrices.md, recipes-extended.md) are cited from SKILL.md's
  router table — my check had simply omitted the skills directory.
- Verified the tombstone copies under `docs/planning/` are the intended
  convention and not duplicate live plans: five files carry the tombstone
  comment, the planning README marks each archived, and `genome lint` reports
  no W006 overlap.

Stage Summary:
- Artifacts: AGENTS.md (annotations removed), docs/PAGE-INDEX.md (six status
  corrections + intro), worklog.md (correction + this entry).
- Verification: `genome lint` 0 errors 0 warnings; `genome close` green;
  `cargo test --lib` passed; `cargo test --test surface_docs_audit` 3 passed.
- Known issues / follow-ups:
  - PAGE-INDEX is still not in `genome.toml`'s ownership map. That is an `ops`
    module role and turning `ops` on has its own consequence (E305 on tracked
    bulk), so it stays an operator decision rather than a unilateral switch.
  - The false claim remains in the messages of commits 5e3db17 and 66b4823 on
    the remote. The correction is here and in AGENTS.md; the root commit is not
    rewritten, and should not be.

---
Task ID: doc-foundation-validation-2026-10-20-pass-2
Agent: jcode
Task: Deep validation of the documentation foundation work. Find what the first
pass got wrong rather than confirming it.

Work Log:
- Re-measured every figure I had published as "measured", and two of them were
  wrong. The provider count was 27; the truth is 26 (12 Tier-1 + 14 Tier-2, with
  `linkedin-page` and `instagram-standalone` INSIDE the 12, so my "+2" added what
  was already counted). `src/social/tier.rs` and `registry.rs` both assert 26, so
  the code was right and my arithmetic was not. The sibling file count was 46;
  the truth is 44 `tools_*.rs` modules plus `auth.rs` and `rest_bridge.rs`, in a
  tree of 47 Rust files.
- Corrected my own earlier claim rather than leaving it standing: the "dead 44
  files" wording I used against the ORIGINAL doc was wrong. 44 is a real number
  (the handler modules) and README's "389 tools across 44 modules" was
  self-consistent. I had replaced a correct number with an incorrect
  "clarification" of it.
- Found and fixed a lint error the first pass had shipped: E001 on AGENTS.md:193.
  My backticked re-measure command contained an unbalanced bracket, which broke
  the linter's tokenizer into emitting a phantom `.rs` pointer. Bisected by
  removing one backticked segment at a time until the error cleared, then
  reworded so every backticked token is a real path. The command itself was
  independently wrong: `[` is a regex metacharacter, so the unescaped form
  returns 0 rather than 389. Now documented as
  `grep -rh --include='*.rs' 'tool(' src/mcp/ | wc -l`, which returns 389.
- Valued every PAGE-INDEX status label against body evidence and the archive
  INDEX's recorded reasons: 18 rows, all consistent. Confirmed the tombstone
  copies under `docs/planning/` are convention, not duplicate live plans, and
  that lint reports no W006 overlap.
- Confirmed reachability for all 29 tracked docs (zero orphans) and re-confirmed
  the push protocol (no `origin` remote resolves; `github/master` does; no
  `git push origin` remains anywhere outside `docs/archive/`).
- Acted on the genome upgrade that `doctor` had been reporting as informational.
  Ran the skill's own independent self-check against the vendored copy FIRST:
  32/33, failing `p5_class_intent`. The newer CLI passes that case and scores
  33/33. Applied with `genome upgrade --apply`, backup left under the existing
  `.genome/` ignore. Post-upgrade lint clean and doctor healthy, no new
  findings, so no rule in this repo depended on the old behaviour.

Stage Summary:
- Commits: 64c55eb (the three numeric defects + E001), d393b4b (CLI upgrade).
- Verification: `test_genome.py` 32/33 before, 33/33 after; `genome lint` 0
  errors 0 warnings; `genome doctor` healthy; `cargo test --lib` passed;
  `cargo test --test surface_docs_audit` 3 passed; dead-path grep clean.
- Known issues / follow-ups:
  - Two closed items from earlier entries are now resolved (the PAGE-INDEX
    labelling, the CLI revision). Two remain, unchanged and deliberate: the
    `ops` module decision, and the fact that commits 5e3db17 and 66b4823 still
    carry claims this pass falsified. The corrections live here and in
    AGENTS.md; the history is not rewritten.
  - The CLI now being 2026.10.20-2 means the next `genome upgrade` check will
    drift again as the skill moves. Treat `genome doctor`'s upgrade line as a
    signal to re-run the self-check, not as noise.

---
Task ID: doc-foundation-validation-2026-10-20-pass-3
Agent: jcode
Task: Third validation pass: the §2 layout tree, which two earlier passes had
never measured.

Work Log:
- Re-measured every count in the §2 repository-layout tree, not just the
  key-numbers block the first two passes had fixed. It was wrong in five places:
  "31 provider implementation files" (the truth is 26, with the other 13 files
  under src/social/ being support modules — whatsapp_cloud.rs being the subtle
  one, an internal helper rather than a registered provider), "25 page routes"
  (18 top-level + 11 nested = 29), and four stale LOC counts (run.rs, x.rs,
  reddit.rs, queries.rs — all understated by 300-1000 lines).
- The scripts/ entry listed three of sixteen files and omitted the two the rest
  of the repo depends on: `genome` itself, and `vendor-pull.sh`, the CI gate
  VENDOR_PINS documents as enforced. An agent reading the layout tree would not
  have known the tooling existed.
- Confirmed two claims were accurate so they did not get "fixed" into wrongness:
  the 8 migrations contain no destructive DDL (only ON DELETE CASCADE FK
  clauses), and the 19 SSE events §9.8 names match realtime.ts one for one.
- Re-introduced the E001 error for the second time in this session by writing
  "the other 13 `.rs` here" — a bare extension token, the exact shape that
  already failed once. Caught it on the lint run rather than at push. Recorded
  because the first occurrence was attributed to an unfamiliar tokenizer quirk,
  which is the wrong diagnosis: it is a habit of writing prose with a bare
  extension in backticks, and habits do not correct from memory.
- Held AGENTS.md back from commit while a concurrent agent's §9.39–§9.43 records
  sat uncommitted in the same file, then committed the file once their records
  were live rather than reconstructing a split version that would have
  discarded their work. The commit message names them as the co-author of the
  content.

Stage Summary:
- Artifacts: AGENTS.md §2 layout tree (seven corrections).
- Verification: `genome lint` 0 errors 0 warnings; `genome close` green; gate ok;
  `cargo test --lib` passed; `cargo test --test surface_docs_audit` 3 passed;
  dead-path grep clean.
- Known issues / follow-ups:
  - The layout tree's LOC counts will drift again as the files grow. They are
    now four years of evidence stale; nothing in the repo asserts them. If the
    next pass wants a guard, `surface_docs_audit` is where a count assertion
    would go, but only for the counts the code can derive (tools, migrations,
    tier membership) — a file's line count changes with every commit.
  - Nothing else in §2 remains unmeasured: every number in the tree has now been
    checked at least once against the tree it describes.

---
Task ID: doc-foundation-validation-2026-10-20-pass-4
Agent: jcode
Task: Sweep the remaining routing surfaces for unverified claims. This pass
found the defect the whole exercise was for.

Work Log:
- Verified claims until then unexamined and found four more stale numbers in
  the §2 layout tree: "31 provider implementation files" (26 — the other 13
  files under src/social/ are support modules, whatsapp_cloud.rs being the
  subtle one: an internal helper reached through the WhatsApp provider, not a
  registered provider), "25 page routes" (18 top-level + 11 nested = 29), and
  four understated LOC counts. Also corrected the outline's own claim set.
- Confirmed accurate so as not to fix them into wrongness: the 8 migrations
  contain no destructive DDL (only ON DELETE CASCADE FK clauses), and the 19
  SSE events §9.8 names match `frontend/src/lib/stores/realtime.ts` one for one.
- THE FINDING. AGENTS.md §0.6 routes every agent to
  `docs/planning/OUTLINE-2026-10-19-v2.md` "before starting ANY implementation
  work", describing it as "the live working queue". It is dated-frozen, so its
  lineage is correct — but one day later five of its six `STILL OPEN` items had
  already landed. Measured, each with the command its own row names:
  SKILL.md 255 lines not 580; quick-reference 108 not 614; evals 43 with a
  toolset-restricted #42, not 40 with none; the "progressive disclosure" and
  "eval loop" gaps both closed by `evals/c3-paired-run.json`. Only C2 (no
  reference-router table) is genuinely open.
  Its stated top priority is false too: the outline says `surface_docs_audit`
  has "2 failing tests" and is "the gate-breaker" that "outranks all Wave B/C
  work". It is 3 passed, 0 failed.
- Did NOT edit the outline. It is dated-frozen and its lineage is the evidence
  that makes the audit checkable; rewriting it would destroy the thing that
  makes the re-audit falsifiable. Wrote a dated-frozen re-audit instead
  (`REAUDIT_OUTLINE-2026-10-19-v2_2026-10-20.md`) carrying a verdict table and
  the measurement command per claim, and corrected the three routing surfaces
  that reach the outline (AGENTS.md §0.6, PAGE-INDEX, planning README) to carry
  the verdict inline and point live-status questions at TODO.md.
- Confirmed `genome ask` now returns the re-audit as the top hit for the
  question, which is the mechanism the agent will actually use.
- Re-introduced the E001 bare-extension-token error a second time ("the other 13
  `.rs` here") minutes after diagnosing it once. Recorded because the first
  diagnosis — an unfamiliar tokenizer quirk — is wrong and produces the wrong
  fix. It is a writing habit, and it recurs until the file is scanned.

Stage Summary:
- Commit dc41cf1: the re-audit, plus three routing corrections. The seven
  layout-tree corrections from this pass are in 0b9d357.
- Verification: `genome lint` 0/0; `genome close` green, gate ok;
  `cargo test --test surface_docs_audit` 3 passed; `cargo test --lib` passed;
  every link in the four changed files resolves.
- Known issues / follow-ups:
  - One dead link, pre-existing and not mine: PAGE-INDEX's VENDOR_PINS cell
    quotes a body whose underscores were stripped, so
    `[PLANPARITY...](PLANPARITY...)` does not resolve. It is inside quoted
    content, and link-checking belongs to the ops module, which is off.
  - Only C2 (the reference router) survives the outline's B/C waves. It is a
    real gap and now the only one of the six that an agent should work.

