# Page index — every active doc, the question it owns

`Status` is what the `Owns` column is conditional on: **LIVE** rows answer their
question today and are safe to route to. **ARCHIVED** rows are history kept for
provenance — their content was superseded, so read them for what happened, never
as the current answer. Where a row was superseded, the row names its successor.

Every doc in this index has a row; a doc with no row does not exist (the
tombstone convention, AGENTS.md §0.6).

| File | Status | Owns |
|---|---|---|
| [`SESSION_HOOKS.md`](SESSION_HOOKS.md) | LIVE | Session Hooks — Ambient Context at Session Start — AXI §7: Register your tool into the agent's session lifecycle so every conversation starts with rele… |
| [`agent-integration.md`](agent-integration.md) | LIVE | Agent integration — Moved out of the README to keep it short. Content is unchanged.… |
| [`GAPS_2026-10-15.md`](archive/GAPS_2026-10-15.md) | ARCHIVED (superseded) | Superseded by the OUTLINE succession; its gaps are closed or tracked in `TODO.md`. History only.… |
| [`OUTLINE-2026-10-16.md`](archive/OUTLINE-2026-10-16.md) | ARCHIVED (executed, superseded) | History only. Execution `986f3a2..c219cc1`; superseded by OUTLINE-2026-10-17.… |
| [`OUTLINE-2026-10-17.md`](archive/OUTLINE-2026-10-17.md) | ARCHIVED (superseded) | History only. Its Wave A–C items are tracked in `TODO.md`.… |
| [`OUTLINE-2026-10-19-v3.md`](archive/OUTLINE-2026-10-19-v3.md) | ARCHIVED (superseded) | History only. Superseded by OUTLINE-2026-10-19-v2, the live queue.… |
| [`OUTLINE-2026-10-19.md`](archive/OUTLINE-2026-10-19.md) | ARCHIVED (superseded) | History only. Superseded by OUTLINE-2026-10-19-v2, the live queue.… |
| [`AUDIT_2026-10-14.md`](audit/AUDIT_2026-10-14.md) | ARCHIVED (executed) | History. The 2026-10-14 end-to-end audit: P0 bugs, the cookie and API matrix, and the phased plan it produced. Executed through the 2026-10-15/16 cycles; the register in `TODO.md` carries what is still open.… |
| [`deployment.md`](deployment.md) | LIVE | Deployment — Moved out of the README to keep it short. Content is unchanged.… |
| [`DECISIONS_2026-10-19.md`](planning/DECISIONS_2026-10-19.md) | LIVE | Decisions — 2026-10-19 (open decision points and recommendations) — Companion to OUTLINE-2026-10-19-v3.md (archived; see docs/archive) (§ "Decisions needed") and EXECUT… |
| [`EXECUTION_2026-10-19.md`](planning/EXECUTION_2026-10-19.md) | LIVE | Execution record — 2026-10-19 (red-team follow-through) — Evidence for the register transitions made this cycle. Written after the red-team pass in this sessi… |
| [`OUTLINE-2026-10-19-v2.md`](planning/OUTLINE-2026-10-19-v2.md) | LIVE (stale verdicts — see the re-audit) | The working queue as of 2026-10-19. Audit-revised that day; dated-frozen and largely superseded since. Verified 2026-10-20: its six "STILL OPEN" B/C items are all false (SKILL.md 255 lines not 580; quick-reference 108 not 614; evals 43 incl. toolset-shaped #42; the eval-loop file exists; the recipes block is gone). Only C2 (reference router) is genuinely open, and its "D-test redness" priority claim is false.… |
| [`REAUDIT_OUTLINE-2026-10-19-v2_2026-10-20.md`](planning/REAUDIT_OUTLINE-2026-10-19-v2_2026-10-20.md) | LIVE | The dated re-audit of the outline above: every claim re-measured against the tree, each with its measurement command. Dated-frozen like what it audits — run the commands at its bottom before believing either date.… |
| [`PHASES_P0-P9.md`](planning/PHASES_P0-P9.md) | LIVE | P0–P9 — single-user manual suite plan — Authoring the phase plan the objective names. The objective ("phases P0 to P9 in TODO.md each meet t… |
| [`PLAN_PARITY_DEPTH_SINGLEUSER_v25.md`](planning/PLAN_PARITY_DEPTH_SINGLEUSER_v25.md) | LIVE | Social Forge — Single-User Depth Parity Plan (v25) — > Date: 2026-09-30 > Supersedes: v22 audit, v24 upgrade guide (background reading, not re-litigated… |
| [`SESSION_2026-10-19.md`](planning/SESSION_2026-10-19.md) | LIVE | Session Self-Reflection — Scope: social-forge dashboard work this session — removing billing, building the activity log and re… |
| [`TARGETING_AND_WRITER_DISCIPLINE.md`](planning/TARGETING_AND_WRITER_DISCIPLINE.md) | LIVE | Targeting, collisions, and the single-writer rule — Covers three failure modes that are invisible from the happy path:… |
| [`VENDOR_PINS.md`](planning/VENDOR_PINS.md) | LIVE | Vendor Pins — social-forge — Phase B0 of [PLANPARITYDEPTHSINGLEUSERv25.md](./PLANPARITYDEPTHSINGLEUSERv25.md) §9.… |
| [`platforms-and-cli.md`](platforms-and-cli.md) | LIVE | Platforms and CLI — Moved out of the README to keep it short. Content is unchanged.… |
