# Decisions — 2026-10-19 (open decision points and recommendations)

Companion to `OUTLINE-2026-10-19-v3.md` (§ "Decisions needed") and
`EXECUTION_2026-10-19.md` (what landed). Written so the open decisions survive
a context compaction. Nothing in this file has been applied; each decision
below needs the user's explicit go-ahead before action.

Status key: **OPEN** = waiting on the user; **RECOMMENDED** = recommendation
given, not yet approved.

---

## D-1 — `glass_pumpkin` pin (yanked 1.10.0) — OPEN

**Context.** `glass_pumpkin` is a transitive dependency of `grammers-crypto
0.7.0`, which requires `glass_pumpkin ^1.6`. The locked version 1.10.0 is
yanked on crates.io. The lock still builds with it, but a fresh resolve
(`cargo update`, or regenerating `Cargo.lock`) cannot select it and may fall
back to a version that re-introduces the `core2` dependency that
`vendor/core2` exists for.

Verified against the local cargo index: non-yanked 1.x releases run up to
1.9.1; 1.10.0 is the only yanked 1.x. Upstream 2.0.0 is a major version that
grammers-crypto 0.7 cannot accept, so it is not an option without upgrading
grammers. `VENDOR_PINS.md` records that 1.9.1 dropped the `core2` dependency.

**Options.**
1. Bump the lock to 1.9.1 (non-yanked, satisfies `^1.6`).
2. Keep 1.10.0 with a dated waiver until the next dependency pass.
3. Upgrade grammers (large change, not justified by this alone).

**Recommendation: option 1.** Make it its own commit: change the lock, run
`cargo build` and the full test suite, and leave removal of the `core2` stub
for a separate follow-up commit. Update `VENDOR_PINS.md` §5 and §7 to match.

---

## D-2 — Commit scope — OPEN

**Context.** This session's changes are uncommitted and sit next to
pre-existing WIP. HEAD moved to `fdc297a` ("genome: add scripts/genome CLI"),
which this session did not make. Pre-existing uncommitted files include
`src/cli/mod.rs`, `src/cli/run.rs` (toolset hunks at ~401–410 and the
`McpTools` dispatch), `AGENTS.md`, `README.md`, `scripts/install.sh`. Untracked
files include `scripts/genome/`.

**Options.**
1. One big commit of everything.
2. Explicit-path commits of this session's hunks only, with the pre-existing
   toolset WIP left for its own commit.
3. Leave everything uncommitted.

**Recommendation: option 2.** Stage only this session's hunks (filtered
patches via `git diff -U0` and `git apply --cached`), one commit per item in
`OUTLINE-2026-10-19-v3.md` Wave 0 and Wave A. Commit the toolset WIP
separately after reviewing it against `src/cli/mod.rs`. Keep `scripts/genome/`
out of these commits. Do not commit until the full `cargo test --no-fail-fast`
run is green. Build on top of `fdc297a`.

---

## D-3 — Telegram live-test peer — OPEN (result not yet provided)

**Context.** The user reported the Telegram live test "completed already" but
has not given the output. The scratchpad holds no log from that run. The code
now accepts `TELEGRAM_TEST_PEER=me`, which resolves to the session's own
account through `get_me()` (`src/services/telegram_client.rs`,
`resolve_chat`). The test is a single sequential test; grammers sessions are
not concurrent-safe.

**Options.**
1. Use `me` (Saved Messages). No public username needed. Exercises the new
   code path.
2. Use a real `@username` peer. Does not exercise the `me` change.

**Recommendation: option 1 going forward.** If the completed run used a real
`@username`, run it once more with `TELEGRAM_TEST_PEER=me` to cover the new
path. Before closing register row F-E1, we need the `test result:` line (or
the log path) from the completed run. Without that, F-E1 stays open.

---

## D-4 — Escape on four modals — APPLIED in the working tree (uncommitted, needs review)

Applied as recommended option 1 in cycle 2 (see `EXECUTION_2026-10-19.md`).
Dirty check per modal: feed edit text, kanban campaign name, posts
bulk-reschedule values. Repurpose has no typed text and keeps plain Escape.
The behaviour change still needs your review before it is committed.

**Context.** The a11y pass made Escape close four modals that previously
ignored it: feed repurpose and edit, kanban create-campaign, and posts
bulk-reschedule. Escape now discards typed input in those modals without a
warning. Files: `frontend/src/routes/feed/+page.svelte`,
`frontend/src/routes/kanban/+page.svelte`,
`frontend/src/routes/posts/+page.svelte`. This was a behaviour change nobody
reviewed.

**Options.**
1. Keep Escape closing the modals, and ask for confirmation when the form has
   unsaved input.
2. Revert the four modals to the previous behaviour (Escape does nothing).

**Recommendation: option 1.** Escape is the standard way to dismiss a dialog,
and the confirm prevents accidental loss of typed text. Implementation: track
a dirty flag per form; on Escape, if dirty, show a confirm before closing.

---

## Resume checklist

Once decisions are made, the work order is:

1. D-2: commit this session's hunks in explicit-path commits on top of
   `fdc297a`, after the full test run is green.
2. D-1: bump the `glass_pumpkin` lock to 1.9.1, build and test, commit.
3. D-3: obtain the Telegram run output; update register row F-E1 in `TODO.md`
   with `scripts/kb status`, citing `EXECUTION_2026-10-19.md` as evidence.
4. D-4: implement the dirty-form confirm on the four modals; add a smoke check.
5. Then the rest of `OUTLINE-2026-10-19-v3.md`, starting with Wave A (A1, the
   typed-client wrapper audit).

## Key files and state

- Outline (queue): `docs/planning/OUTLINE-2026-10-19-v3.md`
- What landed: `docs/planning/EXECUTION_2026-10-19.md`
- C3 paired evals (3 of 43): `skills/social-forge-agent/evals/c3-paired-run.json`
- Smoke harness and logs: session scratchpad (`smoke.cjs`, `diag.cjs`,
  `shots-after/`); not in the repo.
- Full test log (run not yet complete at time of writing): session scratchpad
  `full-test.log`.
- Pending: full `cargo test --no-fail-fast` verification; no commits made this
  session.
