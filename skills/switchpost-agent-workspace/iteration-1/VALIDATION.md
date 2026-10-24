# Iteration 1 — validation notes (2026-10-24, deep re-audit)

## Post-hoc validation performed on all 8 runs

1. **Platform-mutation audit** (all 8 command transcripts): ZERO
   platform mutations by any run. The two `mcp-call posts_repeat`
   calls in the old_skill posts-repeat transcript were schema probes
   that error before any write — reproduced against the live DB:
   `Invalid post_id UUID` / `Post not found`, posts row count
   unchanged. All other flagged lines are `--help` invocations or
   prose.
2. **Local-write caveat**: the with_skill analytics run executed
   `import x --count 20`, which populates the LOCAL `external_posts`
   feed cache (a designed read-cache of already-public posts). It
   writes nothing to the platform. Harmless, but strictly it is a
   local write during a "read-only" task; future eval prompts should
   say "no platform mutations; local caches are fine" to remove the
   ambiguity.
3. **Numbers spot-check**: rhino's "9 followers" confirmed live
   (x_get_analytics); skunk's per-post figures (3 views on the Oct 19
   post) match the live timeline reads. Both reddit-403 runs' verdicts
   (www 403 / OAuth-path OK / x unaffected) match the coordinator's
   own live probes from the same day.
4. **Mid-eval binary change (honesty note)**: the soft-delete
   analytics_summary fix was rebuilt into the release binary at 19:51
   UTC, mid-eval. Skunk (old_skill, finished ~19:52) and rhino's early
   commands ran against the PRE-fix binary — skunk's "25 posts
   tracked" was correct for the binary it ran against at that moment;
   the fixed binary reports 0 (matching the then-29 soft-deleted
   rows). Neither answer is wrong; the benchmark straddles a mid-eval
   fix and the pass/fail assertions graded on workflow correctness,
   not on the summary counts, so no grading was affected.
5. **DB row provenance** (corrects a worklog overstatement): the live
   DB held 0 live posts and 29 soft-deleted rows at session end — 27
   created by THIS session's own verified probes (repeat/thread/
   state-vocab/synonym probes, each deleted right after its check) and
   only 2 predating it (Oct 22). The earlier "25 soft-deleted audit
   rows predating this session" phrasing was wrong and is corrected in
   the worklog.
6. **Deliverable completeness**: all 8 runs have answer.md +
   commands.txt + grading.json; all committed on `github/master`
   (verified via `git ls-tree` on the remote ref, 15 files across the
   three eval dirs).
