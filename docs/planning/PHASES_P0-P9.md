# P0–P9 — single-user manual suite plan

Authoring the phase plan the objective names. The objective ("phases P0 to P9 in
`TODO.md` each meet their acceptance criteria") presumed a phased plan existed; the
register was flat, so the phases are defined here and each row in `TODO.md` maps to one.

Owner of the phase plan: this file. `TODO.md` stays the live-status register (AGENTS rule 1:
one owner per question — a plan and a register are different questions).

## What the objective actually demands

> Make the switchpost web dashboard a single-user manual suite for solo founders covering
> composer, calendar and scheduling, campaign planning, analytics, research, and a read-only
> agent-activity log, matching postiz-app's manual features without its team, billing, admin,
> agent-chat, or video bloat.

> Done when: clippy `-D warnings`, `cargo test`, frontend type-check and build, `kb lint`,
> `docs_lint` pass; `probe.mjs` shows 0 FAIL at 1440, 768 and 375 on the listed pages; and no
> team, workspace, billing, or admin routes exist. Live-credential runs stay deferred.

Two additions are implied by the surface list and not by the gate list: **research** and
**agent-activity** have no routes in the pre-phase tree, so P5 and P6 exist to create them.

## The phases

| id | phase | acceptance criteria |
|---|---|---|
| P0 | **Baseline gates green** | `cargo clippy --all-targets --locked -- -D warnings` exit 0; `cargo test` 0 failed; `svelte-check` 0/0; `pnpm build` exit 0; `kb lint` exit 0; `genome lint` exit 0 |
| P1 | **Billing removed** | no `/api/billing*` route in `src/api/mod.rs`; `src/api/billing.rs` absent; no `stripe`/`Stripe` identifier in `src/config.rs`, `src/cli/run.rs`, `src/social/mod.rs`, AGENTS env tables, README env table |
| P2 | **Manual surface audit** | every one of composer, calendar+scheduling, campaign planning, analytics, media, tags, feed/RSS, search has a route and a page; no route in the set `team`, `workspace`, `billing`, `admin`, `org`, `member`, `invite`, `role` |
| P3 | **Contrast-safe design tokens** | a helper that returns a WCAG-AA text colour for a provider brand hex in **both** themes; verified over the whole palette (no palette entry below 4.5:1 on either surface); probe reports 0 contrast FAIL |
| P4 | **Probe harness correct** | `probe.mjs` drives a real browser, authenticates, dismisses the onboarding overlay, sets the theme through the app's own channel (localStorage), and its overlap detector excludes ancestor/descendant pairs |
| P5 | **Research surface built** | `GET /api/research` returns own-top-posts, competitor ranking, posting-time slots, content gaps and feed titles from stored data only; `/research` page renders five sections with a per-section empty state naming the missing source; `cargo test --test research_aggregate_test` green |
| P6 | **Agent-activity log built** | `GET /api/activity` merges `events_log` + `automation_logs` + `publish_attempts` newest-first under one user; read-only (every write verb 405 with a session, 401 without); `/activity` page with kind/range/provider filters, cursor paging and live updates; `cargo test --test activity_feed_test` green |
| P7 | **Nav and integration** | `/research` and `/activity` reachable from the sidebar under one Insights group with icons; no duplicate or orphaned entries |
| P8 | **Gate run** | P0's six commands re-run on the finished tree and all exit 0 |
| P9 | **Probe clean** | `probe.mjs` reports **FAIL: 0** at 1440, 768 and 375 across the listed pages in both themes |

## Register mapping

| phase | register row(s) | current |
|---|---|---|
| P0 | — | done (this session) |
| P1 | (new row needed: billing removal) | done (this session) |
| P2 | AUD-9, AUD-12, AUD-23 | done |
| P3 | AUD-15 | done |
| P4 | AUD-9 | done |
| P5 | (new row needed: research) | done (this session) |
| P6 | (new row needed: activity log) | done (this session) |
| P7 | (new row needed: nav wiring) | done (this session) |
| P8 | — | done (this session) |
| P9 | AUD-9 | done (this session) |

P1, P5, P6 and P7 need register rows — the register only carries rows that exist, and a
terminal state must name its evidence (AGENTS rule 4).

## Deliberate non-goals

Whatever the objective does not name. Recorded so a later reader does not read them as
oversight:

- **postiz's team/workspace/billing/admin surfaces** — explicitly excluded; P1 and P2 remove
  rather than port them.
- **postiz's agent-chat and video** — explicitly excluded.
- **Live-credential runs** (V1a–V1d, C3, LIVE-X-FLAKE) — deferred by the user; the objective
  says they stay deferred and are not required.
- **A P0–P9 restructure of `TODO.md` itself** — the phases live here, not in the register.
