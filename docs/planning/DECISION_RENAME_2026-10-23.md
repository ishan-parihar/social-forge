# Decision: project name — N-1

Status key: **APPROVED** = the user has accepted the recommendation and the execution is in
progress; sections below are updated to match the approved plan rather than the original
recommendation.

Companion analysis to the competitor red-team below. **The user approved `switchpost` on
2026-10-23 and the rename is executing**, so this file is now both the analysis and the
running record. One change was made to the original recommendation on approval: **no
compatibility shim.** See §9.1 for what that costs and why the trade is right at 0 stars.

---

## 0. Situation classified, and the three frameworks doing the work

**The situation.** A composite spanning three families, so no single leaf governs. The atlas's
selection protocol routes it: question 3 (long horizon, sequence constrains the future) →
`causality-and-dynamics/time-and-dynamics`; question 5 (competing in an arena) →
`strategic-action/strategy-and-organization`; and the decision is ultimately about what a name
makes strangers think the thing *is* → `language-and-measure/language-and-sensemaking`. No
cross-domain playbook covers "name a project", so the layer-to-family map governs.

**Frameworks selected (3, each with one line of justification):**

| Framework | Why it is load-bearing here |
|---|---|
| `time-and-dynamics/**Path dependence**` | A rename is a state change with accumulated commitments (stars, links, docs, muscle memory). The question is which locks are worth breaking and when. |
| `strategy-and-organization/**Strategy kernel**` | The name is the visible surface of a positioning choice against Postiz, which has already taken the category's headline position. The kernel forces the diagnosis before the move. |
| `language-and-measure/**Prototype theory**` + **conceptual blending** | Star adoption is a categorization act: a stranger decides what the tool *is* from its name, by similarity to a central example, before reading a line of the README. |

**Reconcile rule applied: path-dependence dominance.** The strategy kernel would pick the
strategically cleanest position and the language card the most evocative semantics, but path
dependence decides *when* the move is cheap, and the answer is "now and only now". Strategy and
language then get to choose *which* move, inside the window path dependence opened.

---

## 1. The diagnosis (strategy kernel, first move)

The strategy kernel demands a written diagnosis before any candidate name is scored. Written in
the vocabulary of competitive position:

**The category has a new incumbent, and it took our headline.** Measured 2026-10-23:

| Actor | Position | Evidence |
|---|---|---|
| `gitroomhq/postiz-app` | **37,594 stars**, repo description now literally "📨 The ultimate **agentic** social media scheduling tool 🤖" | GitHub API |
| Postiz's strategy | Shipped `postiz-agent` CLI + MCP server + a published skill on ClawHub; programmatic landing pages for every agent×platform pair | superframeworks.com case study ($145K MRR) |
| `postmcp.dev` | "MCP-native publishing" for AI agents, public beta | web |

**Consequence.** The three-way interface (CLI + REST + MCP) was switchpost's
differentiation, and it is now table stakes in the category's marketing. "Agentic social
media" is claimed, owned, and funded. A name that competes inside that phrase loses, because
the incumbent's version of the phrase is already the prototype a stranger compares against.

**The winnable diagnosis is focus, and it is the position we already actually hold.** Two
structural facts the incumbent cannot copy quickly:

1. **Zero coupling between name and API surface.** Measured: no MCP tool name carries the
   project prefix. All 389 tools are provider-scoped (`x_*`, `posts_*`, `di_*`, `li_*`). A
   rename therefore cannot break a single agent integration. This is rare and it is an asset.
2. **The local-first posture.** Single ~15MB Rust binary, SQLite, no Docker requirement, no
   Redis, no Postgres. Postiz is Node/Next.js with Postgres and more moving parts. The name
   should point at that, not at "agentic scheduling".

**Guiding policy (one sentence, with the exclusion the card requires):**

> We will be the tool a developer reaches for when they want their *own machine* to run their
> social infrastructure as a service, and we will not compete inside the phrase "agentic
> social media scheduling" that Postiz has already bought.

