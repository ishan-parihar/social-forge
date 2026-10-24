# Trigger-optimization run — 2026-10-24

## Status: BLOCKED by environment (no reliable measurement produced)

The skill-creator description-optimization loop (`scripts/run_loop.py`)
was run twice against `trigger-eval-set.json` (20 queries: 13
should-trigger, 7 should-not-trigger, all realistic user phrasings).

## What happened

1. **Run 1** (default model): died silently at the improver step. All
   claude -p calls errored with `400 model "claude-opus-5-5" is not a
   configured combo` — this host routes through an aroute gateway whose
   routable list (glm-5.3, jev, weaver, …) does not include the CLI's
   default model.

2. **Run 2** (`ANTHROPIC_MODEL=glm-5.3`, `--model glm-5.3`): iteration 1
   completed its eval phase (20 queries × claude -p, 4 workers) and the
   bash wrapper hit its 600s cap before the improver finished. Measured:
   **precision 100%, recall 0%** — but a manual diagnosis showed this is
   a HARNESS ARTIFACT, not a description defect: with the same env var,
   `claude -p` intermittently refuses the model
   (`[claude-code:unrecognized_model] {"model":"glm-5.3"}`) and the
   sessions that do run show the skill's command file present in
   `slash_commands` yet the Skill tool never fires — on ANY query. The
   model-routing layer on this host is inconsistent between invocations
   (same command: one run answers, the next errors).

## Why we did NOT ship a "best description"

Both loop runs measured recall=0 for every should-trigger query. If
taken at face value that would demand a description rewrite — but the
manual probe proved the Skill tool never fires even in a fully-working
session on this host, so the measurement cannot distinguish a bad
description from a broken harness. Rewriting the description based on
this data would be optimizing against noise. The description as of
v0.6.3 (with the added trigger classes) stands on the behavioral eval
instead: the with-skill agents used the right routes.

## What is preserved

- `trigger-eval-set.json` — the 20-query set, ready to run on a host
  where `claude -p` model routing works (the skill-creator doc's
  original assumption: Claude Code with a working default model).
- `run_loop.log`, `run_loop2.log` — both raw runs, including the
  iteration-1 precision/recall numbers for the record.
- The run_loop invocation that WOULD work: from the skill-creator dir,
  `python -m scripts.run_loop --eval-set <set> --skill-path <skill>
  --model <routable-model> --max-iterations 5`.

## Follow-up (on a host with working claude -p routing)

Run the loop with the preserved eval set, apply `best_description` to
the SKILL.md frontmatter, and record train/test scores in the worklog.
