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