**Coherent actions that policy generates** (each named here, each scored against candidates
below): the name must sound like infrastructure a developer owns, must not contain "post",
"social", "agency", or "agent", must survive the CLI `$ <name> --help` read-aloud test, and
must be short enough to be a crate/package name.

---

## 2. Red-team: the namespace is already colonized

The user's four candidates were tested against three measurements: exact-name collisions,
category prototype, and read-aloud legibility. All four fail, for different and instructive
reasons.

### 2.1 The `post*` prefix is a warzone

A 30-second audit of what "post" now means in developer infrastructure:

| Name | Stars | What it is |
|---|---|---|
| `dispatch` | 6,485★ | incident-response dispatch |
| `conduit` | 2,357★ | Matrix chat server |
| `outpost` | 1,069★ | outbound proxy/firewall |
| PostHog, Postman, Postmark, PostMCP, Postiz | — | product analytics, API client, email, MCP posting, scheduling |

**This kills `pogency`.** It reverses the problem rather than escaping it: it *adds* a "post"
prefix to an invented "gency" stem. It reads as a typo of "potency", has no exact collision
(pure luck), and commits the name to the exact semantic ground where a 37.6k-star incumbent
sits with a funded SEO programme. Prototype-theory verdict: a stranger sorts it as "another
post scheduler", which is the category we just diagnosed as lost.

### 2.2 The `social*` namespace is saturated by SaaS incumbents

SocialPilot, SocialBee, SocialOomph, Socioboard, SociaHive, Sociota: "social-" reads as a B2B
SaaS dashboard with a pricing page, and it directly contradicts the single-binary
local-first positioning. **This kills `sophp`/`sofo`** on grounds beyond aesthetics: they are
compressions of "social forge", so they inherit the dead semantic *and* import the "agency
stem" problem below. `sofo` is additionally near-identical to "Sofort", the dominant European
payments brand; its GitHub fuzzy match count is 1,073, the highest of any candidate tested.

### 2.3 The `-ency`/`agency` stem is fashion, not signal

"Agency" is the single most saturated suffix in the current wave of AI-tool naming, and it
carries a semantic liability the brief itself should reject: this product is explicitly
**single-user and anti-team** (AGENTS.md §0.5.1, a non-negotiable design preference). "Agency"
promises the organizational layer the architecture refuses to build. **This kills `sogency`
and `sociagency`.** A stranger reading either expects multi-tenant team roles and is
disappointed by a one-password binary.

### 2.4 `frogso` fails the only test that cannot be argued with

It is "social forge" reversed and scrambled. It is pronounceable but meaningless, and meaning
is the whole asset. There are 50 GitHub fuzzy matches (`frogsoft-cms`, `frogsol2`), so it is
not even clean. Prototype theory is decisive: a name with no central example behind it gets
sorted into the nearest available category, and "frog" adjacency in a social-posting tool
invites a reading the founder would not want in a README screenshot.

### 2.5 And the current name is not clean either

`social-forge` has **6 exact-name GitHub repositories**, one of which (`Social-Forge/social-forge`)
is an org handle. Staying is not a no-op option, and it forfeits the window.

---

## 3. What the replacement must satisfy (prototype theory, applied)

Rosch's finding, applied: a stranger categorizes by similarity to a central example, and
**basic-level categories** (short names, high feature overlap) are the ones people default to.
The task is therefore not to find an evocative word. It is to find a word whose *central
example already exists in the developer's head* and is the right one.

The central examples available to a developer, sorted by whether they point at what this
product actually is:

| Central example | Points at | Verdict |
|---|---|---|
| A forge / anvil | building things, heat, transformation | **wrong.** We do not build social posts; we dispatch and route them. |
| An agency / a bureau | teams, workflow, humans-in-the-loop | **wrong.** Explicitly out of scope. |
| A mailbox / a carrier / a waystation | **delivery of a message to somewhere, reliably, from your own ground** | **right.** This is literally the architecture. |
| A switchboard / a switch / a post | **routing one input to many destinations** | **right, conditionally.** "Switch" is clean; "post" is contested. |

