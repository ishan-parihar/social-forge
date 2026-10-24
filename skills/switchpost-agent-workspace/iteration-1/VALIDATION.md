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

---

## Validation round 2 (2026-10-24, later the same hour)

Seven further surfaces, all green:

1. **Grading reproducibility**: an independent re-grading script
   (fresh logic, same artifacts) reproduces the committed pass/fail
   matrix exactly — 0 mismatches across all 8 runs.
2. **Assertion discrimination**: five adversarial fake answers (the
   date-only end_date, the `until` param, an answer without the
   per-post route, a "reconnect your account" verdict, a recovery
   answer that never names error_message) are all correctly REJECTED
   by the assertions. The grading is not a rubber stamp.
3. **Skill sync + guards**: installed skill identical to the repo
   (5/5 files); doc-CLI guard 3/3; skill_install 2/2.
4. **Code-fix regressions**: all four session fixes re-verified on
   the current binary — analytics_summary reports 0 (soft-deleted
   excluded); `--state errored` returns 0 WITH a live draft present
   (proving it filters rather than falls through); `--json` emits
   real JSON on both the shim ok-path and the error arm.
5. **Lib suite**: 521/521 at HEAD.
6. **Answer-claim spot-checks**: analytics_get_post still rejects
   feed uuids ("Post not found"); doctor still reports reddit
   healthy/live_verified; `posts get` exists as documented.
7. **Tree + remote**: clean tree (`.serena/` junk untracked only);
   remote at 3e21cbe.

No further corrections required — round 2 found nothing the round-1
record had wrong.

---

## Validation round 3 (2026-10-24, same hour)

Nine further surfaces — the artifact-internal consistency that rounds
1 and 2 did not re-derive. All green:

1. **Benchmark internal consistency**: benchmark.json's per-run
   pass_rates match the grading.json files they were aggregated from,
   8/8, 0 mismatches.
2. **Snapshot integrity**: skill-snapshot/ is byte-identical to the
   claimed v0.6.2 commit (5/5 files EXACT) — the old-skill arm really
   graded the skill as it existed then, not as it is now.
3. **Metadata ↔ runs**: all 4 eval_metadata.json files map to the
   correct run dirs with answer.md + commands.txt + grading.json
   present, 0 problems.
4. **benchmark.md ↔ benchmark.json**: the aggregate table's pass
   rates (79%±25% vs 100%±0%, delta -0.21 = old−with), token means
   (2,869,225 vs 1,700,000) and time means (2475s vs 2220s) all
   recompute exactly from the run records.
5. **Pass-rate arithmetic**: every grading.json's pass_rate equals
   passed/total, 8/8.
6. **Regex re-match**: all 18 regex evidence patterns re-executed
   against the answer texts reproduce the committed verdicts, 0
   mismatches.
7. **Answer-claim live reproduction (fresh)**: the causal
   differentiator was re-executed live on the current binary —
   date-only end_date fails with "Invalid end_date (use RFC3339):
   premature end of input" while RFC3339 succeeds ("Post will repeat
   every 7 days until 2026-11-30T23:59:59+00:00"). Probe cleanup
   verified: live rows 0, all 7 probe posts soft-deleted.
8. **Transcript read-only audit (round-3 form)**: all 8 commands.txt
   re-scanned for write-shaped verbs — 6/8 contain none; the 3
   write-shaped lines in the remaining two are all `--help` forms,
   and the one DATABASE_URL-annotated command in eval-failed-recovery/
   old_skill is explicitly marked as never run. Zero mutations.
9. **DB residue accounting**: soft-deleted is now 37 (was 29
   pre-session) — 8 rows created by this session's own regression and
   causal probes, all soft-deleted after use; live count 0.

No corrections required — rounds 1-3 now cover every artifact,
every number, every claim.

---

## Validation round 4 (2026-10-24, same hour)

Nine further surfaces — the answers' live factual claims, the run logs,
and the one defect this round actually found:

1. **Task parity across arms**: both arms of every eval answered the
   same task prompt (confirmed via eval_metadata prompts + answer
   openings; the with/old twins address identical tasks).
2. **Timing plausibility**: all 8 timing.json durations are plausible
   worker wall-clock spans (900-4500s) and match benchmark.json.
3. **Token-rounding disclosure**: benchmark.json explicitly notes
   "Timing/token data approximate from observed swarm stats" — the
   round numbers (2.5M, 3.2M...) are disclosed approximations, not
   fabricated precision.
