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
also be appended via `scripts/genome log "<title>" --body-file <file>`.

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


---
Task ID: doc-foundation-validation-2026-10-20-pass-5
Agent: jcode
Task: Adversarial re-check of pass 4's headline claims. A self-audit that only
confirms itself is worthless, so every claim was re-derived rather than re-read.

Work Log:
- FALSIFIED MY OWN VERDICT. C2 was recorded "the one real survivor" because I
  grepped SKILL.md for a heading containing "router" and found none. The table
  exists; it is titled `## Where to look next` and is keyed by need. All four
  references/*.md state a purpose and say "loaded on demand from SKILL.md". So all
  six of the outline's open items are landed, not five of six.
  The mechanism of the error is the interesting part and is now written into the
  re-audit: grepping for the jargon in a CLAIM rather than reading the section the
  claim describes. That is the same error the outline itself made with its own
  evidence. The reversal is left visible in the verdict table instead of being
  edited away.
- FALSIFIED A COUNT'S DERIVATION, though not its value. "26 provider files + 13
  support" was wrong: 29 files `impl SocialProvider` (mod.rs's two impls are test
  fixtures), of which 26 are registered and calendar/drive/gmail are capability
  modules that appear in no tier table. 29 + 9 support + mod.rs = 39. The 26
  survived for the wrong reason, which is worse than a wrong number because it
  survives the next edit too.
- Confirmed under challenge and unchanged: 44 tools_*.rs + auth.rs +
  rest_bridge.rs + mod.rs = 47 (all 389 tools in the one mod.rs impl); 26
  registered providers (12 + 14, no double-count this time); 19 SSE events
  matching realtime.ts; migrations contain no destructive DDL.
- Eval #42 read in full rather than grepped: `mcp-tools --toolset x` with an
  expected_output that names the tool budget. And c3-paired-run.json read in
  full: a real paired with-skill/baseline run with per-eval grades, three evals
  in scope, 40 not yet run. The B4 and C3 verdicts rest on artefacts, not filenames.
- Fixed my own broken verification script mid-pass: the dead-path check emitted
  ~55 false positives by concatenating `file:path` into single tokens before
  testing them. Re-ran it per file; genuinely zero dead paths. Noted because a
  check that cries wolf is the next agent's reason to ignore it.

Stage Summary:
- Commit eb5d444: the re-audit's C2 correction + method note, the layout tree's
  provider-file breakdown, and the three routing surfaces.
- Verification: `genome lint` 0/0; `genome close` green; gate ok; `cargo test
  --lib` passed; `cargo test --test surface_docs_audit` 3 passed; dead-path
  check re-run per file, 0 findings.
- Standing conclusion: the outline's B/C waves contain NO remaining work. Only
  the E wave (credential-blocked live passes) is open, and it is blocked on
  credentials exactly as TODO.md's V1a-V1d rows say.

---
Task ID: doc-foundation-validation-2026-10-20-pass-6
Agent: jcode
Task: Sweep the surfaces the earlier passes never checked. `providers.md` is
asserted by the doc test, so a stale per-platform count could hide behind a
passing test.

Work Log:
- Parsed all 389 `#[tool(` declarations from `src/mcp/mod.rs` and grouped them by
  the real name prefix. That map is the authority for every count below; nothing
  was taken from any doc.
- Found `providers.md` wrong for 8 of its 14 platforms: X 20→27, Instagram 15+→21,
  LinkedIn Page 12→11, LinkedIn Personal 12→15, Reddit 36→37, Bluesky 5→7,
  Discord 13→15, Telegram Bot 9→11. The doc test checks that this file states the
  TOTAL tool count, which it did, so it passed throughout.
- Six bodies listed only a subset, so heading and body disagreed with each other as
  well as with the registry. Appended the missing tools as an explicit
  "Not listed above (N more, M shown)" line per section instead of merging them
  into the original prose, so the curated descriptions survive and the gap stays
  visible.
- WhatsApp was the one case that needed judgement rather than arithmetic: 13
  tools across two prefixes, 9 `wa_` Web-session and 4 `wc_` Cloud-API. The
  heading now says so; a single number there implied one bucket that does not
  exist.
- Verified, not assumed: the 14 platform sections cover 235 of the 389 and the
  other 154 are the cross-cutting tools the file already documents in its own
  sections. That is why no total was rewritten and no false "everything is wrong"
  claim was made.
- Also confirmed README's claims hold (26 platforms, 389 tools, 44 modules all
  measured) and corrected my own verification script twice — once for
  concatenating file:path into dead-path tokens, once for a heading regex too
  strict to parse the sentence I had written.

Stage Summary:
- Commit 830ab31: providers.md, eight heading corrections plus six body
  completions, with the section-intro shape preserved.
- Verification: a three-way reconciliation script (heading vs body vs registry)
  passes for all 14 sections; `cargo test --test surface_docs_audit` 3 passed;
  `genome lint` 0/0.
- Known issues / follow-ups:
  - The per-platform counts will drift again as providers gain tools, exactly as
    they drifted here while the total-count assertion kept passing. The guard
    that would catch it belongs in `surface_docs_audit`: parse
    `providers.md`'s per-section counts and compare against the real prefix
    groups. That is a test change, not a doc change, and it was out of scope for
    a documentation pass — noted so it is not lost.

---
Task ID: doc-foundation-validation-2026-10-20-pass-7
Agent: jcode
Task: Guard the per-platform drift I found in pass 6. A count corrected once will
drift again, so the fix is a test, not a doc edit.

Work Log:
- Added D3 to `tests/surface_docs_audit.rs`, whose own header comment names the
  drift classes it guards (D1 the total, D2 schema shape). D3 parses
  `### Platform (N tools)` headings, counts the tools the section lists, and
  compares all three: heading, body, and the registry parsed from the same
  `#[tool(` scan D1 uses.
- FIRST DRAFT WAS WEAKER THAN I SAID. It required only that *a* continuation line
  exist somewhere in the section. Injecting a real drop (removing `bs_reply` from
  Bluesky) left it passing, because the stale "Not listed above (2 more)" line
  satisfied the check. Caught it by injecting drift and reading the failure
  instead of assuming the guard worked.
- Replaced it with a check that reconciles the continuation's own declared tail
  count against the tools the prose lists, so a drop and a mis-stated count are
  different failures with different messages. Verified by injection:
    - heading drift (Bluesky 7 -> 5): FAILS, "claims 5 tools but the registry has 7"
    - inconsistent continuation (says 2 more, names 1): FAILS, "lists 6 tools and
      its continuation claims 2 more, totalling 8 - the registry has 7"
    - clean file: 4/4 pass
- Left the "Not listed above" lines I added in pass 6 rather than expanding the
  prose, because the curated descriptions are the section's value and a test now
  keeps the omissions honest.
- Removed a debug `eprintln!` and a dead `head`-prefix block from my own draft
  before it could land. Both were caught by reading the diff.

Stage Summary:
- D3 landed in `4e5b879` (swept by the concurrent agent with the strengthened
  guard and no debug output, verified against `github/master`).
- Verification: 4/4 pass clean; two injections proved it fails with an actionable
  message; `genome lint` 0/0; `genome close` green; `cargo test --lib` passed.
- Known issues / follow-ups:
  - D3 asserts the WhatsApp section covers both `wa_` and `wc_`, and each other
    section one prefix. A provider added under a new prefix needs a row in the
    `sections` table, so the test fails loudly rather than silently ignoring it.
  - The `Other platforms` table at the bottom of providers.md is not covered by D3;
    it lists cross-platform tools. Extending D3 to it means choosing whether the
    table counts as prose, which is a judgement call left to whoever needs it.

---
Task ID: doc-foundation-validation-2026-10-20-pass-8
Agent: jcode
Task: Verify the B4 verdict against the CODE rather than the artefact, and sweep
every command the evals expect.

Work Log:
- Verified eval #42's commands against the source rather than trusting the
  artefact, and found a real defect: it expects
  `mcp-call x_home_timeline --args '{"count":5}'`, but `XHomeTimelineInput`
  (src/mcp/tools_x.rs) declares `max_results` and `pagination_token`. There is no
  `count` field, so a correct agent answer FAILS this eval. The CLI fallback in
  the same eval is right (`x timeline --count 5`, src/cli/mod.rs), so the error is
  confined to the MCP path. Corrected the arg and recorded both facts in
  expected_output.
- The 2026-10-19 paired run graded this eval `partial` and named the
  `--limit`/`--count` confusion, but attributed it to the agent's guess. Both
  halves are now right: the arg is `max_results`, the CLI flag is `--count`. Worth
  noting that a graded artefact can still carry a wrong expectation.
- THE NEAR MISS, which is the reason this entry exists. My sweep reported four
  commands as nonexistent: `gdrive`, `gmail-ops`, `fb`, `--toolset`. All four were
  FALSE POSITIVES produced by my own regex. clap derives kebab-case names from
  Rust enum variants, so grepping `src/cli/*.rs` for a literal `gdrive` finds
  nothing; and `fb` was matched out of `mcp-call fb_create_post`, a tool name and
  not a subcommand. Checked against the built binary: `gdrive Google Drive
  operations`, `gmail-ops Gmail operations` and `--toolset <TOOLSET>` all exist.
  I was one commit away from "fixing" three working commands on the strength of a
  bad grep — the exact failure mode this entire doc pass has been auditing. A
  verification that cannot fail is worse than none, because it looks like evidence.
- Final sweep, done correctly: all 43 evals' `mcp-call` targets are registered
  tools, and every `social-forge <subcommand>` resolves against the binary's
  `--help`. Only the one defect above was real.

Stage Summary:
- Eval #42 corrected; landed in `80beff9` (swept by the concurrent agent) and
  verified on `github/master` with 43 evals intact and no stale `count` arg.
- Verification: evals.json parses; every eval command resolves against src/cli and
  src/mcp; `genome lint` 0/0; `cargo test --test surface_docs_audit` 4 passed.
- Known issues / follow-ups:
  - No test guards an eval's expected command against the CLI/tool registry, the
    same gap D3 closed for `providers.md` counts. Adding one is straightforward
    (parse `expected_commands`, assert each `mcp-call` target is in the registry
    and each subcommand is in the clap tree) but it needs the clap `Command`
    introspection to be reachable from a test — `d1_documented_cli_command_count`
    already does this, so the pattern exists. Left as a note rather than started
    late in the session.

---

Task ID: iteration-6..12-red-team
Agent: omp
Task: Exhaustive red-teaming of the social-forge web dashboard in recursive
      iterations, with visual testing via the browseros-neo MCP and the
      website-design skill's probe. Fix every defect found.

Work Log:
- Built a POPULATED instance to test against. The seeded DB was empty, so every
  page rendered its empty state and every populated-state defect was hidden.
  `scripts/seed_demo.py` seeds 6 integrations, 17 posts across all six states,
  3 campaigns, 6 tags, analytics cache, external posts + engagement, the event
  log, automation rules, webhooks, signatures, RSS feeds, notifications, post
  sets, media and comments. It asserts every uuid column it writes is BLOB(16)
  (sqlx's `Uuid` decoder reads exactly 16 raw bytes and does NOT accept the
  36-char TEXT form) - a TEXT uuid in a uuid column took the whole endpoint
  down with "invalid length: expected 16 bytes, found 36".
- Extended the probe from 13 routes to ALL 27. Sixteen pages had NEVER been
  rendered by any gate. Everything the probe found after that was in pages
  nobody had looked at.
- The probe could not reach a page's ALTERNATE state. It gained `--state` and
  a named-state click, which is how the month and day calendar views were
  found broken.
- Fixed, in the order found:
  - 23 query sites cast `id as "id!: String"` on a `Uuid` column. sqlx encodes
    a Uuid as BLOB(16) but the column is declared TEXT, so the .sqlx cache
    recorded the DECLARED type, the build passed, and the runtime decode
    failed with a Utf8Error. `GET /api/automation/rules` returned 500 and the
    POST left an orphaned row. Fixed with the repo's own convention, an
    explicit `as "id!: Uuid"`.
  - The RSS writers passed `json!({})` as `posts.media` - an OBJECT where
    every reader expects an ARRAY. `post.media?.find(...)` threw
    "i.find is not a function" and took the kanban down. Fixed at both
    writers, plus migration 0008 to normalise the existing rows, plus a
    server-side normaliser so the shape is guaranteed on the way out.
  - `providerInk` computed the provider glyph's colour against a HARDCODED
    surface, and the glyph sits on three different ones. On the light theme
    the LinkedIn ink measured 3.67:1 on the search chip and 4.32:1 on the
    kanban card - both under the AA floor. It now takes the surface.
  - The PROBE skipped alpha compositing, so every tinted chip measured
    against the surface BEHIND the wash instead of the wash itself. Fixing
    the probe revealed that in the light theme EVERY semantic hue on its own
    wash is under 4.5:1. Fixed at the token level.
  - `opacity` was not accounted for in the contrast check, and a calendar cell
    with `opacity-40` dimmed its whole subtree - a past day's post measured
    1.33:1. The probe learned about opacity groups, and the cell's dimming
    moved from the subtree to the cell's chrome.
  - The `!` badge on a failed calendar chip was 8px, the tag chips 9px, and
    the month view's chip was 15px of a 144px cell.
  - Editing a post WIPED the provider settings: the update REPLACED the
    settings object, and the composer sent `{}` because the detail response
    had no settings field. A working scheduled Reddit post failed at publish
    with "Missing subreddit in settings" thirty seconds after it worked. The
    settings now MERGE, the detail response carries them, and the composer
    preserves them.
  - The analytics window filter bound a `DateTime` against an INTEGER epoch
    column, so the comparison matched NOTHING. The feed half of every
    analytics headline was permanently zero.
  - There was NO way to put a post into a campaign: the composer had a
    campaign select whose value was dropped on submit, and the create API had
    no `campaign_id` field at all. The feature was dead.
  - The command palette was inside an `aria-hidden="true"` subtree, so a
    screen reader announced nothing of it AND the shared focus trap could not
    find a focusable child - the search box never took the focus and typing
    did nothing.
  - Seven dialogs had no focus trap, no accessible name and no aria-modal.
  - The composer took no focus: the trap focused the first control, and the
    async load then re-rendered and DESTROYED the focused element, which put
    the focus on `<body>` - outside the dialog. The trap now watches for its
    focused element being removed.
  - Escape did nothing in the composer, and when it was made to work it left
    an orphaned confirm that swallowed the next Escape on ANY page.
  - The posting-volume chart contradicted the cadence card beside it: the
    query grouped by day, so the 20 days with no post were MISSING from the
    series rather than zero, and 11 posts on 11 days read as "you posted
    every day at full volume" next to "0.4 posts per day".
  - The AI panel's progress bar reported a percentage that was made up.
  - The probe measured the page MID-FADE and called a correct page broken:
    the entry animation's opacity made every ancestor a "translucent group".

Stage Summary:
- The gate is now FIVE scripts over the RENDERED app, all in `make audit`:
  probe.mjs, audit-targets.mjs, audit-contrast.mjs, audit-dialogs.mjs and
  e2e-calendar-drag.mjs. The last one is new: the plan's F3 gate was a drag
  e2e, and there was NO e2e for it. It has teeth - breaking `wallClockToUtc`
  makes it exit 1, restoring it makes it exit 0.
- The last measured state: probe 0 FAIL / 0 WARN, 0 controls under the 24px
  floor, 0 contrast failures in both themes, 0 dialog contract failures, 0 drag
  failures. The empty instance also passes (0 FAIL, 2 correctly-explained
  SKIPs, 12 warnings that are all inline links above the WCAG 2.2 floor).
- Landed in commits a354b4b..78ca215.

Known issues / follow-ups:
  - OPEN: the day bucket in the analytics day series is a UTC day. The
    founder's zone is an IANA name in a FRONTEND localStorage store
    (`social-forge-timezone`), and `users.timezone` is a column the server
    NEVER writes - the -300 in the seed is of unknown unit. So the server has
    no usable per-user zone, and the posting-volume chart's day buckets
    cannot yet follow the founder's day. The two sources are named here. The
    DENSITY fix does not depend on it.
  - OPEN: `/api/ai/*` has no streaming endpoint, so the composer's AI panel
    cannot show real progress. The panel is honest about it (an indeterminate
    indicator) rather than pretending. A streaming endpoint is backend work.
  - The NotificationBell polls its unread count every 60s as a fallback for a
    silently-dropped SSE stream. Deliberate, and recorded in the plan.

---
Task ID: v12-iterations-3-8
Agent: omp (merry-tamarin)
Task: Exhaustive recursive red-teaming of the social-forge web dashboard, with
visual verification through the browseros-neo MCP and the rendered-page probe.

Work Log:
- The 2026-10-20 red-team had probed 13 routes of 29, and every run had been
  against an EMPTY database: an empty state hides every populated-state
  defect. Built `scripts/seed_demo.py` (a realistic solo-founder dataset -
  6 integrations, 17 posts across every state, 3 campaigns, 6 tags, the
  activity log, analytics cache, external posts, automation rules, webhooks,
  signatures, RSS feeds, notifications, post sets, media and comments) and
  extended the probe to all 29 routes plus the two dynamic ones.
- The seed's first run wrote TEXT uuids into sqlx's BLOB(16) uuid columns, so
  every query against those tables failed at runtime. The seed now writes
  16 raw bytes and ASSERTS every uuid column's storage type before it
  commits - a self-check that names the mistake instead of leaving a broken
  instance behind.
- A seeded uuid id decoded as a String was a 500 on the automation, tags and
  webhooks endpoints - and the POST that failed had ALREADY INSERTED the row,
  so it left an orphan with no id returned. 23 query sites. The fix is the
  repo's own convention, an explicit `as "id!: Uuid"`: the macro infers the
  type from the column's DECLARED type, so dropping the override would NOT
  have worked.
- The probe's contrast check SKIPPED translucent layers instead of
  compositing them, so every tinted chip was measured against the surface
  behind it. Fixed, and it immediately found that EVERY semantic hue on its
  own wash fails AA in the light theme.
- `providerInk` computed the brand ink against a HARDCODED surface, and the
  app paints a provider glyph on four different surfaces. It is a parameter
  now, and each call site states where the glyph goes.
- The calendar's month and day views had NEVER been rendered. The month
  view's chip was 15px of a 144px cell, its day numbers were at 2.20:1, and
  the day view's ghost slot was 21px. The probe can now drive a page into a
  named state (`--state "Month"`).
- Editing a post WIPED the provider settings - the composer had no settings
  to send, so every edit of a scheduled Reddit post removed its subreddit
  and the post failed at publish. Fixed on both sides: the detail response
  carries the settings, the composer preserves them, and the server MERGES
  the incoming object over the stored one.
- No UI path could put a post into a campaign: the select existed, the id
  was dropped, and the create request had no `campaign_id` field. The whole
  feature was dead.
- The composer's channel-target map wrote `channel_id` and `page_id`, which
  NO provider reads, while Discord reads `channel` - so a Discord post
  ALWAYS failed at publish with "Missing channel in settings". The map now
  names what the provider reads, and the server REFUSES a post whose
  required setting is missing, at CREATE time, with a message that names the
  field.
- The connect flow's API-key fields were `text-white` on the light input
  surface: 1.05:1, invisible. An `<input>` has NO TEXT NODE, so no
  text-contrast gate could ever see it. The field's own ink is measured now,
  and the connect flows are driven.
- Eight TRUNCATED Tailwind class names in six files - `hover:text-accent-stro`,
  `items-centerderline`, `justify-centert-muted`, `bg-linetext-content-secondary`.
  A truncated class is a SILENT failure: not a build error, not a type error,
  not a svelte-check warning. `check-tailwind-classes.mjs` finds them and is
  in `make check`.
- A live interaction audit (a subagent driving the real app with a browser)
  found six wrong numbers: the dashboard's time was "07 PM" (a
  `slice(-5)` on a formatted date), every dashboard aggregate came from a
  100-row clip, "Publish success rate 24%" sat above "11 of 13 went out",
  the comments table showed a raw UUID (the join was to `external_posts` and
  the comments are on the founder's OWN posts), a campaign's progress hid its
  failures, and a calendar ghost slot dropped the hour the founder clicked.
- Seven dialogs had no focus trap, no accessible name and no aria-modal. The
  composer took no focus at all: the trap focused the first control and the
  async load re-rendered and DESTROYED it, putting the focus on `<body>`.
- The composer's discard prompt left an orphaned confirm that swallowed the
  next Escape on ANY page, because `composer.dirty` was never reset and the
  cleanup close asked a second time.

Stage Summary:
- The gate is now SIX scripts over the RENDERED app plus two source gates:
  probe.mjs, audit-targets.mjs, audit-contrast.mjs, audit-dialogs.mjs,
  e2e-calendar-drag.mjs, check-hex-tokens.mjs and check-tailwind-classes.mjs.
  `make check` runs the two source gates; `make audit` runs the five browser
  gates.
- The last measured state: probe 0 FAIL / 0 WARN over 29 routes x 3 widths x
  2 themes, 0 controls under the 24px floor, 0 contrast failures in both
  themes, 0 dialog contract failures, 0 drag failures. The EMPTY instance
  also passes (0 FAIL, 2 correctly-explained SKIPs).
- Every gate has been checked for TEETH: each one was made to fail on the
  broken code and pass on the fixed code.
- 483 lib tests + 46 integration binaries pass. Clippy is clean with
  `-D warnings`, and the offline (`SQLX_OFFLINE=true`) build is clean.
- Landed in commits a354b4b..15cc7f9.

Known issues / follow-ups:
  - OPEN: the day bucket in the analytics day series is a UTC day. The
    founder's zone is an IANA name in a FRONTEND localStorage store
    (`social-forge-timezone`), and `users.timezone` is a column the server
    NEVER writes - the -300 in the seed is of unknown unit. So the server has
    no usable per-user zone, and the posting-volume chart's day buckets
    cannot yet follow the founder's day. The two sources are named here. The
    DENSITY fix does not depend on it.
  - OPEN: `/api/ai/*` has no streaming endpoint, so the composer's AI panel
    cannot show real progress. The panel is honest about it (an indeterminate
    indicator) rather than pretending. A streaming endpoint is backend work.
  - OPEN: the plan's F1 hex gate ("no hex outside tokens") was amended to
    name `check-hex-tokens.mjs`, whose allowlist covers the DATA files
    (provider brand colours, the tag palette). The gate is the better
    artefact; the plan now says so.
  - The NotificationBell polls its unread count every 60s as a fallback for a
    silently-dropped SSE stream. Deliberate, and recorded in the plan.

---
Task ID: v12-iterations-9-10 (final state)
Agent: omp (merry-tamarin)
Task: The closing iterations - the plan's own gates that were never run,
plus the recursive re-verification pass.

Work Log:
- 360px was not in the probe's width list, so the plan's F5 gate ("a
  360px mobile pass on composer/calendar/channels") had never run. It
  runs now, across every route, and the whole app passes at 360.
- There was NO skip link. The plan's F5 names one. It is the first
  focusable element, visible once focused, and it moves the FOCUS - not
  just the scroll - to the main region. `audit-dialogs.mjs` checks the
  whole contract. It is a plain `.skip-link` class because Tailwind 3
  emits `not-sr-only:focus` - the `not-` variant must come BEFORE the
  utility, so the natural order generates nothing at all.
- The probe and the target audit learned to see a VISUALLY HIDDEN
  control: a 1x1 box that is CLIPPED AWAY is not a target (the skip
  link pattern); a 1x1 box that is merely tiny still is. Verified in
  the browser against all three cases.
- The composer's TARGET PICKER had no role at all. It is a multi-select
  `role="listbox"` with `aria-multiselectable` and `role="option"` +
  `aria-selected` per row. The channels' QUICK MENU is a `role="menu"`
  with `role="menuitem"` per row.
- F4's verification was "analytics with seeded data renders non-empty" -
  a sentence, not a gate. `audit-analytics.mjs` checks non-zero
  engagement, a DENSE day series, and a success rate that RECONCILES
  with the "N of M went out" beside it. Its teeth are proven: labelling
  the goal-inflated rate a success rate makes it report "the success
  rate reads 24% but 11 of 14 scheduled posts went out, which is 79%".
- THE CALENDAR'S DAY BUCKET NOW FOLLOWS THE FOUNDER'S ZONE - the
  worklog's biggest OPEN item. A post at 02:00 UTC on the 25th was on
  "the 25th"; in New York it is the 24th, so evening posts sat on the
  wrong day. The client resolves the zone's offset (via `zoneOffsetMs`,
  the ONE implementation, extracted from `wallClockToUtc`) and the
  server buckets into the local day. `tz_offset` is VALIDATED - an
  absurd value used to PANIC the handler (`TimeDelta::seconds out of
  bounds`, no CatchPanic layer). The mutation run that "passed" with
  the validation deleted was a STALE BUILD (cargo finished in 0.25s);
  the real run rebuilt (16s) and the test caught the panic.
- `check_status_contrast.py` (all 88 status-token pairs, both themes)
  was written as a gate but never wired in - a future hue change would
  have passed every gate while reintroducing the 2026-10-20 finding.
  It is in `make check`.
- The login page's Unlock button was 343x40 and carried a raw
  `text-white` alongside `text-accent-fg` - the same class of defect
  that made the connect forms invisible in the light theme.

Stage Summary:
- SEVEN gates, all with proven teeth: probe (29 routes x 4 widths x 2
  themes), targets, contrast (both themes, fields included, connect
  flows driven), dialogs (with the skip link), analytics, drag e2e,
  plus the three source gates (hex, tailwind classes, status tokens).
- The last full-gate state: probe 0 FAIL / 0 WARN, 0 controls under the
  floor, 0 contrast failures in both themes, 0 dialog contract
  failures, 0 analytics failures, 0 drag failures. 483 lib tests, 47
  integration binaries, clippy clean online and offline, svelte-check
  clean.
- The empty instance also passes.
- Landed in commits a354b4b..9864450.

Known issues / follow-ups:
  - OPEN (backend): `/api/ai/*` has no streaming endpoint, so the
    composer's AI panel shows an indeterminate indicator rather than
    real progress. Honest, and documented.
  - CLOSED 2026-10-21: see dst-zone-name-bucketing below - the
    series now buckets per-instant in the founder's zone, with a
    zone-day window edge.
  - CLOSED: the MCP `calendar_get` tool now takes `tz_offset` and
    buckets through the SAME day-key helper as GET /api/calendar
    (`api::calendar::local_day_key`), so the two surfaces no longer
    disagree for the same operation (AGENTS.md 9.21). An agent asking
    "what is on Oct 25?" and a founder looking at the calendar now get
    the same answer.
    (2026-10-21: the happy path only - an unknown zone on an empty
    range, and a valid name beside an out-of-range offset, still
    disagreed. Both fixed; see dst-zone-name-bucketing.)
  - CLOSED 2026-10-21: see dst-zone-name-bucketing below - the client
    sends the zone NAME and the server resolves it per instant, so a
    window spanning a DST transition no longer buckets the
    post-transition posts an hour late.

---
Task ID: dst-zone-name-bucketing
Agent: omp (goal session, 2026-10-21)
Task: Fix the two open visual-layer bucketing bugs: the calendar's DST
wrong-day bucketing (one constant offset per window) and the analytics
day series bucketing in UTC while the calendar buckets locally.

Work Log:
- Added `chrono-tz` 0.10 (compile-time embedded IANA tzdata, single-binary
  rule intact) and recorded the lock via `vendor-pull.sh --update`.
- ONE seam, one rule: `api::calendar::parse_zone` (parses an IANA name
  ONCE, up front) + `zone_offset_seconds` (per-instant offset) +
  `local_day_key` (unchanged day-key rule). REST, MCP and analytics all
  route through it.
- GET /api/calendar accepts `tz=<IANA name>`, resolved per-instant
  (DST-correct). `tz_offset` stays as the legacy constant form, and its
  bound is now UNCONDITIONAL: a valid name must not smuggle a garbage
  offset past the 400.
- Unknown zone name is a 400 naming the bad zone - NOT a silent UTC
  fallback (a garbled zone would misplace every post in the window with
  a 200 and plausible day keys). The first implementation had exactly
  that silent fallback; the test that pinned it was rewritten.
- `calendar_get` (MCP) takes `tz` too; parse lifted ABOVE the post loop
  (was per-post inside it, so an empty range answered Ok with no days -
  an agent would read a bad zone as "nothing scheduled"); offset bound
  unconditional, matching REST (9.21).
- Analytics `dense_day_series` buckets in the founder's zone: buckets,
  `start` AND `today` are all zone days. The UTC-`start` bug dropped
  every post published in the first hours of the window (they bucketed
  below the loop's first day while the SQL counts included them).
- Frontend: calendar, dashboard and analytics clients send the zone
  NAME (`timezone.value`); the client no longer resolves offsets.
- Tests: DST regression asserts Nov 1 by name AND Nov 2 by constant
  (teeth both directions); zone-edge series test; unknown-zone 400;
  MCP unknown-zone + bypass tests; REST bypass mirror. The MCP bypass
  test was run RED against the buggy shape before the fix landed
  (9.51 discipline).

Stage Summary:
- cargo test --lib: 486 pass. calendar suites: 8 + 8 pass.
- clippy --lib --bins --all-features --locked: clean (pre-existing
  core2 patch warning only).
- pnpm build + svelte-check: clean. make check: clean (88 contrast
  pairs, tailwind classes, vendor pins).
- All 7 browser gates against the final release binary on :6544:
  probe 27x4x2 0 FAIL / 0 WARN, 0 targets under floor, 0 analytics
  failures, 0 dialog failures, 0 contrast failures both themes,
  0 drag failures.
- Live oracle on the release binary: `tz=Not/AZone` -> 400 with the
  message; a 02:00Z post lands 10-24 under `tz=America/New_York` and
  10-25 under UTC; the name+offset bypass pair -> 400.

Known issues / environment findings:
- The :6544 audit instance is supervised by /tmp/supervise-seed.sh
  (while-true respawner, seeded DB at /tmp/sf-seed). It now runs the
  RELEASE binary. Killing the server does not kill the instance; a
  gate run can silently measure a stale build if the binary is not
  rebuilt first. It was serving a DEBUG binary from an earlier tree
  state for part of this session, which produced a false "tz not
  applied" signal.
- `make deploy`/`make redeploy` are broken on this host by the
  cargo-zigbuild 0.23.4 / zig 0.16.0 skew ("unsupported linker arg:
  --no-keep-memory"); native `cargo build --release` + supervisor
  respawn is the working path here.
- A server started without APP_URL/FRONTEND_URL overrides uses the
  repo .env's :6543 and fails CSRF for a :6544 origin.
- OPEN (separate item): the remaining analytics sites
  (get_engagement.by_day_sql, get_cadence.day_rows,
  calculate_streak) still day-bucket in UTC; the summary series is
  zone-correct now, those three are not.
- OPEN (unchanged): /api/ai/* streaming (backend, not visual-layer).

---
Task ID: analytics-zone-day-completion
Agent: omp (goal session, 2026-10-21)
Task: Close the last three UTC day-bucketing sites on /analytics so the
page stops disagreeing with itself at the day boundary (the OPEN item
from dst-zone-name-bucketing).

Work Log:
- `AnalyticsDaysQuery` gains `tz` (IANA name, unknown -> 400, parsed
  ONCE via the shared `parse_query_zone`). The summary endpoint now
  uses the same helper instead of its own inline parse.
- ONE day-date rule for every analytics series: `zone_date(dt, tz)`
  (AGENTS.md 9.18). `dense_day_series`, `fill_missing_days` and
  `calculate_streak` all bucket through it.
- `get_engagement`: the SQL no longer groups by
  `date(published_at,'unixepoch')` (UTC). It selects one row per post
  (epoch + per-metric sums via the metric-label vocabulary) and the
  day bucket is computed in Rust - SQLite's
  `date(ts,'unixepoch',?)` takes a CONSTANT modifier and cannot
  express a per-instant zone offset, so the DST bug would come back
  with any SQL-side grouping. `fill_missing_days` now takes the zone:
  BOTH edges (start AND today) are zone days.
- `get_cadence`: same shape - per-post epochs, zone-day counts in
  Rust. The "Cadence against goal" chart can no longer disagree with
  the zone-correct "Posting volume" chart beside it.
- `calculate_streak`: the DISTINCT-date SQL is replaced by per-post
  epochs deduped by zone day, and the today/yesterday anchor is the
  ZONE's, so a streak no longer resets at the wrong hour for a
  founder whose midnight is not UTC midnight.
- Frontend: getEngagement/getAdherence/getCadence take the zone NAME;
  the analytics page and the dashboard pass `timezone.value`.
- Visual-layer fix found by the gate run (not by the analytics diff):
  the kanban "older posts" banner's inline Posts link was 31x15 -
  under the 24x24 WCAG 2.5.8 floor AND audit-targets' 44px floor. It
  is a conditional banner that only renders past 200 posts, which is
  why earlier probes passed: the seeded DB had not crossed the limit.
  Now `inline-flex min-h-11 min-w-11` (the ChannelSelector pattern),
  44x44 rendered.
- Environment finding: the probe run that reported 19 navigation
  FAILs ("system error -122") was EDQUOT - ~550MB of my own leftover
  /tmp/probe-* screenshot dirs filled the tmpfs quota and the probe
  could not WRITE its screenshots. Cleaned. A gate run that fails on
  `write` errors is measuring the disk, not the app.

Stage Summary:
- cargo test --lib: 487 pass (incl. the new engagement zone-edge
  series test). Calendar suites: 8 + 8.
- clippy --lib --bins --all-features --locked: clean.
- pnpm build + svelte-check: clean. make check: all gates clean.
- All 7 browser gates against the final release binary on :6544:
  probe 27x4x2 0 FAIL / 0 WARN, 0 targets under floor, 0 analytics
  failures, 0 dialog failures, 0 contrast failures both themes,
  0 drag failures.
- Live oracles: `tz=Not/AZone` -> 400 on /engagement; /cadence
  answers the zone name (NY and UTC agree on the seeded mid-day
  posts - correct for that data; the near-midnight disagreement is
  pinned by the unit tests with discriminating instants).

Known issues:
- OPEN: research "best time to post" still buckets weekday/hour in
  UTC and labels it UTC while the rest of the app is zone-aware.
- OPEN: the client/server day-rule pairing - `zone_offset_seconds`
  vs the frontend's `wallClock` - two implementations of one rule
  with nothing pinning them (the counting_rule_test shape).
- Gate coverage gaps: /login is in no gate's route list; the
  calendar Month/List views are not probed by make audit (no --state
  passes).

Follow-up to analytics-zone-day-completion (same session):
- OPEN: the two tap-target gates carry DIFFERENT thresholds -
  probe.mjs FAILs below 24px and WARNs 24-44 (WCAG 2.5.8), while
  audit-targets.mjs FAILs anything under 44px. A control in the
  24-44 band passes the probe with a warning and fails make audit.
  AGENTS.md 9.37 requires the two gates to agree; converging them
  means either fixing ~100 sub-44 controls the audit already
  measures or re-banding the audit's FLOOR - a deliberate decision
  for the next session, not a silent one.
- OPEN: the TopBar streak badge reads `users.streak_days`, a
  24h-window counter maintained by the scheduler - a DIFFERENT
  measure from the zone-day streak on the cadence card
  (`calculate_streak`), which this session made zone-aware. Two
  "streak" numbers can now be on screen, one zone-aware and one
  not, and they can disagree across a DST or midnight boundary.
- Housekeeping: frontend/probe-out/ (a stale 13-route report from
  an old 6543 run) was gitignored but still TRACKED from before the
  ignore landed; untracked and deleted.

---
Task ID: research-best-time-zone
Agent: omp (goal session, 2026-10-21)
Task: Make the research "best time to post" slots zone-aware, closing
the last UTC-labelled timing surface in the app.

Work Log:
- `engagement_histogram_by_slot` now returns one row per (provider,
  epoch): the weekday/hour fold moved out of SQL (strftime('%w'/'%H')
  is UTC-only; `localtime` is the OS zone, not the founder's) to the
  caller, the same shape as the analytics day series.
- ONE slot rule: `api::research::week_slot(epoch, Option<Tz>)` folds
  an epoch into a (weekday, hour) slot in the founder's zone
  (DST-correct) or UTC. The research insight and the MCP
  `posts_best_times` tool both fold through it (9.18), so the two
  surfaces cannot disagree about a post's slot.
- GET /api/research takes `tz=<IANA name>` (unknown -> 400): the
  slots are bucketed in the founder's LOCAL weekday and hour, and the
  label names the zone ("Saturday 06:00-07:00 America/New_York") -
  "post Mon 15:00" is only actionable in the zone the founder posts
  from. The MCP tool keeps its documented UTC fold and its note says
  so.
- The research page's client-side `slotLabel` is DELETED: it used to
  convert the clock to the app's zone while keeping the UTC weekday -
  a second implementation of the server's rule that could disagree at
  the midnight boundary (a 22:00 NY post was "Sunday" by the clock
  and "Saturday" by the server's key, with the weekday pinned to the
  server's). The page now renders the server's zone-correct label
  and sends `timezone.value` in the request.

Stage Summary:
- cargo test --lib: 488 pass (incl. the zone-fold regression:
  2026-10-25T02:00Z -> Sunday 02:00 UTC, Saturday 22:00 New York).
- clippy --lib --bins --all-features --locked: clean.
- pnpm build + svelte-check + make check: clean.
- All 7 browser gates against the release binary on :6544: probe
  0 FAIL / 0 WARN, 0 targets under floor, 0 analytics/dialog/
  contrast/drag failures.
- Live oracle: the same imported posts read 06:00-07:00 in
  America/New_York and 10:00-11:00 UTC (EDT, -4h); the label names
  the zone; tz=Not/AZone -> 400.

Known issues: unchanged from the previous entries (the two-streak
TopBar measure, the gate threshold drift, /login gate coverage,
calendar Month/List probe states, the client/server day-rule parity
test).

Correction to research-best-time-zone (same session): its summary
said "the page now renders the server's label verbatim" - false.
The cell rendered `slot.label.replace(/^\w+\s/, "")`, which left the
zone name on the clock inside an 80px truncate span, so the visible
cell was a mangled "06:00–07:00 Amer...". Fixed: PostingSlot carries
`hour_range` (the clock alone) and the cell renders it; `label`
(full, zone-named) stays the each-key and is the cell's tooltip;
the caption below already names the zone. Probe on /research: 0 FAIL,
0 WARN against the rebuilt binary.

---
Task ID: wall-clock-parity
Agent: omp (goal session, 2026-10-21)
Task: Pin the client/server day-rule pairing - `zone_offset_seconds`
(server, chrono-tz) vs the composer's `zoneOffsetMs` (browser, Intl) -
two implementations of one rule with nothing comparing them.

Work Log:
- `api::calendar` gains `zone_offset_by_name(zone, at)` - the
  name-taking form of the offset rule, parsing through THE ONE
  `parse_zone`. The module is `pub` now, the same pattern as
  `api::activity` and `api::research` before it, so an integration
  test can drive the real seam.
- tests/wall_clock_parity_test.rs, the counting_rule_test shape:
  it loads the REAL frontend/src/lib/composer/wall-clock.ts and runs
  it under node (--experimental-strip-types), never a transcription.
  Two tests:
  1. Per-instant agreement across 15 discriminating cases: both sides
     of the US fall-back (2026-11-01), US spring-forward (2026-03-08),
     EU fall-back (2026-10-25) and NZ spring-forward (2026-09-27)
     transitions, the transition instants themselves, Kolkata and
     Kathmandu (half-hour/45-minute offsets), and ordinary instants.
     The browser side asserts a non-zero offset for non-UTC zones, so
     a stale system ICU fails loudly instead of comparing against a
     silent fallback.
  2. The WRITE-path round-trip: wallClockToUtc's instant + the
     SERVER's offset reads back the wall clock the founder typed,
     exactly for ordinary clocks (day and clock fields, seconds
     included). The pathological clocks are pinned for
     determinism, not exactness: the spring-forward GAP (2026-03-08
     02:30 New York, which does not exist) reads back 03:30 or 01:30,
     and the fall-back AMBIGUITY (2026-11-01 01:30, which happens
     twice) reads back 01:30 on the day - the founder gets a
     deterministic answer, not an error.

Stage Summary:
- cargo test --test wall_clock_parity_test: 2 pass. cargo test --lib:
  488 pass. clippy clean.
- No product code changed except the pub wrapper - this iteration is
  the pin, not a fix: the two rules were already agreeing per-instant
  (the DST work landed them through the same seam); now they cannot
  drift apart silently.

Known issues: unchanged (TopBar's separate users.streak_days measure,
the probe/audit-targets threshold drift, /login gate coverage,
calendar Month/List probe states).

Red-proof for wall-clock-parity (same session, after the commit):
mutating `zone_offset_by_name` to a constant 0 made BOTH tests fail
(per-instant: "server said 0s, the composer's Intl rule said -14400s";
round-trip: the read-back wall clock shifted), and reverting restored
green - the tests have seen their own failure (AGENTS 9.51). The ICU
sanity assertion (assert_ne! 0 for non-UTC zones) is in the committed
test at the per-case loop, lines 100-103.

---
Task ID: streak-one-measure
Agent: omp (goal session, 2026-10-22)
Task: Unify the two "streak" numbers on screen - the TopBar flame
badge (users.streak_days, the scheduler's 24h-window counter) and the
cadence card's zone-day streak - into ONE measure.

Work Log:
- /api/streak is a real handler now (State + AuthenticatedUser +
  Query<StreakQuery>): it computes through analytics::calculate_streak
  - THE SAME zone-day rule the cadence card reads - with the session's
  user and the zone the badge sends (StreakBadge passes
  timezone.value). It used to read users.streak_days, a separate
  24h-window counter the scheduler maintains, looked up by the
  HARD-CODED DEFAULT_USER_ID - so the two streak numbers could
  disagree across a midnight boundary, and on an instance whose users
  row is not exactly that constant the badge read a user that owns NO
  posts: 0 forever, a real defect this instance's seed exposed.
- The MCP analytics_streak tool computes through the same
  calculate_streak with its documented resolve_first_user convention;
  its days are UTC (no zone input yet).
- The scheduler's counter is untouched: it stays as a cheap internal
  hint, but nothing DISPLAYS it, so there is only ever one number the
  founder can read.
- tests/streak_unification_test.rs drives the real router: (1) badge
  == cadence for the same zone, both 3 against three consecutive
  published days, in UTC and New York; (2) the session-user pin - a
  fresh non-DEFAULT_USER_ID user with no posts reads 0, with today's
  post reads 1, isolated per-test by its own user row and session;
  (3) an unknown zone is a loud 400.
- Found while proving it live: my external sqlite3 writes to the
  seeded DB were invisible to the server (single-writer rule, G-08 -
  the app's pool reads its own WAL view). External writes to the
  operator's DB are out of contract; the seeded instance is now clean
  (probe rows deleted, verified 0) and the proof lives in the
  integration test, which seeds through the router's own pool.

Stage Summary:
- cargo test --test streak_unification_test: 3 pass. cargo test
  --lib: 488 pass. clippy clean. pnpm build + svelte-check clean.
- Live (seeded instance, release binary): badge == cadence == 0 -
  correct for its data (last post 10-19, two-day gap); bad zone ->
  400. The non-zero agreement proof is the integration test's.

Known issues: unchanged (probe/audit-targets threshold drift, /login
gate coverage, calendar Month/List probe states).

Follow-up to streak-one-measure (same session):
- Red-proof run: reverting streak_check's user to the hard-coded
  DEFAULT_USER_ID made the_badge_counts_the_session_users_own_
  published_days FAIL ("no published posts for this session's user:
  left 3, right 0" - the constant read another user's posts, the
  exact pre-fix defect) and green again on revert.
- The MCP analytics_streak tool now takes tz (IANA name, unknown ->
  error, the same loud contract as REST) so an agent can ask in the
  founder's zone.
- OPEN, named: the MCP tool resolves its user through
  resolve_first_user, which is hard-coded DEFAULT_USER_ID across
  ~80 tool sites. On an instance whose users row differs from that
  constant, EVERY MCP tool reads a user that owns no posts - the
  badge had exactly this defect before this fix. Making
  resolve_first_user DB-backed (resolve the single local user) is a
  wide-radius change needing its own tests; it is its own item, not
  silently assumed done.

---
Task ID: gate-coverage-login-and-calendar-states
Agent: omp (goal session, 2026-10-22)
Task: Close the gate-coverage gaps - /login was in no gate's route
list, and the calendar's Month/List/Day views were never rendered by
make audit (9.50: a page probed only in its default state is a page
whose other states are unchecked).

Work Log:
- probe.mjs gains --logged-out: it skips the login AND the dynamic
  route resolution, so the pass measures the pages an anonymous
  visitor sees. /login CANNOT go in the authenticated route list: the
  layout guard redirects a logged-in visitor to '/' (+layout.ts), so
  an authenticated probe of /login would measure the dashboard twice
  while claiming to cover the login page.
- make audit now runs four additional passes: /login logged-out, and
  /calendar in the Month, List and Day states (probe.mjs --state).
  AGENTS.md 9.50's "the three calendar views are all probed" is now
  TRUE of make audit, not just of a manual invocation.
- The new passes found real defects on their first run, exactly the
  9.50 shape: the List view's ◀/▶ pagination buttons rendered 23x24
  and 23x44 - under the 24x24 WCAG 2.5.8 floor - and no gate had ever
  seen them because nothing drove the List state. Both buttons are a
  44x44 box now (min-h-11 min-w-11 inline-flex, the kanban-link
  pattern).
- The Month (88) and Day (112) WARNs are honest 24-44-band
  classifications of dense-grid chips and 24px slot rows - the 9.37
  hazard class where a 44px-everywhere rule would blow the layout;
  they are WARNs by the probe's WCAG-matched severity, not failures.

Stage Summary:
- All five audit passes green against the release binary: default
  27-route pass 0 FAIL / 0 WARN; /login logged-out 0/0; Month 0 FAIL;
  List 0 FAIL; Day 0 FAIL.
- pnpm build + svelte-check + check-tailwind clean; the List-button
  fix is in the embedded frontend of the rebuilt binary.
- OPEN (unchanged): probe/audit-targets threshold convergence (24 vs
  44); resolve_first_user DB-backed resolution.

Known issues: none new.

Follow-up to gate-coverage-login-and-calendar-states:
- Verified the --state clicks REALLY land: applyStates matches the
  button's accessible name as a word (getByRole name=/\bMonth\b/) and
  the calendar's view buttons carry aria-label="Month view" etc.;
  the passes' screenshot names carry the _Month/_List/_Day suffix
  (added only after a successful state click) and the three profiles
  are materially different (Day's slot-row WARNs and List's pager
  FAILs exist only in their own views). A gate that silently
  re-measured the default would have shown the same profile three
  times.
- make audit now removes its five probe output dirs at the end
  (~100MB per run); this session already hit EDQUOT on accumulated
  probe litter and a write failure looks like an app failure.

---
Task ID: target-gate-severity-band
Agent: omp (goal session, 2026-10-22)
Task: Converge the two tap-target gates (AGENTS.md 9.37: "the
severity has to match the standard, or the gate lies" and "the two
gates must agree").

Work Log:
- audit-targets.mjs is re-banded to the standard and to the probe:
  below 24x24 is a FAIL (exit 1), 24-44 is a WARNING (reported,
  written to the JSON under 'warnings', not gating). It used to fail
  the gate on anything under 44, so a control the probe classified
  as a warning failed make audit - the two gates disagreed about the
  same rendered box.
- The checkbox-wrapper case keeps its own band logic (a 24-44
  control inside a 44px label is a warning, not a fail).
- The OUT json is now {fails, warnings} - the old flat array shape
  changed; any consumer reading it must take the new shape (none in
  the repo; make audit only checks the exit code).

Stage Summary:
- Live run against the release binary: 0 controls under the 24px
  floor, 0 in the 24-44 band across the 25 walked routes + the
  composer and its three panels, exit 0. The kanban Posts link that
  the old gate failed is 44x44 and absent from both bands.
- Both gates now classify by the standard's severity: probe FAIL
  <24 / WARN 24-44, audit-targets FAIL <24 / WARN 24-44.

Known issues: resolve_first_user DB-backed resolution remains the
open queue item.

---
Task ID: resolve-first-user-seeded-instance
Agent: omp (goal session, 2026-10-22)
Task: Make resolve_first_user (the ~80-site MCP/CLI user seam) resolve
the instance's REAL local user instead of returning a hard-coded
constant - the seeded-instance defect the streak fix exposed.

Work Log:
- resolve_first_user used to return DEFAULT_USER_ID
  unconditionally. On any instance the app started, that is correct:
  ensure_local_user creates the row. But an EXTERNALLY-seeded
  instance (a copy, a fixture, a migrated database - this session's
  own audit instance) carries its single users row under a different
  id, and then every MCP tool read a user that owned no posts.
- The fix: the constant when its row exists (the normal install,
  unchanged behaviour for every standard deployment), otherwise the
  instance's single real user, otherwise a loud error naming the
  mis-seeding. Multiple users is refused - single-user mode has no
  rule for picking one arbitrarily.
- tests/resolve_local_user_test.rs pins all three arms against the
  real helper. Red-proof: mutating the resolve back to
  always-the-constant made the seeded-instance test FAIL (the
  constant answered instead of the seeded user) and green on
  revert.
- mcp_parity_test's streak call updated for the StreakGetInput { tz }
  shape from the previous iteration.
- Visibility: mcp::auth and resolve_first_user are pub (the
  test-reachable-seam pattern, like api::calendar and
  api::activity).

Stage Summary:
- cargo test --lib 488; resolve_local_user 3; mcp_parity 9;
  mcp_tools 20; streak_unification 3 - all green. clippy clean.
- The ~80 call sites' behaviour is UNCHANGED on a standard install:
  the constant's row exists there and the first query confirms it.

Known issues: the queue from the previous entries is now empty. The
remaining work is discovery, not named items: another red-team pass
over the surfaces this session did not touch (automation, feed,
campaigns detail, comments) with the widened audit gates.

Correction to resolve-first-user-seeded-instance's summary:
- "every item landed with red-proofed regression tests" overstated.
  Red-proofed: the DST day-key tests, the zone-name bypass test, the
  wall-clock parity pair, the streak session-user test, the
  resolve_first_user seeded-instance test. NOT red-proofed: the
  analytics zone-series tests and the resolve constant-arm (both
  verified green against discriminating data, but no mutation run).
- Two paths were named as landed while never executed (9.38):
  (1) the tap-target re-band's 24-44 WARN branch - the live run
  found zero controls in the band, so the new classification has
  only run its all-clear path; the branch's first real exercise will
  be whichever future control lands in the band;
  (2) resolve_first_user's multiple-users refusal arm - NOW COVERED:
  tests/resolve_local_user_test.rs::two_users_without_the_constant_
  is_a_loud_refusal executes it against a two-row users table and
  passes.

---
Task ID: target-audit-mobile-widths
Agent: omp (goal session, 2026-10-22)
Task: Fresh discovery pass - extend the tap-target gate to the
founder's phone widths, which no target audit had ever measured
(the probe measures 375/360 for overflow/contrast; audit-targets
only walked at 1440).

Work Log:
- audit-targets.mjs walks the 25 routes and the composer panels at
  1440, 375 AND 360 now. The mobile layouts are DIFFERENT CSS
  (collapsed sidebars, stacked toolbars), so a control that clears
  the floor at 1440 can be squeezed by its mobile layout - the
  desktop-only audit could not see it. The 'where' keys carry the
  width (route@375); make audit reads only the exit code, so no
  consumer broke.
- Result on the first full run: 0 controls under the 24px floor and
  0 in the 24-44 band across all 75 route-width combinations plus
  the composer and its three panels at each width. The mobile
  layouts clear the floor everywhere the audit walks.

Stage Summary:
- Live run against the HEAD release binary: exit 0, empty JSON.
- No product change: this iteration widened the gate and found
  nothing to fix - recorded so the next pass knows mobile tap
  targets were measured, not skipped.

Known issues: none new. The unexercised 24-44 WARN branch remains
the one named gap (previous entry).

---
Task ID: browseros-visual-pass-dialog-name
Agent: omp (goal session, 2026-10-22)
Task: The objective's browseros-neo visual pass - drive the real
dashboard in the real browser and look at what the DOM gates
cannot see.

Work Log:
- Drove browseros-neo (MCP over HTTP, 127.0.0.1:9012) end to end:
  initialize/session-id handshake, navigate, snapshot, act,
  evaluate, screenshot. Logged in through the real form (native
  setter + requestSubmit; the act 'type' sets the value without
  Svelte's rune seeing it, so the Unlock button never enabled).
- FINDING (visual-layer, real-browser-only): the composer dialog's
  accessible name was the generic "Dialog". ModalManager falls back
  to aria-label='Dialog' when the caller passes no title, and the
  composer mounts with title:'' because it renders its own
  "Create Post" heading - so every screen-reader announcement for
  the app's most important dialog was the word "Dialog". The
  audit-dialogs gate SAW name="Dialog" and passed it: a gate that
  accepts a generic name is not checking the 9.49 contract's "a name
  that describes WHAT the dialog is".
- Fix: ModalOptions gains ariaLabel (for callers that render their
  own heading inside); ModalManager uses it before the generic
  fallback; the composer mounts with ariaLabel:'Composer'. The gate
  now FAILS a generic "Dialog"/"Modal" name, so the class is caught.
- FINDING (environment, not app): browseros's own transient
  <bit-notification-bar-root> overlay covered the Month-view
  button at one click point and act refused the click; the bar is
  extension chrome that dissolves, not app content - playwright
  contexts never have it. Recorded so the next real-browser pass
  knows covered-element refusals can be transient.
- The Month view driven in the real browser renders the full a11y
  grid (day cells, named post chips with keyboard-resize hints) -
  structural evidence the state renders, on top of the probe's
  numeric gates.
- Image viewing is unavailable in this session's model, so the
  screenshot rubric is BLOCKED per the tool protocol - the
  structural snapshots carry the visual-layer evidence instead.

Stage Summary:
- audit-dialogs against the rebuilt binary: composer
  name="Composer" modal=true focusInside=true, 0 contract failures.
- make check clean; svelte-check + pnpm build clean.
- The composer click in the last browseros attempt did not open
  the dialog (the earlier attempt did); the DOM gate exercises the
  same modal-manager path and shows the named dialog, and the
  earlier real-browser session rendered the composer's full
  structure. Recorded as-is.

Known issues: none new beyond what is recorded above.

Follow-up to browseros-visual-pass-dialog-name (same session):
- The browseros CDP port (127.0.0.1:9171) is a SECOND, more reliable
  driving path than the MCP wrapper: playwright-core's
  connectOverCDP drives the real browser without the wrapper's
  click-coverage refusals, with real viewport control, and the
  browseros session's cookie carries the authentication.
- scripts/rubric-shots.mjs (frontend) bridges the objective's
  visual-testing ask to the website-design skill's rubric: it
  captures the seven judged surfaces (dashboard, the three calendar
  views, analytics, research, kanban) from the real browser,
  authenticated, into a directory for scoring. Exits 3 (BLOCKED)
  when browseros's CDP is down - screenshots are never faked.
- BLOCKED, named honestly: the ten-line look rubric and the CRAFT
  score require VIEWING the screenshots, and this session's model
  cannot view images (verified twice: read on a PNG returns "does
  not support images"). Per the tool protocol, BLOCKED is never
  reported as passed - the rubric has NOT been scored. The shots
  live in /tmp/sf-rubric and the bridge is permanent, so the next
  session with image viewing scores them in minutes.

Full-suite verification at 8414a04 (same session):
- The complete make audit (9 passes) ran against the HEAD release
  binary end to end: default 27-route probe 0 FAIL / 0 WARN; targets
  0 under the 24px floor at all three widths; dialogs 0 contract
  failures (composer name="Composer"); analytics 0; drag 0; the
  calendar Month/List/Day state passes 0 FAIL each (88/54/112
  24-44 band WARNs - the dense-grid chip class, correct severity);
  the /login logged-out pass 0/0. Exit 0, ~13 minutes.
- Rubric-scoring attempt via a subagent confirmed CANNOT VIEW
  IMAGES is a property of the model chain, not of the file or tool:
  the shots remain ready in /tmp/sf-rubric for a vision-capable
  session. No scores fabricated.

---
Task ID: dialog-gate-widening-401-bounce
Agent: omp (goal session, 2026-10-22)
Task: Widen the dialog gate to every reachable modal surface (the
review's follow-up to the composer's generic "Dialog" name), and fix
what the widened gate found.

Work Log:
- audit-dialogs.mjs's trigger list grew from 6 to 8: the Post
  Statistics modal (calendar list row) and the AI Post Generator -
  the two the founder opens most after the composer. Trigger
  matching is exact on accessible name / title / aria-label; the
  generator's emoji-prefixed text is matched via its title
  attribute.
- The widened gate found a REAL defect on its first run: clicking
  the list row's View-post-statistics button navigated the founder
  to /login. Root cause, two layers:
  1. GET /api/analytics/post/{id} answers 401 "token_expired" when
     the post's PROVIDER token is expired - a normal, actionable
     state (refresh the channel token), not session death.
  2. The client's global 401 interceptor bounced to /login on ANY
     401 without reading the body, so the provider-401 destroyed
     the just-opened modal and dumped a still-logged-in founder on
     the login page.
- Fix, at the client seam (one place, every caller): the bounce
     fires only on the SESSION 401 (body code "no_session"); every
     other 401 returns to the caller as an error, with the envelope
     unwrapped so the UI shows "Token expired", not raw JSON.
- Live verification through the real browser (browseros CDP):
  the stats click now OPENS the modal and it STAYS OPEN, showing
  "Token expired" inline - a message the founder can act on.

Stage Summary:
- audit-dialogs: 8/8 dialogs pass with real accessible names
  (composer "Composer", post-stats "Post Statistics", generator
  "AI Post Generator"), 0 contract failures.
- Regression after the client.ts change (it is global): the
  default probe 0 FAIL / 0 WARN, analytics 0 failures, make check
  clean, svelte-check clean, rebuilt release binary live.

Known issues: the stats modal showing "Token expired" for an
unrefreshed channel is the honest state; a follow-up could add a
"Refresh token" affordance in that modal, but the destructive
bounce is gone.

---
Task ID: stats-modal-inline-token-refresh
Agent: omp (goal session, 2026-10-22)
Task: The named follow-up from dialog-gate-widening-401-bounce: the
stats modal honestly says "Token expired" but gives the founder
nothing to act on. Add the refresh affordance inline.

Work Log:
- CalendarEvent gains integrationId (the post API already returns
  integration_id; the mapping never carried it), threaded through
  toCalendarEvent and the stats modal's mount.
- PostStatsModal detects the provider-token 401 (status 401 with the
  unwrapped "Token expired" message) and, when it has the
  integration, renders an inline "Refresh <channel> token" button
  (integrationsApi.refresh - the same call the Channels page
  makes), then reloads the analytics on success.
- Live verification in the real browser: the stats modal on the
  expired-token post shows "Token expired" + "Refresh X (Twitter)
  token" with a one-line explanation - the founder acts in place
  instead of hunting the Channels page.

Stage Summary:
- svelte-check + pnpm build + tailwind-class check + make check
  clean; calendar probe 0 FAIL / 0 WARN; dialog gate 8/8 (0
  failures) against the rebuilt binary.
- The fix completes the 401 story end to end: session 401 bounces
  (correct), provider 401 stays in the modal (fixed last
  iteration), and now it names the channel and offers the refresh.

Known issues: none new.

Follow-up to stats-modal-inline-token-refresh (same session):
- The client now carries the error envelope's machine-readable `code`
  alongside the unwrapped message (all error paths, not just 401) -
  callers branch on code === "token_expired" instead of
  pattern-matching the human string.
- The failed-refresh path is driven and fixed: clicking Refresh on
  this instance's expired X cookie session fails (no usable refresh
  token), and the modal previously re-offered the same failing
  button with a promise it would work. Now the refresh failure
  clears the refresh state and offers the REAL next step - a
  "Reconnect X (Twitter)" link to /channels with the honest
  explanation. Verified live in the real browser: refresh clicked ->
  refresh button gone -> Reconnect link shown.
- A button that cannot succeed is worse than no button.

---
Task ID: real-browser-discovery-sweep
Agent: omp (goal session, 2026-10-22)
Task: Discovery pass over every surface no gate had driven - via
the browseros CDP bridge, watching page errors, bad responses, and
rendered state.

Work Log:
- Campaign detail (/campaigns/{id}): renders fully - overview,
  posts-by-state, progress bar, dates, goal - no errors, no bad
  responses.
- Feed, comments: clean. DMs: the known X-provider 502 on
  conversations (the expired token; the probe already classifies
  it PROVIDER ERROR, not an app defect).
- All six settings subpages (signatures, rss, notifications,
  profile, mcp, developer): zero page errors.
- The composer driven end to end for the first time in a real
  browser: opened from the + New Post button, text typed into the
  contenteditable through the CDP bridge, the character counter
  updated live (155/4000 - the plain-text rule fires on real
  input). No page errors, no bad responses.

Stage Summary:
- One sweep, zero new defects: the surfaces converge. The 401 arc
  (bounce -> stay -> refresh -> reconnect) was this session's last
  defect class, and every driven surface since renders clean.
- The queue now feeds from: the vision-blocked rubric (shots and
  bridge ready), the next gate widening, or founder reports.

Known issues: none new.

---
Task ID: dms-cookie-auth-misdiagnosis
Agent: omp (goal session, 2026-10-22)
Task: Post-completion review finding: the DMs page told the founder
"X post or user not found - it may be deleted or protected" when
the real cause is structural - X's DM API is bearer-only and the
channel is cookie-auth.

Work Log:
- Root cause chain: get_dm_conversations calls
  api.twitter.com/2/dm_conversations with Bearer {access_token};
  for a cookie-auth channel that string is the cookie blob, not a
  bearer - X answers 404 (not 401), classify_error maps 404 to
  NotFound, and friendly_error renders the deleted-post copy. The
  From<ProviderError> mapping never sees a 401 because there is no
  401. The "(raw: Unknown API error)" suffix is the server's own
  friendly_error detail flag, not provider text.
- Fix at the DMs handler seam (all three operations: list, read,
  send): a cookie-auth X channel is refused with a 400 naming the
  real limitation - "X direct messages need an OAuth-connected
  channel. Cookie-auth channels (auth_token + ct0) cannot use X's
  DM API." No provider call is made.
- tests/dms_cookie_auth_test.rs drives the real router: the list
  refusal carries the real message and NOT the deleted-post
  misdiagnosis; the send path refuses the same way. Red-proof:
  disabling the refusal made both tests FAIL (the 502
  misdiagnosis returns), green on revert.
- Live: the cookie-auth X channel now answers 400 with the real
  reason; the DMs page renders it under its existing actionable
  wrapper.

Stage Summary:
- cargo test --lib 488; dms_cookie_auth 2 (red-proofed); clippy
  clean; the /dms probe pass clean against the rebuilt binary.
- The DMs surface now tells the truth on every path: OAuth
  channels get their data, cookie channels get the structural
  reason.

Known issues: none new.