**Conceptual blending, applied properly.** The blend that has emergent structure neither input
holds alone is *[a switchboard] × [a waystation you own]*. A switchboard routes one input to
many destinations; a waystation sits on your own land; blended, they produce something new:
**a piece of infrastructure that takes one thing you have and puts it in many places, running
on hardware you control.** That is the product. The test the card sets is whether a new move
appeared that neither input had, and one did: "switch" implies reversibility and selection,
which is what a multi-platform dispatch engine does at the moment it decides which channel
gets which content.

---

## 4. The candidates, scored

Score bands, not fake precision (atlas convention): **strong / adequate / weak / fatal**.

| | **`switchpost`** | `forgeway` | `pogency` | `sogency` | `sociagency` | `sofo`/`sophp` | `frogso` | `switchpost` |
|---|---|---|---|---|---|---|---|---|
| Available: GitHub exact | **strong** (1 repo, 1★) | **strong** (3, top 1★) | strong (0) | strong (0) | strong (0) | adequate (37 fuzzy) | weak (50 fuzzy) | **fatal** (6 exact, 1 is an org) |
| Available: PyPI | **strong** (free) | **strong** (free) | strong | strong | strong | strong | strong | strong (free) |
| Available: `the_name` domains (com/dev/io) | **strong** (all free) | **strong** (all free) | strong | strong | strong | adequate | strong | n/a |
| Reads as developer-owned infrastructure | **strong** | **strong** | weak | weak | weak | weak | weak | adequate |
| Contains contested `post*` prefix | no | no | **fatal** | no | no | no | no | no |
| Contains contested `-ency`/`agency` | no | no | no | **fatal** | **fatal** | **fatal** | no | no |
| Promises team features we refuse to build | no | no | no | **fatal** | **fatal** | no | no | no |
| CLI read-aloud: `$ <name> --help` | **strong** | **strong** | weak | weak | weak | weak | weak | adequate |
| Short enough for crate + package + binary | **strong** (9 chars) | **strong** (7 chars) | strong | adequate (9) | adequate (10) | **strong** (4) | **strong** (6) | adequate (12) |
| Survives a hiring/interview sentence ("I built ___") | **strong** | adequate | weak | weak | weak | weak | **fatal** | adequate |
| Differentiation vs Postiz's claimed phrase | **strong** | **strong** | **fatal** | weak | weak | weak | adequate | adequate |
| **Overall** | **RECOMMENDED** | **ALTERNATIVE** | reject | reject | reject | reject | reject | **not neutral** |

crates.io could not be verified: every query to `crates.io/api/v1` returned **HTTP 403** on
2026-10-23 (rate-limited / bot-blocked from this environment). This is an UNKNOWN, recorded as
such rather than assumed. Any chosen name must have its crate availability confirmed before
execution, and `herald`, `patchbay`, `towncrier`, `dispatch`, `conduit`, `outpost` and
`signalbox` were additionally rejected on GitHub-collision grounds (see §6). npm returned 200
for all dictionary-word candidates, which is expected and irrelevant to a Rust binary.

---

## 5. Recommendation

**Primary: `switchpost`.**

One sentence: it is the only candidate whose central prototype is exactly what the product
does, whose name carries zero contested prefix, whose exact collision count is the lowest of
every candidate tested, and which reads as infrastructure a developer owns rather than a SaaS
they rent.

Why it beats the alternative and the field:

- **The blend is real, not decorative.** "Switch" carries routing-plus-selection and
  "post" carries delivery; blended, they produce "one input, many destinations, chosen per
  destination", which is the registry, the per-platform semaphores and the circuit breaker in
  one word. No other candidate produced an emergent move.
- **It survives the read-aloud test that kills most tool names.** `$ switchpost --help`,
  `switchpost mcp`, `brew install switchpost`, `docker run switchpost`, and "I built
  switchpost" are all sentences a person says without correcting themselves.