4. **eval0 live claims re-verified NOW**: followers 9 / following 93 /
   user id 3301263462 (x_get_me live); latest OWN post is still
   "The research tab ranks..." Oct 19 13:59:35 UTC (x_user_tweets
   live — the Oct-24 tweets in the HOME timeline are followed
   accounts', not own posts); the single reply landed on exactly
   "The worst part of social media management..." (reply_count: 1
   live). All claims hold at validation time, not just run time.
5. **eval2 live claims re-verified NOW**: the Reddit 403 "whoa there,
   pardner" still reproduces on the www path; the OAuth paths
   (reddit_get_saved → real Listing + modhash, reddit_list_friends →
   UserList) still succeed — the www-vs-OAuth endpoint distinction
   the answer drew is real and current.
6. **eval3 workflow claims re-verified NOW**: posts list --state error
   (returns 0 on the clean DB), posts update --settings/--content/
   --title/--media flags exist as documented, posts publish help says
   "queued or errored post" as the answer claims.
7. **eval1 claims re-verified NOW**: no native CLI repeat subcommand
   (confirmed), tools_search still returns posts_repeat with the
   documented description, and the old-skill's two probe calls
   ("nonexistent-schema-probe", "not-a-date") still error BEFORE any
   write (Invalid post_id UUID / Post not found; DB unchanged).
   Eval-design note: the old-skill probe's bad post_id meant it could
   never have discovered the RFC3339 requirement by probing — the
   assertion correctly grades the answer artifact.
8. **review.html**: embeds the real committed data (0.667 appears
   once, for eval0 old_skill; per-eval verdicts match grading.json).
9. **DEFECT FOUND AND FIXED**: trigger-optimization/README.md and the
   worklog said the trigger eval set was "12 should-trigger, 8
   should-not" — the set is **13/7**, and both run_loop logs graded
   13/7 (13 expected=True lines, 7 expected=False, in each). The
   a5b13f8 commit message carries the same wrong count (immutable;
   corrected here per protocol). Both files corrected in this commit.

---

## Validation round 5 (2026-10-24, same hour)

Six further surfaces — the grading's coverage, the causal
differential, and two record corrections:

1. **Workspace file count**: 49 files (the round-4 commit message
   said "52 files" — corrected here; the count was taken before
   `.serena/` was excluded).
2. **Causal differential verified at the source**: the current skill
   teaches `end_date` + FULL RFC3339 at 3 sites (SKILL.md:86,
   recipes.md:89-90); the v0.6.2 snapshot has ZERO `end_date`
   mentions — its only repeat guidance was "until an end date" with
   no param name, no format, no example. The old-skill failure is
   the direct product of that gap.
3. **Schema discoverability**: `mcp-tools` lists descriptions only;
   the input schema is discoverable only field-by-field via
   `missing field` errors, and the RFC3339 format requirement is
   reachable ONLY through a call with a real, scheduled post (the
   post-exists check runs BEFORE the date check — a nonexistent-id
   probe can never see it). The skill's example is the only cheap
   route; the eval measures a real capability gap, not a trick.
4. **Fresh faithful regrade**: all 11 committed expectation texts
   re-implemented from scratch and re-run against the 8 answers —
   0 mismatches. The committed evidence strings are exact and honest
   (with_skill: `value: 2026-11-30T23:59:59Z` passed; old_skill:
   `value: 2026-11-30` failed).
5. **COVERAGE DISCLOSURE (new finding)**: eval_metadata declares 15
   assertions but grading.json graded only 11 (per-run: 3+2+3+3).
   Four designed assertions were not in the graded matrix: analytics'
   "read-only transcript" and "numbers not placeholders", and
   posts-repeat's "uses mcp-call" and "no repeat actually created",
   and failed-recovery's "discovery actually run". All four were
   re-run fresh in this round across BOTH arms: all 10 rows PASS for
   both arms. So the headline 11/11 (100%) vs 9/11 (81.8%) is
   accurate AS GRADED, and the full 15-assertion matrix would read
   16/16 (100%) vs 14/16 (87.5%) — same direction, slightly narrower
   gap, because the dropped assertions were safety/quality checks
   both arms passed. The dropped assertions' safety content WAS
   independently covered by the v0.6.7 mutation audit and the
   round-3 read-only audit, so no unsafe run went undetected.
6. **Local-write reconciliation**: the with_skill analytics run's
   `import x --count 20` (feed-cache write) does NOT violate the
   assertion as worded ("create, schedule, publish, or delete") —
   the VALIDATION.md caveat was the looser reading, and both
   statements are consistent.
