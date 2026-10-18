# Planning index

Each file here is a dated or live planning record. `AGENTS.md` §0.6 is the
authority on which document answers which question; this page only lists them.

| File | Status | What it covers |
|---|---|---|
| [`PLAN_PARITY_DEPTH_SINGLEUSER_v25.md`](PLAN_PARITY_DEPTH_SINGLEUSER_v25.md) | live | Tier rationale (§1) and parity phases. Read before parity or feature work. |
| [`TARGETING_AND_WRITER_DISCIPLINE.md`](TARGETING_AND_WRITER_DISCIPLINE.md) | live | Targeting, collisions, and the single-writer rule (G-08). Why one long-lived process owns the database. |
| [`VENDOR_PINS.md`](VENDOR_PINS.md) | live (CI gate) | Pinned transitive dependencies, enforced by `scripts/vendor-pull.sh --check`. |
| [`OUTLINE-2026-10-19-v2.md`](OUTLINE-2026-10-19-v2.md) | live queue per AGENTS §0.6 | P0 gate, token efficiency, eval loop, credential-blocked live passes. |
| [`OUTLINE-2026-10-19-v3.md`](OUTLINE-2026-10-19-v3.md) | unresolved | Second outline for the same question as v2. `kb lint` flags the overlap (W006). Owner to decide which one is live. |
| [`OUTLINE-2026-10-19.md`](OUTLINE-2026-10-19.md) | unresolved | Earlier 2026-10-19 outline. `kb lint` flags it against v2 and v3 (W006). |
| [`OUTLINE-2026-10-17.md`](OUTLINE-2026-10-17.md) | superseded | Replaced by the 2026-10-19 v2 outline. Its Wave A–C items live in `TODO.md`. |
| [`OUTLINE-2026-10-16.md`](OUTLINE-2026-10-16.md) | history | Executed predecessor outline (`d5b9bbd..7873f84`). |
| [`DECISIONS_2026-10-19.md`](DECISIONS_2026-10-19.md) | open decisions | Decision points with recommendations. Nothing in it is applied until the user approves it. |
| [`EXECUTION_2026-10-19.md`](EXECUTION_2026-10-19.md) | record | Cycle 3 execution log: what landed, gate results, decisions for the user, and what did not land. |
| [`AUDIT_2026-10-14.md`](AUDIT_2026-10-14.md) | history | End-to-end audit: P0 bugs, cookie and API matrix, phased plan. |
| [`GAPS_2026-10-15.md`](GAPS_2026-10-15.md) | history | What the 13 audit commits did not close, before the register existed. |

The live work register is `TODO.md` at the repository root.