- **It cannot be confused with the incumbent.** Postiz owns "agentic social media
  scheduling". Nothing in `switchpost` sits in that phrase.
- **It is cheap to own.** 1 exact GitHub repo at 1 star, PyPI free, `switchpost.com`,
  `switchpost.dev` and `switchpost.io` all unregistered.
- **The 10-year test.** One binary on one machine routing content to 26 platforms, in 2036,
  still reads correctly. "frogso" does not. "pogency" does not.

**Alternative, if the founder prefers a name without the `post` syllable at all:
`forgeway`.** It keeps the "forge" heritage the current README is built on (the hero spec is
a forge), is 7 characters, has 3 exact GitHub repos with a top star count of 1, and is free on
PyPI and all three domains. It is the better answer for a founder who wants continuity with
the existing brand asset at `assets/readme/hero.svg` and the forge-fire palette. It is weaker
than `switchpost` on central prototype only.

**If the founder wants a name with no collision history whatsoever**, the generation method
that produced `switchpost` should be re-run rather than a new word invented: take the central
example (`switch`/`relay`/`carrier`/`waystation`), blend it with a second input from the
architecture (`bay`, `lane`, `post`, `line`, `yard`), then screen for exact-collision = 0.
`switchbay`, `relaypost` and `carrierpost` are the next three that screen should produce;
`switchpost` was simply the first one found and it was clean.

---

## 6. Rejected on collision grounds (not on taste)

Measured 2026-10-23 against `api.github.com/search/repositories`, exact-name matches:

| Candidate | Exact-name repos | Top stars | Why rejected |
|---|---|---|---|
| `herald` | 53 | 142 | The most interesting semantic candidate (a herald is a messenger you send), but `kOlapsis/herald` at 46★ is *a self-hosted MCP bridge*, and `timzaak/herald` at 12★ is an AI-agent platform. Direct adjacency to our differentiator. |
| `patchbay` | — | — | PyPI taken (HTTP 200); semantically close to `switchpost` and worse on every axis. |
| `towncrier` | 38 | 927 | `twisted/towncrier` at 927★ is a well-known Python release-notes tool; PyPI taken. |
| `dispatch` | 37 | 6,485 | Owned by a 6.4k-star incident-response tool. |
| `conduit` | 54 | 2,357 | Owned by a 2.4k-star Matrix server. |
| `outpost` | 62 | 1,069 | Owned by a 1.1k-star proxy tool; also carries the `post*` prefix. |
| `signalbox` | 70 | 25 | 70 exact-name repos; PyPI taken. |
| `pigeonpost` | 10 | 11 | 10 exact repos, npm taken. |
| `majordomo` | 3 | 407 | `sergejey/majordomo` at 407★; also the *agency/head-of-household* semantic that violates the no-team rule. |
| `ringmaster` | 4 | 69 | `microsoft/ringmaster` at 69★; same agency-semantic problem. |
| `steppe` | 1 | 49 | Clean and short but semantically empty. |

---

## 7. The rename window: what path dependence says, and what it prices

Path dependence enumerates accumulated commitments and sorts them by cost of unwinding. Done
against this tree:

| Lock | Cost to unwind **today** | Growth rate |
|---|---|---|
| GitHub stars / forks / watchers | **0 stars, 0 forks, 0 subscribers** (`api.github.com/repos/ishan-parihar/switchpost`) | +1 per star, permanently |
| External links, citations, listicles | None measurable | **fastest-growing lock; decides everything else** |
| MCP tool-name contract | **Zero breakage.** 0 of 389 tool names carry the project prefix | flat forever; the name never enters it |
| SQL schema / migrations | **0 occurrences of the literal name in `migrations/`** | flat |
| Rust source | 239 occurrences of `switchpost`/`switchpost` across 21 files, all internal identifiers | flat; a `sed` pass, verify by `cargo check` |
| Frontend | 28 occurrences in `frontend/src` | flat; verify by `pnpm build` + `svelte-check` |
| Docs | 119 in `docs/`, 43 in `README.md`, 33 in `AGENTS.md` | **grows with every doc written** |
| Scripts / systemd / Makefile / service | 81 occurrences | flat |
| Installed skill directory | 413 occurrences in `skills/`, and the directory name `skills/switchpost-agent/` | **grows with every skill revision** |
| `.env.example` / config defaults / DB path / config dir | 26 occurrences of the old name; `~/.social-forge/.env` and `data/social-forge.db` are **operator-visible paths that a live install has on disk** | **the one lock that silently breaks a live deployment** — resolved by the direct rename plus the ordered data move in §9.2 |
| Crates.io package name | UNKNOWN (403) | grows only once published |
| npm name | 200 for every dictionary word tested (i.e. taken); irrelevant unless we publish JS | flat |
| Trademark | Unmeasured: USPTO TESS needs a key. Both finalists are unregistered on GitHub/PyPI/domains, which is the observable proxy | **the cheapest moment to file is before launch** |

**The verdict the card produces.** Two locks are zero-cost now and grow forever: adoption
links, and the installed-skill surface. Two locks are permanent costs that exist no matter
when we act: the `~/.switchpost/` config path, and any operator's existing SQLite file. The
permanent ones are **migration costs, not blocking costs** (§9.2): a WAL checkpoint, a move of
the `.db` / `-wal` / `-shm` trio together, and a re-run of `init` convert an irreversible break
into a supported upgrade path. The user chose the direct rename over the shim (§9.1); the data
move is the one step that cannot be skipped, and it is sequenced in §9.2 rather than left to
chance.

**Therefore: the window is open now and closes with the first external link that matters.**
The correct sequencing is a rename *before* the launch push, not after.

---

## 8. The falsifier, and what would revoke this recommendation

Atlas discipline: the falsifier that would prove the selection wrong.

1. **If `switchpost.com` or the GitHub handle is claimed by squatters within the week**,
   `.com` is the weakest of the three domains anyway; the recommendation survives on `.dev`
   plus the GitHub handle, and the name is not invalidated.
2. **If crates.io shows `switchpost` occupied when the 403 clears**, the name is still valid
   as a project name (the binary ships via `brew` / release tarballs / the installer, not
   crates.io), but the package-name asset is lost. Re-run the screen, switch to `forgeway`.
3. **If a >1k-star tool named anything containing "switch" ships in this category in the next
   quarter**, the central prototype becomes contested and the prototype-theory half of the
   selection is wrong. Re-run §3's blend screen from a different first input (`relay`,
   `carrier`, `waystation`).
4. **The structural falsifier for the whole approach:** if Postiz's repositioning is
   *correct*, then "agentic social media scheduling" is the category and `switchpost` is a
   dodge. The evidence cuts against it: 37.6k stars on a Node/Postgres/Docker stack is winning
   the *cloud* game, and the local-first single-binary founder is a different buyer with a
   different prototype. If that separation turns out to be false, the honest move is to adopt
   the contested phrase and compete on depth, not to rename again.

---

## 9. The execution sequence (APPROVED: direct rename, no shim)

Ordered so that nothing irreversible happens before the reversible work is verified. Each step
is a separate commit, per AGENTS.md §4.1. The user approved this sequence on 2026-10-23 with
one deviation from the original recommendation, recorded in §9.1.

1. **Claim the handles first** (reversible, cheap): `switchpost.com` + `.dev` + GitHub org or
   repo rename, and confirm crates.io now that the 403 has cleared. *Nothing in the tree yet.*
2. **Code + config pass** in one commit, and the config paths move **directly, with no legacy
   fallback**: `Cargo.toml` (crate name, `[[bin]]`, description, repository, homepage,
   keywords), the `switchpost` crate identifier across `src/`, `SwitchpostMcpServer`, clap's
   `#[command(name = "switchpost")]`, the tracing target string, `UNIT_NAME`,
   `.env.example`, and every `~/.social-forge/` and `data/social-forge.db` reference in
   `src/config.rs` — `DEFAULT_DATABASE_URL`, `TEST_DATABASE_URL`, the SQLite-guard error text
   and its `postgres://switchpost:...` example, and the operator-facing help strings.
3. **Tests + skill pass:** 32 files under `tests/` reference the command name, and
   `doc_cli_commands_test.rs` *walks* `skills/switchpost-agent/`, so the skill directory
   rename (`switchpost-agent` → `switchpost-agent`) must land in the same commit or the suite
   cannot find its own fixtures.
4. **Verification gate before the docs pass:** `cargo check --lib --bin switchpost` (0
   errors), `cargo test --lib` (all pass), `cd frontend && pnpm build`, `pnpm exec svelte-check
   --threshold error` (0 errors). The MCP tool surface is provably untouched (0 prefix
   occurrences), so no integration test is needed for the API contract.
5. **Docs pass**, and this is where the audience work in §10 lands: README, AGENTS.md,
   `docs/agent-integration.md`, `docs/platforms-and-cli.md`, PAGE-INDEX rows, the skill
   directory rename, and the session hooks in `docs/SESSION_HOOKS.md`.
6. **Ops pass:** `.github/workflows/{ci,release}.yml` (artifact names and the
   `target/${{ matrix.target }}/release/...` binary paths), `Dockerfile`, `Dockerfile.build`,
   `Makefile` targets, `scripts/social-forge-start.sh` → `scripts/switchpost-start.sh`,
   `scripts/social-forge.service` → `scripts/switchpost.service`, `genome.toml`,
   `workspace-lint.yaml`.
7. **On-disk data migration:** move `data/social-forge.db*` to `data/switchpost.db*` and
   `~/.social-forge/` to `~/.switchpost/`, in that order, after a WAL checkpoint. See §9.2 —
   a live server held the writer lock at approval time, so the move is sequenced rather than a
   bare `mv`.
8. **Relaunch, not rebrand:** new README hero (the existing forge spec becomes a
   *waystation/switchboard* spec), the first launch post, and the sponsor block. The rename is
   worth nothing until a stranger sees it.

### 9.1 The approved deviation: no compatibility shim

The original recommendation asked for a legacy-path fallback in `config.rs`. The user declined
it: **the project is very alpha, so everything is renamed directly, config directories
included.** Recorded so the omission is deliberate rather than an oversight.

**What it costs.** An install that has already run keeps a `~/.social-forge/.env` and a
`data/social-forge.db` that the renamed binary will not read. The operator re-runs the
init/config command, re-points `DATABASE_URL`, and reconnects the channels whose OAuth rows
lived in the old file. The schema is unaffected by the rename: no migration defines, alters, or
references a table or column whose name contains the old project name. The old database is not
corrupt, it is a file the new binary is not looking at.

**The one thing that cannot be renamed:** `migrations/0001_init.sql` carries the old name in a
**header comment**, and sqlx checksums every migration file. Editing that comment made every
test boot fail with *"pool: migration 1 was previously applied but has been modified"*, because
the recorded checksum no longer matches. It is not a cosmetic choice — a modified migration is
sqlx's guard against an operator running a changed schema against a database that already took
the old one. The migration file is therefore reverted to its original bytes and **keeps the old
name in the comment**. Correcting the earlier measurement in this section: the name *does*
appear in `migrations/`, but only in a comment, and that comment is now a permanent fossil. The
rename cannot reach it, and trying to is what proved the rule.

**Why the trade is right.** A shim protects accumulated adoption, and accumulated adoption is
0 stars, 0 forks, 0 subscribers. It would have spent permanent complexity in the config loader
to protect a lock that does not exist yet, and the only thing it would have preserved — one
developer's already-connected channels — is cheaper to reconnect than the code is to carry.
Correct call. It becomes the wrong call only after the first real user, which is exactly the
moment to revisit it.

### 9.2 The on-disk state at approval (measured, not assumed)

A rename of this shape can destroy data if the database is written while its path moves, so the
state was measured first:

```
data/social-forge.db              806,912 bytes
data/social-forge.db-wal         2,130,072 bytes   <- uncheckpointed writes
data/social-forge.db-shm            32,768 bytes
data/social-forge.writer.lock      pid 426867       <- a live server holds this
127.0.0.1:6544  LISTEN  pid 426867 ("social-forge")
~/.social-forge/.env               46 bytes         (APP_PASSWORD)
```

So the move is ordered, not a bare `mv`: stop the server (or confirm it stopped), checkpoint the
WAL, move the `.db` / `-wal` / `-shm` trio **together**, then start. Copy first and only move
after the new path opens and migrates cleanly. Moving the `.db` while leaving `-wal` behind
loses the tail of the database, and the tail is where the recent writes are.

### The rename cost, stated plainly

**1,009 occurrences of `social-forge` across 92 tracked files** (`git ls-files | xargs grep -Io
"social-forge" | wc -l`). Counting variants adds 3,688 `social_forge`, 2,469 `SocialForge`,
2,902 `Social Forge`, 58 `Social-Forge`, 17 `SOCIAL_FORGE`, and 379 `social-forge-agent`, but
most of those overlap the first. The first per-directory scan said 268,537 occurrences; that was
`target/` (26 GB of build artifacts) and the agent caches, which are gitignored and untouched —
the tracked-file count is the only one that matters.

Distribution of the 1,009: 175 `src/` (21 files), 117 `docs/` (21), 117 `frontend/` (38), 76
`scripts/` (8), 68 `tests/` (32), 415 `skills/` (7), 35 top-level files (README 37, AGENTS 29,
worklog 16, Cargo 5, Makefile 5, Dockerfile 4, Dockerfile.build 8, .env.example 4, genome 3,
TODO 2, workspace-lint 1), and 1 in `.sqlx/` — which is a redacted email inside a cached query
payload, **not** the project name, so the offline cache needs no regeneration.

The mechanical part is one `sed` pass per variant plus a review of the `~/.switchpost/` path,
the skill directory, and the three hook registrations. The expensive part is not code: it is
that **every external reference has to be re-earned**, which is the argument for doing it at 0
stars.

---

## 10. The audience, as archetypes (and the README restructure blueprint)

Prototype theory, applied to the adoption decision: a stranger does not read the README and
then decide. They see the name in a feed, categorize it by similarity to a central example,
and then spend 15 seconds checking whether their prototype matches. The README's first job is
not to explain the architecture. It is to make the categorization instant and correct.

Four archetypes, in descending order of how much viral weight they carry:

| # | Archetype | What they want in the first 15 seconds | Where they arrive from |
|---|---|---|---|
| 1 | **The agent-native developer** (the star multiplier) | "My Claude/Cursor/Codex can post for me, from my own box, in 2 commands." | Agent skills, ClawHub, HN, Reddit, awesome-lists, MCP directories |
| 2 | **The solo founder / indie operator** | "One person, 26 channels, no $99/mo, no team-seat tax." | Show HN, r/selfhosted, r/SideProject, alternative.to comparison pages |
| 3 | **The self-hoster / infrastructure-minded** | "One binary, one SQLite file, no Docker, systemd unit, telemetry off." | r/selfhosted, awesome-selfhosted, Hacker News |
| 4 | **The contributor / evaluator** | "Is this maintained? Are the tests real? Is the architecture honest?" | The repo itself, star history, commit cadence |

Archetype 1 is the star multiplier and the reason the rename window matters: agent-native
developers are the cohort that stars things, that write the listicles, and that are being
actively farmed by the incumbent's ClawHub skill. They categorize by asking "is this an
*agent* tool or a *dashboard* tool", and `switchpost` answers correctly in one syllable.

### The restructured README (target shape, ordered for 15-second categorization)

The current README leads with an ASCII architecture box and a "What it is" paragraph, which
makes the stranger work before they can categorize. Restructure for the archetype order:

1. **Hook, 1 sentence, category-first.** Name the basic-level category and the single
   differentiator in the same line. Not "a high-performance social media orchestration
   engine"; that is expert-level and the stranger has no central example for it. Target shape:
   *"The social posting infrastructure that runs on your own hardware. One binary, 26
   platforms, and your AI agent can drive it."* The hook must pass the categorization test
   before it passes any other test.
2. **Proof in the first screen.** Replace the ASCII block with the existing
   `assets/readme/hero.svg` (re-specified from forge to waystation/switchboard), then badges,
   then the Quick Start *above the architecture*, in three labeled paths:
   - **Agent path** (archetype 1, first because it is the multiplier): the MCP config block,
     the skill install, and `switchpost mcp`. This should be the *first* code block on the
     page, not the ninth.
   - **Human path** (archetype 2): `brew` / tarball install, `./switchpost init`, the dashboard
     URL.
   - **Ops path** (archetype 3): the systemd unit, the single `.db` file, `docker build` as
     *optional*, and the explicit "no Docker required, no Redis, no Postgres" statement.
3. **The comparison table, repositioned.** It currently sits mid-page and compares against
   Buffer/Hootsuite/n8n/Postiz. Move it up, and re-anchor it: the first row should be
   "Runs on your own machine, no Docker required" and the row that wins is not "26 platforms"
   but *zero coupling between name and API* and *agent-native three ways*. Postiz now answers
   "agentic", so the table must stop claiming that phrase as ours; it should claim
   **local-first** and **depth** instead, which are the two things the incumbent's stack
   cannot concede without rewriting itself.
4. **Architecture, as one picture + one paragraph.** Keep the diagram, keep the numbers that
   are true, and lead the prose with the claim that the name restructure in §7 makes safe:
   "no MCP tool name carries the project prefix, so renaming this project cannot break your
   integration." That is a genuinely unusual property and nobody else advertises it.
5. **Docs/ as a routing index, not a wall.** The README links into `docs/PAGE-INDEX.md`, which
   is already good. Every link in the README should answer the question the archetype came
   with: "how do I connect X" → platforms-and-cli.md; "how does my agent talk to it" →
   agent-integration.md; "how do I deploy it" → deployment.md.
6. **Support and sponsorship, kept and moved up one section.** Stars and donations are the
   stated objective, so the sponsor block belongs directly after the feature list, not at the
   bottom of a page many readers never finish. The GitHub Sponsors badge already exists; add
   the one-line framing that this is a single-maintainer local-first tool.

### Same treatment for the other user-facing docs

`AGENTS.md` (105 occurrences of the old name across 33 in the file itself plus the §0.6 doc
map), `docs/agent-integration.md` (the MCP config block, the session hooks), the skill at
`skills/switchpost-agent/` (413 occurrences and the directory name itself), and the three
installer/hook files. Each gets the rename in the same commit as its content pass, not after,
so no doc is ever left describing a project that no longer exists.

---

## 11. What this decision does NOT do

Deliberately out of scope, recorded so it is not re-litigated:

- **No execute before approval.** Discharged: the user approved `switchpost` on 2026-10-23, so
  the rename is running and this line is kept only as the record of what it used to say.
- **No trademark filing.** USPTO TESS requires a key; the observable proxies are clean for both
  finalists, but a filing decision is legal advice and stays with the user.
- **No rename of the crate unless crates.io confirms availability** once the 403 clears.
- **No team/multi-user angle in the new positioning**, ever. AGENTS.md §0.5.1 makes it a
  non-negotiable preference, and it is one of the four candidates' fatal flaws.
- **No adoption of the phrase "agentic social media scheduling"** as our own headline. The
  strategy kernel's exclusion clause survives the rename.

---

*Written 2026-10-23. Every measurement above names the command that produced it; re-run them
before acting on this file, because two of them (crates.io, USPTO) are UNKNOWNs and one (the
incumbent's star count) moves daily.*
